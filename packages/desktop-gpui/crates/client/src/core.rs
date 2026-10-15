//! The client state machine, free of GPUI: it turns worker events into state and requests.
//! `Client` wraps it in a GPUI entity; tests drive it directly.

use crate::connection::{Connection, ConnectionConfig, WorkerEvent, session_frame};
use crate::model::{
    AgentControls, AgentSummary, ClientEvent, ProviderSummary, WorkspaceSummary, provider_label,
};
use crate::terminal::{self, TerminalFrame, TerminalHandle};
use crate::timeline::{FetchPlan, LiveRow, TimelineState, TimelineSync};
use anyhow::anyhow;
use futures::channel::{mpsc, oneshot};
use serde_json::{Value, json};
use std::collections::{BTreeSet, HashMap};

type Reply<T> = oneshot::Sender<anyhow::Result<T>>;

enum Pending {
    Agents {
        first: bool,
    },
    Workspaces {
        first: bool,
    },
    Providers,
    Models(String),
    Timeline {
        agent_id: String,
        plan: FetchPlan,
        generation: u64,
    },
    TimelineSubscription,
    CreateAgent(Reply<String>),
    CreateTerminal(Reply<TerminalHandle>),
    SubscribeTerminal {
        terminal_id: String,
        reply: Option<(
            Reply<TerminalHandle>,
            mpsc::UnboundedReceiver<TerminalFrame>,
        )>,
    },
    Action(&'static str),
}

#[derive(Default)]
struct AgentExtra {
    modes: Vec<(String, String)>,
    current_mode: Option<String>,
    permissions: Vec<Value>,
}

struct TerminalStream {
    sender: mpsc::UnboundedSender<TerminalFrame>,
    slot: Option<u8>,
}

/// Session events this client subscribes to; with `explicit_event_subscriptions` the daemon
/// sends them only on request.
const SESSION_EVENTS: [&str; 2] = ["agent_permission_request", "agent_permission_resolved"];

/// Client state for one host.
pub struct ClientCore {
    conn: Connection,
    generation: Option<u64>,
    ready: bool,
    server_info: Option<Value>,
    next_request: u64,
    pending: HashMap<String, Pending>,
    agents: Vec<AgentSummary>,
    extras: HashMap<String, AgentExtra>,
    workspaces: Vec<WorkspaceSummary>,
    providers: Vec<ProviderSummary>,
    viewed: Vec<String>,
    timelines: HashMap<String, TimelineSync>,
    timeline_subscription: Option<String>,
    latest_timeline_request: Option<String>,
    terminals: HashMap<String, TerminalStream>,
    slots: HashMap<u8, String>,
    bootstrapped: bool,
    events: Vec<ClientEvent>,
    directory_changed: bool,
    dirty_timelines: BTreeSet<String>,
}

impl ClientCore {
    /// Starts the socket worker. `sink` receives worker events; feed them back to `handle`.
    pub fn new(
        config: ConnectionConfig,
        sink: impl FnMut(WorkerEvent) -> bool + Send + 'static,
    ) -> Self {
        Self {
            conn: Connection::spawn(config, sink),
            generation: None,
            ready: false,
            server_info: None,
            next_request: 0,
            pending: HashMap::new(),
            agents: Vec::new(),
            extras: HashMap::new(),
            workspaces: Vec::new(),
            providers: Vec::new(),
            viewed: Vec::new(),
            timelines: HashMap::new(),
            timeline_subscription: None,
            latest_timeline_request: None,
            terminals: HashMap::new(),
            slots: HashMap::new(),
            bootstrapped: false,
            events: Vec::new(),
            directory_changed: false,
            dirty_timelines: BTreeSet::new(),
        }
    }

    pub fn is_connected(&self) -> bool {
        self.ready
    }

    /// The `server_info` status payload of the current connection.
    pub fn server_info(&self) -> Option<&Value> {
        self.server_info.as_ref()
    }

    pub fn agents(&self) -> &[AgentSummary] {
        &self.agents
    }

    pub fn workspaces(&self) -> &[WorkspaceSummary] {
        &self.workspaces
    }

    pub fn providers(&self) -> &[ProviderSummary] {
        &self.providers
    }

    pub fn timeline(&self, agent_id: &str) -> Option<&TimelineState> {
        self.timelines.get(agent_id).map(|sync| &sync.state)
    }

    /// Events since the last call, with one `AgentsChanged` for any directory change.
    pub fn take_events(&mut self) -> Vec<ClientEvent> {
        if std::mem::take(&mut self.directory_changed) {
            self.events.push(ClientEvent::AgentsChanged);
        }
        std::mem::take(&mut self.events)
    }

    pub fn has_dirty_timelines(&self) -> bool {
        !self.dirty_timelines.is_empty()
    }

    /// Viewed agents whose timeline changed since the last call.
    pub fn take_dirty_timelines(&mut self) -> Vec<String> {
        std::mem::take(&mut self.dirty_timelines)
            .into_iter()
            .collect()
    }

    pub fn agent_controls(&self, agent_id: &str) -> Option<AgentControls> {
        let agent = self.agents.iter().find(|agent| agent.id == agent_id)?;
        let extra = self.extras.get(agent_id);
        let models = self
            .providers
            .iter()
            .find(|provider| provider.id == agent.provider)
            .map(|provider| provider.models.clone())
            .unwrap_or_default();
        Some(AgentControls {
            modes: extra.map(|extra| extra.modes.clone()).unwrap_or_default(),
            current_mode: extra.and_then(|extra| extra.current_mode.clone()),
            models,
            current_model: agent.model.clone(),
            running: agent.status == "running",
        })
    }

    // ---- worker events ----

    pub fn handle(&mut self, event: WorkerEvent) {
        match event {
            WorkerEvent::Opened { generation } => self.opened(generation),
            WorkerEvent::Ready {
                generation,
                server_info,
            } => {
                if self.generation == Some(generation) {
                    self.ready = true;
                    self.server_info = Some(server_info);
                    self.events.push(ClientEvent::Connected);
                }
            }
            WorkerEvent::Closed { generation, reason } => self.closed(generation, reason),
            WorkerEvent::Message(message) => self.message(message),
            WorkerEvent::Binary(bytes) => self.binary(&bytes),
        }
    }

    /// A new socket: send `hello`'s followers. The worker holds them until `server_info`.
    fn opened(&mut self, generation: u64) {
        self.generation = Some(generation);
        self.ready = false;
        self.slots.clear();
        self.timeline_subscription = None;
        self.latest_timeline_request = None;
        for stream in self.terminals.values_mut() {
            stream.slot = None;
        }
        // These cannot fail: the generation was just set.
        let _ = self.request(
            json!({
                "type": "fetch_agents_request",
                "sort": [{"key": "updated_at", "direction": "desc"}],
                "page": {"limit": 200},
                "subscribe": {},
            }),
            Pending::Agents { first: true },
        );
        let _ = self.request(
            json!({
                "type": "fetch_workspaces_request",
                "page": {"limit": 200},
                "subscribe": {},
            }),
            Pending::Workspaces { first: true },
        );
        let _ = self.request(
            json!({"type": "list_available_providers_request"}),
            Pending::Providers,
        );
        let _ = self.request(
            json!({"type": "session.events.set_subscription.request", "events": SESSION_EVENTS}),
            Pending::Action("session.events.set_subscription"),
        );
        if !self.viewed.is_empty() {
            self.update_timeline_subscription();
        }
        for agent_id in self.viewed.clone() {
            if let Some(sync) = self.timelines.get_mut(&agent_id) {
                let plan = sync.resume();
                self.send_plan(&agent_id, plan);
            }
        }
        let terminal_ids: Vec<String> = self.terminals.keys().cloned().collect();
        for terminal_id in terminal_ids {
            self.subscribe_terminal(terminal_id, None);
        }
    }

    fn closed(&mut self, generation: u64, reason: String) {
        if self.generation.is_some_and(|current| current != generation) {
            return;
        }
        self.generation = None;
        self.ready = false;
        self.slots.clear();
        self.timeline_subscription = None;
        self.latest_timeline_request = None;
        for (_, pending) in self.pending.drain() {
            let error = || anyhow!("disconnected: {reason}");
            match pending {
                Pending::CreateAgent(reply) => {
                    let _ = reply.send(Err(error()));
                }
                Pending::CreateTerminal(reply) => {
                    let _ = reply.send(Err(error()));
                }
                Pending::SubscribeTerminal {
                    terminal_id,
                    reply: Some((reply, _)),
                } => {
                    self.terminals.remove(&terminal_id);
                    let _ = reply.send(Err(error()));
                }
                _ => {}
            }
        }
        for sync in self.timelines.values_mut() {
            sync.disconnected();
        }
        self.events.push(ClientEvent::Disconnected { reason });
    }

    // ---- sending ----

    fn send(&mut self, message: Value) -> bool {
        let Some(generation) = self.generation else {
            return false;
        };
        match session_frame(message) {
            Ok(text) => {
                self.conn.send_text(generation, text);
                true
            }
            Err(error) => {
                log::error!("{error}");
                false
            }
        }
    }

    /// Sends a request and remembers what its response is for. Returns false when it could
    /// not be sent; the caller then fails its reply.
    fn request(&mut self, mut message: Value, pending: Pending) -> Result<(), Pending> {
        self.next_request += 1;
        let request_id = format!("bgc-{}", self.next_request);
        message["requestId"] = json!(request_id);
        if !self.send(message) {
            return Err(pending);
        }
        self.pending.insert(request_id, pending);
        Ok(())
    }

    fn send_plan(&mut self, agent_id: &str, plan: FetchPlan) {
        let Some(sync) = self.timelines.get(agent_id) else {
            return;
        };
        let generation = sync.generation();
        let message = plan.request(agent_id);
        let _ = self.request(
            message,
            Pending::Timeline {
                agent_id: agent_id.to_string(),
                plan,
                generation,
            },
        );
    }

    fn update_timeline_subscription(&mut self) {
        if self.viewed.is_empty() {
            self.latest_timeline_request = None;
            if let Some(old) = self.timeline_subscription.take() {
                self.release(&old);
            }
            return;
        }
        let agent_ids = self.viewed.clone();
        let request_id = format!("bgc-{}", self.next_request + 1);
        if self
            .request(
                json!({"type": "agent.timeline.set_subscription.request", "agentIds": agent_ids}),
                Pending::TimelineSubscription,
            )
            .is_ok()
        {
            self.latest_timeline_request = Some(request_id);
        }
    }

    fn release(&mut self, subscription_id: &str) {
        let _ = self.request(
            json!({"type": "subscription.release.request", "subscriptionId": subscription_id}),
            Pending::Action("subscription.release"),
        );
    }

    // ---- public requests ----

    pub fn view_agent(&mut self, agent_id: &str) {
        if self.viewed.iter().any(|id| id == agent_id) {
            return;
        }
        self.viewed.push(agent_id.to_string());
        let mut sync = TimelineSync::new();
        if let Some(extra) = self.extras.get(agent_id) {
            sync.state.pending_permissions = extra.permissions.clone();
        }
        let plan = sync.plan_tail();
        self.timelines.insert(agent_id.to_string(), sync);
        if self.generation.is_some() {
            // Subscribe first so rows after the tail page are buffered, not missed.
            self.update_timeline_subscription();
            self.send_plan(agent_id, plan);
        }
    }

    pub fn unview_agent(&mut self, agent_id: &str) {
        let Some(index) = self.viewed.iter().position(|id| id == agent_id) else {
            return;
        };
        self.viewed.remove(index);
        self.timelines.remove(agent_id);
        self.dirty_timelines.remove(agent_id);
        if self.generation.is_some() {
            self.update_timeline_subscription();
        }
    }

    pub fn load_older(&mut self, agent_id: &str) {
        let Some(plan) = self
            .timelines
            .get_mut(agent_id)
            .and_then(TimelineSync::load_older)
        else {
            return;
        };
        self.send_plan(agent_id, plan);
    }

    pub fn send_prompt(&mut self, agent_id: &str, text: String, images: Vec<Value>) {
        let mut message = json!({
            "type": "send_agent_message_request",
            "agentId": agent_id,
            "text": text,
            "messageId": uuid::Uuid::new_v4().to_string(),
        });
        if !images.is_empty() {
            message["images"] = Value::Array(images);
        }
        let _ = self.request(message, Pending::Action("send_agent_message"));
    }

    pub fn cancel_agent(&mut self, agent_id: &str) {
        let _ = self.request(
            json!({"type": "cancel_agent_request", "agentId": agent_id}),
            Pending::Action("cancel_agent"),
        );
    }

    pub fn set_mode(&mut self, agent_id: &str, mode_id: &str) {
        let _ = self.request(
            json!({"type": "set_agent_mode_request", "agentId": agent_id, "modeId": mode_id}),
            Pending::Action("set_agent_mode"),
        );
    }

    pub fn set_model(&mut self, agent_id: &str, model_id: &str) {
        let _ = self.request(
            json!({"type": "set_agent_model_request", "agentId": agent_id, "modelId": model_id}),
            Pending::Action("set_agent_model"),
        );
    }

    /// Answers a permission request. A request with actions gets the first action of the
    /// chosen behavior, as the app's default buttons do.
    pub fn respond_permission(&mut self, agent_id: &str, request_id: &str, allow: bool) {
        let behavior = if allow { "allow" } else { "deny" };
        let mut response = json!({"behavior": behavior});
        let action = self
            .extras
            .get(agent_id)
            .and_then(|extra| extra.permissions.iter().find(|p| p["id"] == request_id))
            .and_then(|request| {
                request["actions"]
                    .as_array()?
                    .iter()
                    .find(|action| action["behavior"] == behavior)?["id"]
                    .as_str()
                    .map(str::to_string)
            });
        if let Some(action) = action {
            response["selectedActionId"] = json!(action);
        }
        // The daemon answers with `agent_permission_resolved`, not a response.
        self.send(json!({
            "type": "agent_permission_response",
            "agentId": agent_id,
            "requestId": request_id,
            "response": response,
        }));
    }

    pub fn create_agent(
        &mut self,
        provider: &str,
        model: Option<&str>,
        cwd: &str,
        prompt: &str,
    ) -> oneshot::Receiver<anyhow::Result<String>> {
        let (reply, receiver) = oneshot::channel();
        let mut config = json!({"provider": provider, "cwd": cwd});
        if let Some(model) = model {
            config["model"] = json!(model);
        }
        let mut message = json!({"type": "create_agent_request", "config": config, "labels": {}});
        if !prompt.is_empty() {
            message["initialPrompt"] = json!(prompt);
            message["clientMessageId"] = json!(uuid::Uuid::new_v4().to_string());
        }
        if let Err(Pending::CreateAgent(reply)) = self.request(message, Pending::CreateAgent(reply))
        {
            let _ = reply.send(Err(anyhow!("not connected")));
        }
        receiver
    }

    pub fn create_terminal(
        &mut self,
        cwd: &str,
    ) -> oneshot::Receiver<anyhow::Result<TerminalHandle>> {
        let (reply, receiver) = oneshot::channel();
        let mut message = json!({"type": "create_terminal_request", "cwd": cwd});
        // The daemon needs a workspace; it resolves one from cwd only for known folders.
        if let Some(workspace) = self
            .workspaces
            .iter()
            .find(|workspace| workspace.cwd == cwd)
        {
            message["workspaceId"] = json!(workspace.id);
        }
        if let Err(Pending::CreateTerminal(reply)) =
            self.request(message, Pending::CreateTerminal(reply))
        {
            let _ = reply.send(Err(anyhow!("not connected")));
        }
        receiver
    }

    pub fn attach_terminal(
        &mut self,
        terminal_id: &str,
    ) -> oneshot::Receiver<anyhow::Result<TerminalHandle>> {
        let (reply, receiver) = oneshot::channel();
        if self.generation.is_none() {
            let _ = reply.send(Err(anyhow!("not connected")));
        } else {
            self.subscribe_terminal(terminal_id.to_string(), Some(reply));
        }
        receiver
    }

    /// Subscribes to a terminal. Without `reply` it renews the subscription of an open handle.
    ///
    /// The request carries no `restore` mode on purpose: with `restore` the daemon sends the
    /// state as ANSI text (opcode 0x05), and only without it does it send the JSON cell grid
    /// (opcode 0x04) that `TerminalFrame::Snapshot` promises. Both carry the full scrollback.
    fn subscribe_terminal(&mut self, terminal_id: String, reply: Option<Reply<TerminalHandle>>) {
        let reply = reply.map(|reply| {
            let (sender, receiver) = mpsc::unbounded();
            self.terminals
                .insert(terminal_id.clone(), TerminalStream { sender, slot: None });
            (reply, receiver)
        });
        let pending = Pending::SubscribeTerminal {
            terminal_id: terminal_id.clone(),
            reply,
        };
        let message = json!({"type": "subscribe_terminal_request", "terminalId": terminal_id});
        if let Err(Pending::SubscribeTerminal {
            reply: Some((reply, _)),
            ..
        }) = self.request(message, pending)
        {
            self.terminals.remove(&terminal_id);
            let _ = reply.send(Err(anyhow!("not connected")));
        }
    }

    pub fn terminal_input(&mut self, terminal_id: &str, bytes: &[u8]) {
        self.send_terminal(terminal_id, terminal::INPUT, bytes);
    }

    pub fn terminal_resize(&mut self, terminal_id: &str, rows: u16, cols: u16, claim: bool) {
        let intent = if claim { "claim" } else { "update" };
        let payload = json!({"rows": rows, "cols": cols, "intent": intent}).to_string();
        self.send_terminal(terminal_id, terminal::RESIZE, payload.as_bytes());
    }

    fn send_terminal(&mut self, terminal_id: &str, opcode: u8, payload: &[u8]) {
        let (Some(generation), Some(slot)) = (
            self.generation,
            self.terminals
                .get(terminal_id)
                .and_then(|stream| stream.slot),
        ) else {
            return;
        };
        self.conn
            .send_binary(generation, terminal::encode(opcode, slot, payload));
    }

    pub fn kill_terminal(&mut self, terminal_id: &str) {
        if let Some(stream) = self.terminals.remove(terminal_id)
            && let Some(slot) = stream.slot
        {
            self.slots.remove(&slot);
        }
        let _ = self.request(
            json!({"type": "kill_terminal_request", "terminalId": terminal_id}),
            Pending::Action("kill_terminal"),
        );
    }

    // ---- incoming ----

    fn binary(&mut self, bytes: &[u8]) {
        let Some((slot, frame)) = terminal::parse_frame(bytes) else {
            return;
        };
        let Some(terminal_id) = self.slots.get(&slot).cloned() else {
            return;
        };
        let delivered = self
            .terminals
            .get(&terminal_id)
            .is_some_and(|stream| stream.sender.unbounded_send(frame).is_ok());
        if !delivered {
            // The view dropped its handle.
            self.terminals.remove(&terminal_id);
            self.slots.remove(&slot);
            self.send(json!({"type": "unsubscribe_terminal_request", "terminalId": terminal_id}));
        }
    }

    fn message(&mut self, message: Value) {
        let kind = message["type"].as_str().unwrap_or_default().to_string();
        if let Some(pending) = message["payload"]["requestId"]
            .as_str()
            .and_then(|id| self.pending.remove(id))
        {
            self.response(pending, &kind, &message["payload"]);
            return;
        }
        let payload = &message["payload"];
        match kind.as_str() {
            "agent_update" => match payload["kind"].as_str() {
                Some("upsert") => self.upsert_agent(&payload["agent"]),
                Some("remove") => {
                    self.remove_agent(payload["agentId"].as_str().unwrap_or_default())
                }
                _ => {}
            },
            "agent_deleted" => self.remove_agent(payload["agentId"].as_str().unwrap_or_default()),
            "terminal_stream_exit" => {
                // Dropping the sender ends the view's frame stream.
                let terminal_id = payload["terminalId"].as_str().unwrap_or_default();
                if let Some(stream) = self.terminals.remove(terminal_id)
                    && let Some(slot) = stream.slot
                {
                    self.slots.remove(&slot);
                }
            }
            "workspace_update" => match payload["kind"].as_str() {
                Some("upsert") => self.upsert_workspace(&payload["workspace"]),
                Some("remove") => {
                    let id = payload["id"].as_str().unwrap_or_default();
                    self.workspaces.retain(|workspace| workspace.id != id);
                    self.directory_changed = true;
                }
                _ => {}
            },
            "agent_stream" => self.stream(payload),
            "agent.timeline.replacement" => {
                let agent_id = payload["agentId"].as_str().unwrap_or_default().to_string();
                if let Some(sync) = self.timelines.get_mut(&agent_id) {
                    let plan = sync.plan_tail();
                    self.send_plan(&agent_id, plan);
                }
            }
            "agent_permission_request" => {
                let agent_id = payload["agentId"].as_str().unwrap_or_default().to_string();
                self.add_permission(&agent_id, payload["request"].clone());
            }
            "agent_permission_resolved" => {
                let agent_id = payload["agentId"].as_str().unwrap_or_default().to_string();
                self.remove_permission(
                    &agent_id,
                    payload["requestId"].as_str().unwrap_or_default(),
                );
            }
            _ => {}
        }
    }

    fn response(&mut self, pending: Pending, kind: &str, payload: &Value) {
        let error = if kind == "rpc_error" {
            Some(
                payload["error"]
                    .as_str()
                    .unwrap_or("request failed")
                    .to_string(),
            )
        } else {
            payload["error"].as_str().map(str::to_string)
        };
        match pending {
            Pending::Agents { first } => {
                if let Some(error) = error {
                    return log::warn!("fetch_agents failed: {error}");
                }
                self.agents_page(payload, first);
            }
            Pending::Workspaces { first } => {
                if let Some(error) = error {
                    return log::warn!("fetch_workspaces failed: {error}");
                }
                self.workspaces_page(payload, first);
            }
            Pending::Providers => {
                if let Some(error) = error {
                    return log::warn!("list_available_providers failed: {error}");
                }
                self.providers_list(payload);
            }
            Pending::Models(provider) => {
                if let Some(error) = error {
                    return log::warn!("list_provider_models {provider} failed: {error}");
                }
                let models: Vec<(String, String)> = payload["models"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter(|model| model["isSelectable"] != false)
                    .filter_map(|model| {
                        let id = model["id"].as_str()?.to_string();
                        let label = model["label"].as_str().unwrap_or(&id).to_string();
                        Some((id, label))
                    })
                    .collect();
                if let Some(summary) = self.providers.iter_mut().find(|p| p.id == provider) {
                    summary.models = models;
                    self.directory_changed = true;
                }
            }
            Pending::Timeline {
                agent_id,
                plan,
                generation,
            } => {
                let page = match &error {
                    Some(error) if kind == "rpc_error" => json!({"error": error}),
                    _ => payload.clone(),
                };
                let Some(sync) = self.timelines.get_mut(&agent_id) else {
                    return;
                };
                let version = sync.state.version;
                let next = sync.on_page(&plan, generation, &page);
                if sync.state.version != version {
                    self.dirty_timelines.insert(agent_id.clone());
                }
                if let Some(next) = next {
                    self.send_plan(&agent_id, next);
                }
            }
            Pending::TimelineSubscription => {
                let Some(new_id) = payload["subscriptionId"].as_str().map(str::to_string) else {
                    if let Some(error) = error {
                        log::warn!("timeline subscription failed: {error}");
                    }
                    return;
                };
                let request_id = payload["requestId"].as_str();
                if request_id.is_some() && request_id == self.latest_timeline_request.as_deref() {
                    // Release the old set only after the new one holds, so no row is missed;
                    // rows delivered twice meanwhile are dropped by sequence.
                    if let Some(old) = self.timeline_subscription.replace(new_id) {
                        self.release(&old);
                    }
                } else {
                    self.release(&new_id);
                }
            }
            Pending::CreateAgent(reply) => {
                let result = match (payload["status"].as_str(), payload["agentId"].as_str()) {
                    (Some("agent_created"), Some(id)) => Ok(id.to_string()),
                    _ => Err(anyhow!(
                        error.unwrap_or_else(|| "agent creation failed".into())
                    )),
                };
                let _ = reply.send(result);
            }
            Pending::CreateTerminal(reply) => match payload["terminal"]["id"].as_str() {
                Some(id) if error.is_none() => self.subscribe_terminal(id.to_string(), Some(reply)),
                _ => {
                    let error = error.unwrap_or_else(|| "terminal creation failed".into());
                    let _ = reply.send(Err(anyhow!(error)));
                }
            },
            Pending::SubscribeTerminal { terminal_id, reply } => {
                let slot = payload["slot"].as_u64().filter(|_| error.is_none());
                let Some(slot) = slot.and_then(|slot| u8::try_from(slot).ok()) else {
                    let error = error.unwrap_or_else(|| "terminal subscription failed".into());
                    if let Some((reply, _)) = reply {
                        self.terminals.remove(&terminal_id);
                        let _ = reply.send(Err(anyhow!(error)));
                    } else {
                        // The terminal is gone after a reconnect; end the view's stream.
                        log::warn!("terminal {terminal_id} resubscribe failed: {error}");
                        self.terminals.remove(&terminal_id);
                    }
                    return;
                };
                let Some(stream) = self.terminals.get_mut(&terminal_id) else {
                    return;
                };
                stream.slot = Some(slot);
                self.slots.insert(slot, terminal_id.clone());
                if let Some((reply, frames)) = reply {
                    let handle = TerminalHandle {
                        id: terminal_id.clone(),
                        frames,
                    };
                    if reply.send(Ok(handle)).is_err() {
                        // Nobody waits for the handle any more.
                        self.terminals.remove(&terminal_id);
                        self.slots.remove(&slot);
                        self.send(
                            json!({"type": "unsubscribe_terminal_request", "terminalId": terminal_id}),
                        );
                    }
                }
            }
            Pending::Action(name) => {
                if let Some(error) = error {
                    log::warn!("{name} failed: {error}");
                }
            }
        }
    }

    // ---- directory ----

    fn agents_page(&mut self, payload: &Value, first: bool) {
        let entries = payload["entries"]
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or_default();
        if first {
            let fresh: BTreeSet<&str> = entries
                .iter()
                .filter_map(|entry| entry["agent"]["id"].as_str())
                .collect();
            let gone: Vec<String> = self
                .agents
                .iter()
                .filter(|agent| !fresh.contains(agent.id.as_str()))
                .map(|agent| agent.id.clone())
                .collect();
            for id in gone {
                self.remove_agent(&id);
            }
        }
        for entry in entries {
            self.upsert_agent(&entry["agent"]);
        }
        self.directory_changed = true;
        if payload["pageInfo"]["hasMore"] == true
            && let Some(cursor) = payload["pageInfo"]["nextCursor"].as_str()
        {
            let _ = self.request(
                json!({
                    "type": "fetch_agents_request",
                    "sort": [{"key": "updated_at", "direction": "desc"}],
                    "page": {"limit": 200, "cursor": cursor},
                }),
                Pending::Agents { first: false },
            );
            return;
        }
        self.bootstrapped = true;
    }

    fn upsert_agent(&mut self, agent: &Value) {
        let Some(id) = agent["id"].as_str() else {
            return;
        };
        if !agent["archivedAt"].is_null() {
            self.remove_agent(id);
            return;
        }
        let runtime = &agent["runtimeInfo"];
        let permissions: Vec<Value> = agent["pendingPermissions"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let summary = AgentSummary {
            id: id.to_string(),
            title: agent["title"]
                .as_str()
                .filter(|title| !title.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| id.chars().take(8).collect()),
            status: agent["status"].as_str().unwrap_or("unknown").to_string(),
            cwd: agent["cwd"].as_str().unwrap_or_default().to_string(),
            workspace_id: agent["workspaceId"].as_str().map(str::to_string),
            provider: agent["provider"].as_str().unwrap_or_default().to_string(),
            model: agent["model"]
                .as_str()
                .or_else(|| runtime["model"].as_str())
                .map(str::to_string),
            pending_permissions: permissions.len(),
        };
        let modes = agent["availableModes"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|mode| {
                let id = mode["id"].as_str()?.to_string();
                let label = mode["label"].as_str().unwrap_or(&id).to_string();
                Some((id, label))
            })
            .collect();
        let current_mode = agent["currentModeId"]
            .as_str()
            .or_else(|| runtime["modeId"].as_str())
            .map(str::to_string);
        match self.agents.iter_mut().find(|existing| existing.id == id) {
            Some(existing) => {
                if existing.status == "running" && summary.status != "running" {
                    self.events.push(ClientEvent::AgentFinished {
                        agent_id: id.to_string(),
                    });
                }
                *existing = summary;
            }
            None => self.agents.push(summary),
        }
        let extra = self.extras.entry(id.to_string()).or_default();
        extra.modes = modes;
        extra.current_mode = current_mode;
        self.set_permissions(id, permissions);
        self.directory_changed = true;
    }

    fn remove_agent(&mut self, id: &str) {
        let before = self.agents.len();
        self.agents.retain(|agent| agent.id != id);
        self.extras.remove(id);
        if self.agents.len() != before {
            self.directory_changed = true;
        }
    }

    fn workspaces_page(&mut self, payload: &Value, first: bool) {
        if first {
            self.workspaces.clear();
        }
        for workspace in payload["entries"].as_array().into_iter().flatten() {
            self.upsert_workspace(workspace);
        }
        self.directory_changed = true;
        if payload["pageInfo"]["hasMore"] == true
            && let Some(cursor) = payload["pageInfo"]["nextCursor"].as_str()
        {
            let _ = self.request(
                json!({
                    "type": "fetch_workspaces_request",
                    "page": {"limit": 200, "cursor": cursor},
                }),
                Pending::Workspaces { first: false },
            );
        }
    }

    fn upsert_workspace(&mut self, workspace: &Value) {
        let Some(id) = workspace["id"].as_str() else {
            return;
        };
        let summary = WorkspaceSummary {
            id: id.to_string(),
            name: workspace["name"].as_str().unwrap_or(id).to_string(),
            cwd: workspace["workspaceDirectory"]
                .as_str()
                .or_else(|| workspace["projectRootPath"].as_str())
                .unwrap_or_default()
                .to_string(),
        };
        match self
            .workspaces
            .iter_mut()
            .find(|existing| existing.id == id)
        {
            Some(existing) => *existing = summary,
            None => self.workspaces.push(summary),
        }
        self.directory_changed = true;
    }

    fn providers_list(&mut self, payload: &Value) {
        let previous = std::mem::take(&mut self.providers);
        for provider in payload["providers"].as_array().into_iter().flatten() {
            let Some(id) = provider["provider"].as_str() else {
                continue;
            };
            let available = provider["available"] == true;
            let models = previous
                .iter()
                .find(|p| p.id == id)
                .map(|p| p.models.clone())
                .unwrap_or_default();
            self.providers.push(ProviderSummary {
                id: id.to_string(),
                label: provider_label(id),
                available,
                models,
            });
            if available {
                let _ = self.request(
                    json!({"type": "list_provider_models_request", "provider": id}),
                    Pending::Models(id.to_string()),
                );
            }
        }
        self.directory_changed = true;
    }

    // ---- live stream and permissions ----

    fn stream(&mut self, payload: &Value) {
        let Some(agent_id) = payload["agentId"].as_str().map(str::to_string) else {
            return;
        };
        let event = &payload["event"];
        match event["type"].as_str() {
            Some("timeline") => {
                let Some(row) = LiveRow::from_stream(payload) else {
                    return;
                };
                let Some(sync) = self.timelines.get_mut(&agent_id) else {
                    return;
                };
                let version = sync.state.version;
                let plan = sync.on_live(row);
                if sync.state.version != version {
                    self.dirty_timelines.insert(agent_id.clone());
                }
                if let Some(plan) = plan {
                    self.send_plan(&agent_id, plan);
                }
            }
            Some("permission_requested") => {
                self.add_permission(&agent_id, event["request"].clone())
            }
            Some("permission_resolved") => {
                self.remove_permission(&agent_id, event["requestId"].as_str().unwrap_or_default())
            }
            _ => {}
        }
    }

    fn add_permission(&mut self, agent_id: &str, request: Value) {
        let Some(extra) = self.extras.get(agent_id) else {
            return;
        };
        if extra.permissions.iter().any(|p| p["id"] == request["id"]) {
            return;
        }
        let mut permissions = extra.permissions.clone();
        permissions.push(request);
        self.set_permissions(agent_id, permissions);
    }

    fn remove_permission(&mut self, agent_id: &str, request_id: &str) {
        let Some(extra) = self.extras.get(agent_id) else {
            return;
        };
        let permissions: Vec<Value> = extra
            .permissions
            .iter()
            .filter(|p| p["id"] != request_id)
            .cloned()
            .collect();
        self.set_permissions(agent_id, permissions);
    }

    /// Stores an agent's open permission requests and reports new ones.
    fn set_permissions(&mut self, agent_id: &str, permissions: Vec<Value>) {
        let extra = self.extras.entry(agent_id.to_string()).or_default();
        if extra.permissions == permissions {
            return;
        }
        let is_new = permissions
            .iter()
            .any(|p| !extra.permissions.iter().any(|old| old["id"] == p["id"]));
        extra.permissions = permissions;
        if is_new && self.bootstrapped {
            self.events.push(ClientEvent::PermissionRequested {
                agent_id: agent_id.to_string(),
            });
        }
        let count = extra.permissions.len();
        if let Some(agent) = self.agents.iter_mut().find(|agent| agent.id == agent_id) {
            agent.pending_permissions = count;
        }
        if let Some(sync) = self.timelines.get_mut(agent_id) {
            sync.state.pending_permissions = extra.permissions.clone();
            sync.state.version += 1;
            self.dirty_timelines.insert(agent_id.to_string());
        }
        self.directory_changed = true;
    }
}
