//! The client state machine, free of GPUI: it turns worker events into state and requests.
//! `Client` wraps it in a GPUI entity; tests drive it directly.

use crate::connection::{Connection, ConnectionConfig, WorkerEvent, session_frame};
use crate::files::{self, TransferFrame};
use crate::model::{
    AgentControls, AgentFeature, AgentFeatureKind, AgentFeatureOption, AgentSummary,
    AttentionReason, ChangedFile, CheckoutDetails, CheckoutStatus, ClientEvent, CommitList,
    DiffStat, DirectoryEvent, FileContent, FileDiff, FileEntry, ProjectNode, ProviderSummary,
    QueuedPrompt, SlashCommand, ThinkingControls, TurnBehavior, WorkspaceNode, WorkspaceSummary,
    provider_label,
};
use crate::terminal::{self, TerminalFrame, TerminalHandle};
use crate::timeline::{FetchPlan, LiveRow, TimelineState, TimelineSync};
use anyhow::anyhow;
use bachuc_protocol::ServerInfo;
use bachuc_protocol::generated::UpsertAgentFeaturesItem;
use futures::channel::{mpsc, oneshot};
use serde::Deserialize as _;
use serde_json::{Value, json};
use std::collections::{BTreeSet, HashMap};

type Reply<T> = oneshot::Sender<anyhow::Result<T>>;

/// The diff the changed files come from, as the app's working diff shows it by default.
const UNCOMMITTED: &str = "uncommitted";

enum Pending {
    Projects,
    FileList(Reply<Vec<FileEntry>>),
    /// `transfer` holds the `FileBegin` metadata and the chunks so far.
    ReadFile {
        path: String,
        reply: Reply<FileContent>,
        transfer: Option<Box<(Value, Vec<u8>)>>,
    },
    CheckoutStatus(CheckoutReply),
    CheckoutFiles {
        branch: Option<String>,
        reply: CheckoutReply,
    },
    CheckoutWatch(String),
    FileDiff {
        path: String,
        reply: Reply<String>,
    },
    CheckoutDiff(Reply<Vec<FileDiff>>),
    Commits(Reply<CommitList>),
    CommitFileDiff(Reply<Option<FileDiff>>),
    SetFeature(Reply<()>),
    Commands(Reply<Vec<SlashCommand>>),
    Queue(String),
    ReadConfig {
        root: String,
        reply: Reply<Value>,
    },
    WriteConfig {
        root: String,
        reply: Reply<()>,
    },
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

impl Pending {
    /// Fails the reply this request carries, if any.
    fn fail(self, error: anyhow::Error) {
        fn send<T>(reply: Reply<T>, error: anyhow::Error) {
            let _ = reply.send(Err(error));
        }
        match self {
            Pending::CreateAgent(reply) => send(reply, error),
            Pending::CreateTerminal(reply) => send(reply, error),
            Pending::SubscribeTerminal {
                reply: Some((reply, _)),
                ..
            } => send(reply, error),
            Pending::FileList(reply) => send(reply, error),
            Pending::ReadFile { reply, .. } => send(reply, error),
            Pending::CheckoutStatus(reply) | Pending::CheckoutFiles { reply, .. } => {
                reply.send(Err(error))
            }
            Pending::FileDiff { reply, .. } => send(reply, error),
            Pending::CheckoutDiff(reply) => send(reply, error),
            Pending::Commits(reply) => send(reply, error),
            Pending::CommitFileDiff(reply) => send(reply, error),
            Pending::SetFeature(reply) => send(reply, error),
            Pending::Commands(reply) => send(reply, error),
            Pending::ReadConfig { reply, .. } => send(reply, error),
            Pending::WriteConfig { reply, .. } => send(reply, error),
            _ => {}
        }
    }
}

/// Where a checkout status answer goes: `checkout_status` or `checkout_details`.
enum CheckoutReply {
    Status(Reply<CheckoutStatus>),
    Details(Reply<CheckoutDetails>),
}

impl CheckoutReply {
    fn send(self, result: anyhow::Result<CheckoutDetails>) {
        match self {
            CheckoutReply::Status(reply) => {
                let _ = reply.send(result.map(|details| CheckoutStatus {
                    branch: details.branch,
                    files: details.files.into_iter().map(ChangedFile::from).collect(),
                }));
            }
            CheckoutReply::Details(reply) => {
                let _ = reply.send(result);
            }
        }
    }
}

#[derive(Default)]
struct AgentExtra {
    modes: Vec<(String, String)>,
    current_mode: Option<String>,
    permissions: Vec<Value>,
    thinking: Option<String>,
    queue: Vec<QueuedPrompt>,
    updated_at: Option<String>,
    requires_attention: bool,
    attention_reason: Option<AttentionReason>,
    features: Vec<AgentFeature>,
}

/// A project as `project.list` or a workspace describes it.
#[derive(Clone)]
struct ProjectInfo {
    id: String,
    name: String,
    root: String,
    /// `projectKey`; a workspace descriptor has none.
    key: Option<String>,
}

/// The tree fields of a workspace that `WorkspaceSummary` does not hold.
struct WorkspaceMeta {
    project: ProjectInfo,
    branch: Option<String>,
    pinned_at: Option<String>,
    diff_stat: Option<DiffStat>,
}

/// Thinking options of one model, as `(id, label)`, and its default.
struct ModelThinking {
    options: Vec<(String, String)>,
    default: Option<String>,
}

struct TerminalStream {
    sender: mpsc::UnboundedSender<TerminalFrame>,
    slot: Option<u8>,
}

/// Session events this client subscribes to; with `explicit_event_subscriptions` the daemon
/// sends them only on request.
const SESSION_EVENTS: [&str; 3] = [
    "agent_permission_request",
    "agent_permission_resolved",
    "project.update",
];

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
    typed_server_info: Option<ServerInfo>,
    project_list: Vec<ProjectInfo>,
    /// `emptyProjects` of the workspace list, for a daemon without `project.list`.
    empty_projects: Vec<ProjectInfo>,
    workspace_meta: HashMap<String, WorkspaceMeta>,
    projects: Vec<ProjectNode>,
    projects_dirty: bool,
    directory_events: Vec<DirectoryEvent>,
    /// Keyed by `(provider, model id)`.
    model_thinking: HashMap<(String, String), ModelThinking>,
    default_models: HashMap<String, String>,
    /// Watched folders and their diff subscription, `None` until the daemon answers.
    checkout_watches: HashMap<String, Option<String>>,
    /// The revision of the last project config read or written, per repo root.
    config_revisions: HashMap<String, Value>,
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
            typed_server_info: None,
            project_list: Vec::new(),
            empty_projects: Vec::new(),
            workspace_meta: HashMap::new(),
            projects: Vec::new(),
            projects_dirty: false,
            directory_events: Vec::new(),
            model_thinking: HashMap::new(),
            default_models: HashMap::new(),
            checkout_watches: HashMap::new(),
            config_revisions: HashMap::new(),
        }
    }

    /// The `server_info` of the current connection as the generated type. `None` before the
    /// first connection, or when a newer daemon sends a value this build cannot decode.
    pub fn typed_server_info(&self) -> Option<&ServerInfo> {
        self.typed_server_info.as_ref()
    }

    /// Whether the host holds prompts sent during a turn (`features.agentMessageQueue`).
    pub fn supports_queue(&self) -> bool {
        self.typed_server_info
            .as_ref()
            .and_then(|info| info.features.as_ref())
            .and_then(|features| features.agent_message_queue)
            == Some(true)
    }

    /// Projects with their workspaces, in the daemon's project order.
    pub fn projects(&self) -> &[ProjectNode] {
        &self.projects
    }

    /// Project tree and checkout events since the last call.
    pub fn take_directory_events(&mut self) -> Vec<DirectoryEvent> {
        std::mem::take(&mut self.directory_events)
    }

    pub fn queued_prompts(&self, agent_id: &str) -> Vec<QueuedPrompt> {
        self.extras
            .get(agent_id)
            .map(|extra| extra.queue.clone())
            .unwrap_or_default()
    }

    /// The thinking options of the agent's model. `None` when the model has none.
    pub fn agent_thinking(&self, agent_id: &str) -> Option<ThinkingControls> {
        let agent = self.agents.iter().find(|agent| agent.id == agent_id)?;
        let model = agent
            .model
            .clone()
            .or_else(|| self.default_models.get(&agent.provider).cloned())?;
        let thinking = self.model_thinking.get(&(agent.provider.clone(), model))?;
        if thinking.options.is_empty() {
            return None;
        }
        let current = self
            .extras
            .get(agent_id)
            .and_then(|extra| extra.thinking.clone())
            .or_else(|| thinking.default.clone());
        Some(ThinkingControls {
            options: thinking.options.clone(),
            current,
        })
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
        if self.directory_changed || std::mem::take(&mut self.projects_dirty) {
            self.rebuild_projects();
        }
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

    /// Whether the agent asks for attention, and why. `None` for an unknown agent.
    pub fn agent_attention(&self, agent_id: &str) -> Option<(bool, Option<AttentionReason>)> {
        if !self.agents.iter().any(|agent| agent.id == agent_id) {
            return None;
        }
        let extra = self.extras.get(agent_id)?;
        Some((extra.requires_attention, extra.attention_reason))
    }

    /// The agent's provider features. A feature type this client does not know is left out.
    pub fn agent_features(&self, agent_id: &str) -> Vec<AgentFeature> {
        self.extras
            .get(agent_id)
            .map(|extra| extra.features.clone())
            .unwrap_or_default()
    }

    /// The key the app groups a project by: its `projectKey`, else `["<serverId>","<projectId>"]`
    /// as `createProjectViewKey` builds it. `None` for an unknown project.
    pub fn project_view_key(&self, project_id: &str) -> Option<String> {
        let mut matches = self
            .project_list
            .iter()
            .chain(&self.empty_projects)
            .chain(self.workspace_meta.values().map(|meta| &meta.project))
            .filter(|project| project.id == project_id)
            .peekable();
        matches.peek()?;
        if let Some(key) = matches.find_map(|project| project.key.clone()) {
            return Some(key);
        }
        let server_id = self.server_info.as_ref()?["serverId"].as_str()?;
        Some(json!([server_id, project_id]).to_string())
    }

    pub fn workspace_diff_stat(&self, workspace_id: &str) -> Option<DiffStat> {
        self.workspace_meta.get(workspace_id)?.diff_stat
    }

    pub fn workspace_pinned_at(&self, workspace_id: &str) -> Option<String> {
        self.workspace_meta.get(workspace_id)?.pinned_at.clone()
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
                    self.typed_server_info = bachuc_protocol::decode_server_info(&server_info)
                        .inspect_err(|error| log::warn!("server_info does not decode: {error}"))
                        .ok();
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
        let _ = self.request(json!({"type": "project.list.request"}), Pending::Projects);
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
        let watched: Vec<String> = self.checkout_watches.keys().cloned().collect();
        for cwd in watched {
            self.subscribe_checkout(cwd);
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
            if let Pending::SubscribeTerminal {
                terminal_id,
                reply: Some(_),
            } = &pending
            {
                self.terminals.remove(terminal_id);
            }
            pending.fail(anyhow!("disconnected: {reason}"));
        }
        for subscription in self.checkout_watches.values_mut() {
            *subscription = None;
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
        self.send_message(agent_id, text, images, None);
    }

    /// Sends a prompt with what it does to a running turn.
    pub fn send_prompt_with(
        &mut self,
        agent_id: &str,
        text: String,
        images: Vec<Value>,
        behavior: TurnBehavior,
    ) {
        self.send_message(agent_id, text, images, Some(behavior));
    }

    fn send_message(
        &mut self,
        agent_id: &str,
        text: String,
        images: Vec<Value>,
        behavior: Option<TurnBehavior>,
    ) {
        let mut message = json!({
            "type": "send_agent_message_request",
            "agentId": agent_id,
            "text": text,
            "messageId": uuid::Uuid::new_v4().to_string(),
        });
        if !images.is_empty() {
            message["images"] = Value::Array(images);
        }
        if let Some(behavior) = behavior {
            message["activeTurnBehavior"] = json!(behavior.wire());
        }
        let _ = self.request(message, Pending::Action("send_agent_message"));
    }

    /// Queues a prompt behind the running turn. Needs `supports_queue`.
    pub fn enqueue_prompt(&mut self, agent_id: &str, text: String, images: Vec<Value>) {
        let mut message =
            json!({"type": "agent.queue.enqueue.request", "agentId": agent_id, "text": text});
        if !images.is_empty() {
            message["images"] = Value::Array(images);
        }
        let _ = self.request(message, Pending::Queue(agent_id.to_string()));
    }

    pub fn cancel_queued(&mut self, agent_id: &str, queued_id: &str) {
        let _ = self.request(
            json!({"type": "agent.queue.cancel.request", "agentId": agent_id, "itemId": queued_id}),
            Pending::Queue(agent_id.to_string()),
        );
    }

    pub fn set_thinking(&mut self, agent_id: &str, option_id: &str) {
        let _ = self.request(
            json!({
                "type": "set_agent_thinking_request",
                "agentId": agent_id,
                "thinkingOptionId": option_id,
            }),
            Pending::Action("set_agent_thinking"),
        );
    }

    pub fn archive_agent(&mut self, agent_id: &str) {
        let _ = self.request(
            json!({"type": "archive_agent_request", "agentId": agent_id}),
            Pending::Action("archive_agent"),
        );
    }

    pub fn rename_agent(&mut self, agent_id: &str, title: &str) {
        let _ = self.request(
            json!({"type": "update_agent_request", "agentId": agent_id, "name": title}),
            Pending::Action("update_agent"),
        );
    }

    pub fn rename_workspace(&mut self, workspace_id: &str, title: &str) {
        let _ = self.request(
            json!({"type": "workspace.title.set.request", "workspaceId": workspace_id, "title": title}),
            Pending::Action("workspace.title.set"),
        );
    }

    pub fn pin_workspace(&mut self, workspace_id: &str, pinned: bool) {
        let _ = self.request(
            json!({"type": "workspace.pin.set.request", "workspaceId": workspace_id, "pinned": pinned}),
            Pending::Action("workspace.pin.set"),
        );
    }

    pub fn archive_workspace(&mut self, workspace_id: &str) {
        let _ = self.request(
            json!({"type": "archive_workspace_request", "workspaceId": workspace_id}),
            Pending::Action("archive_workspace"),
        );
    }

    pub fn mark_workspace_unread(&mut self, workspace_id: &str) {
        let _ = self.request(
            json!({"type": "workspace.mark_unread.request", "workspaceId": workspace_id}),
            Pending::Action("workspace.mark_unread"),
        );
    }

    /// Answers a permission request with a full `AgentPermissionResponse`: question answers,
    /// plan approval, or `updatedInput`.
    pub fn respond_permission_with(&mut self, agent_id: &str, request_id: &str, response: Value) {
        self.send(json!({
            "type": "agent_permission_response",
            "agentId": agent_id,
            "requestId": request_id,
            "response": response,
        }));
    }

    /// Sends a request whose answer resolves the returned receiver.
    fn request_reply<T>(
        &mut self,
        message: Value,
        pending: impl FnOnce(Reply<T>) -> Pending,
    ) -> oneshot::Receiver<anyhow::Result<T>> {
        let (reply, receiver) = oneshot::channel();
        if let Err(pending) = self.request(message, pending(reply)) {
            pending.fail(anyhow!("not connected"));
        }
        receiver
    }

    pub fn list_commands(
        &mut self,
        agent_id: &str,
    ) -> oneshot::Receiver<anyhow::Result<Vec<SlashCommand>>> {
        self.request_reply(
            json!({"type": "list_commands_request", "agentId": agent_id}),
            Pending::Commands,
        )
    }

    pub fn list_directory(
        &mut self,
        cwd: &str,
        path: &str,
    ) -> oneshot::Receiver<anyhow::Result<Vec<FileEntry>>> {
        self.request_reply(
            json!({"type": "file_explorer_request", "cwd": cwd, "path": path, "mode": "list"}),
            Pending::FileList,
        )
    }

    /// Reads a file. The daemon streams it as binary frames; an older one answers in JSON.
    pub fn read_file(
        &mut self,
        cwd: &str,
        path: &str,
    ) -> oneshot::Receiver<anyhow::Result<FileContent>> {
        self.request_reply(
            json!({
                "type": "file_explorer_request",
                "cwd": cwd,
                "path": path,
                "mode": "file",
                "acceptBinary": true,
            }),
            |reply| Pending::ReadFile {
                path: path.to_string(),
                reply,
                transfer: None,
            },
        )
    }

    /// The branch and the uncommitted changed files of a folder.
    pub fn checkout_status(
        &mut self,
        cwd: &str,
    ) -> oneshot::Receiver<anyhow::Result<CheckoutStatus>> {
        self.request_reply(
            json!({"type": "checkout_status_request", "cwd": cwd}),
            |reply| Pending::CheckoutStatus(CheckoutReply::Status(reply)),
        )
    }

    /// `checkout_status` with the git flag and the full file fields.
    pub fn checkout_details(
        &mut self,
        cwd: &str,
    ) -> oneshot::Receiver<anyhow::Result<CheckoutDetails>> {
        self.request_reply(
            json!({"type": "checkout_status_request", "cwd": cwd}),
            |reply| Pending::CheckoutStatus(CheckoutReply::Details(reply)),
        )
    }

    /// Every uncommitted file of `cwd` with its diff, from one request. A folder outside git
    /// gives no files.
    pub fn checkout_diff(&mut self, cwd: &str) -> oneshot::Receiver<anyhow::Result<Vec<FileDiff>>> {
        self.request_reply(
            json!({"type": "checkout.diff.get.request", "cwd": cwd, "compare": {"mode": UNCOMMITTED}}),
            Pending::CheckoutDiff,
        )
    }

    pub fn commits(&mut self, cwd: &str) -> oneshot::Receiver<anyhow::Result<CommitList>> {
        self.request_reply(
            json!({"type": "checkout.commits.list.request", "cwd": cwd}),
            Pending::Commits,
        )
    }

    /// The diff of one file in one commit; `None` when the commit has no text diff for it.
    pub fn commit_file_diff(
        &mut self,
        cwd: &str,
        sha: &str,
        path: &str,
    ) -> oneshot::Receiver<anyhow::Result<Option<FileDiff>>> {
        self.request_reply(
            json!({"type": "checkout.commits.file_diff.request", "cwd": cwd, "sha": sha, "path": path}),
            Pending::CommitFileDiff,
        )
    }

    /// Sets a provider feature; `value` is a bool for a toggle and an option id for a select.
    pub fn set_agent_feature(
        &mut self,
        agent_id: &str,
        feature_id: &str,
        value: Value,
    ) -> oneshot::Receiver<anyhow::Result<()>> {
        self.request_reply(
            json!({
                "type": "set_agent_feature_request",
                "agentId": agent_id,
                "featureId": feature_id,
                "value": value,
            }),
            Pending::SetFeature,
        )
    }

    /// The unified diff of one uncommitted file; empty when the file has no change.
    pub fn file_diff(
        &mut self,
        cwd: &str,
        path: &str,
    ) -> oneshot::Receiver<anyhow::Result<String>> {
        self.request_reply(
            json!({"type": "checkout.diff.get.request", "cwd": cwd, "compare": {"mode": UNCOMMITTED}}),
            |reply| Pending::FileDiff {
                path: path.to_string(),
                reply,
            },
        )
    }

    /// Emits `CheckoutChanged` for `cwd` whenever its changed files change.
    pub fn watch_checkout(&mut self, cwd: &str) {
        if self.checkout_watches.contains_key(cwd) {
            return;
        }
        self.checkout_watches.insert(cwd.to_string(), None);
        self.subscribe_checkout(cwd.to_string());
    }

    pub fn unwatch_checkout(&mut self, cwd: &str) {
        if let Some(Some(subscription_id)) = self.checkout_watches.remove(cwd) {
            self.send(json!({
                "type": "unsubscribe_checkout_diff_request",
                "subscriptionId": subscription_id,
            }));
        }
    }

    fn subscribe_checkout(&mut self, cwd: String) {
        let message = json!({
            "type": "subscribe_checkout_diff_request",
            "cwd": cwd,
            "compare": {"mode": UNCOMMITTED},
        });
        let _ = self.request(message, Pending::CheckoutWatch(cwd));
    }

    /// The project's `paseo.json` (null when it has none). The revision is kept for the next
    /// write.
    pub fn read_project_config(&mut self, root: &str) -> oneshot::Receiver<anyhow::Result<Value>> {
        self.request_reply(
            json!({"type": "read_project_config_request", "repoRoot": root}),
            |reply| Pending::ReadConfig {
                root: root.to_string(),
                reply,
            },
        )
    }

    /// Writes the project's `paseo.json`. It fails as stale when the file changed after the
    /// last read or write through this client.
    pub fn write_project_config(
        &mut self,
        root: &str,
        config: Value,
    ) -> oneshot::Receiver<anyhow::Result<()>> {
        let revision = self
            .config_revisions
            .get(root)
            .cloned()
            .unwrap_or(Value::Null);
        self.request_reply(
            json!({
                "type": "write_project_config_request",
                "repoRoot": root,
                "config": config,
                "expectedRevision": revision,
            }),
            |reply| Pending::WriteConfig {
                root: root.to_string(),
                reply,
            },
        )
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
        if bytes
            .first()
            .copied()
            .is_some_and(files::is_transfer_opcode)
        {
            return self.transfer(bytes);
        }
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

    /// A file transfer frame of a `read_file` request. `FileEnd` completes the read; the
    /// daemon sends no JSON response then.
    fn transfer(&mut self, bytes: &[u8]) {
        let Some(frame) = files::parse_transfer(bytes) else {
            return;
        };
        match frame {
            TransferFrame::Begin {
                request_id,
                metadata,
            } => {
                if let Some(Pending::ReadFile { transfer, .. }) = self.pending.get_mut(&request_id)
                {
                    *transfer = Some(Box::new((metadata, Vec::new())));
                }
            }
            TransferFrame::Chunk { request_id, bytes } => {
                if let Some(Pending::ReadFile {
                    transfer: Some(transfer),
                    ..
                }) = self.pending.get_mut(&request_id)
                {
                    transfer.1.extend_from_slice(&bytes);
                }
            }
            TransferFrame::End { request_id } => {
                if let Some(Pending::ReadFile {
                    transfer: Some(_), ..
                }) = self.pending.get(&request_id)
                    && let Some(Pending::ReadFile {
                        path,
                        reply,
                        transfer: Some(transfer),
                    }) = self.pending.remove(&request_id)
                {
                    let (metadata, buffer) = *transfer;
                    let _ = reply.send(Ok(files::streamed_file(&path, &metadata, buffer)));
                }
            }
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
                    self.workspace_meta.remove(id);
                    self.directory_changed = true;
                }
                _ => {}
            },
            "project.update" => match payload["kind"].as_str() {
                Some("upsert") => {
                    if let Some(project) = project_info(&payload["project"]) {
                        match self.project_list.iter_mut().find(|p| p.id == project.id) {
                            Some(existing) => *existing = project,
                            None => self.project_list.push(project),
                        }
                        self.projects_dirty = true;
                    }
                }
                Some("remove") => {
                    let id = payload["projectId"].as_str().unwrap_or_default();
                    self.project_list.retain(|project| project.id != id);
                    self.empty_projects.retain(|project| project.id != id);
                    self.projects_dirty = true;
                }
                _ => {}
            },
            "checkout_diff_update" => {
                let subscription = payload["subscriptionId"].as_str().unwrap_or_default();
                let cwd = self
                    .checkout_watches
                    .iter()
                    .find(|(_, id)| id.as_deref() == Some(subscription))
                    .map(|(cwd, _)| cwd.clone());
                if let Some(cwd) = cwd {
                    self.directory_events
                        .push(DirectoryEvent::CheckoutChanged { cwd });
                }
            }
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
                for model in payload["models"].as_array().into_iter().flatten() {
                    let Some(id) = model["id"].as_str() else {
                        continue;
                    };
                    if model["isDefault"] == true {
                        self.default_models.insert(provider.clone(), id.to_string());
                    }
                    let options = model["thinkingOptions"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|option| {
                            let id = option["id"].as_str()?.to_string();
                            let label = option["label"].as_str().unwrap_or(&id).to_string();
                            Some((id, label))
                        })
                        .collect();
                    let default = model["defaultThinkingOptionId"]
                        .as_str()
                        .or_else(|| {
                            model["thinkingOptions"]
                                .as_array()?
                                .iter()
                                .find(|option| option["isDefault"] == true)?["id"]
                                .as_str()
                        })
                        .map(str::to_string);
                    let thinking = ModelThinking { options, default };
                    self.model_thinking
                        .insert((provider.clone(), id.to_string()), thinking);
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
            pending => self.parity_response(pending, kind, payload, error),
        }
    }

    /// Answers of the requests added for the parity views.
    fn parity_response(
        &mut self,
        pending: Pending,
        kind: &str,
        payload: &Value,
        error: Option<String>,
    ) {
        // A checkout error is `{code, message}`, which `error` above does not read.
        let rpc_error = (kind == "rpc_error").then(|| error.clone()).flatten();
        match pending {
            Pending::Projects => {
                if let Some(error) = error {
                    return log::warn!("project.list failed: {error}");
                }
                self.project_list = payload["projects"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(project_info)
                    .collect();
                self.projects_dirty = true;
            }
            Pending::FileList(reply) => {
                let result = match (error, payload["directory"].is_object()) {
                    (Some(error), _) => Err(anyhow!(error)),
                    (None, true) => Ok(files::directory_entries(&payload["directory"])),
                    (None, false) => Err(anyhow!("directory listing unavailable")),
                };
                let _ = reply.send(result);
            }
            Pending::ReadFile { reply, .. } => {
                let result = match (error, payload["file"].is_object()) {
                    (Some(error), _) => Err(anyhow!(error)),
                    (None, true) => files::json_file(&payload["file"]),
                    (None, false) => Err(anyhow!("file unavailable")),
                };
                let _ = reply.send(result);
            }
            Pending::CheckoutStatus(reply) => {
                if let Some(error) = rpc_error {
                    reply.send(Err(anyhow!(error)));
                    return;
                }
                if payload["isGit"] != true {
                    let result = match files::checkout_error(payload) {
                        Some((code, message)) if code != "NOT_GIT_REPO" => Err(anyhow!(message)),
                        _ => Ok(CheckoutDetails {
                            is_git: false,
                            branch: None,
                            files: Vec::new(),
                        }),
                    };
                    reply.send(result);
                    return;
                }
                if let Some((_, message)) = files::checkout_error(payload) {
                    reply.send(Err(anyhow!(message)));
                    return;
                }
                // A detached HEAD has no branch to show, as in the app.
                let branch = payload["currentBranch"]
                    .as_str()
                    .filter(|branch| *branch != "HEAD")
                    .map(str::to_string);
                let cwd = payload["cwd"].as_str().unwrap_or_default();
                let message = json!({
                    "type": "checkout.diff.get.request",
                    "cwd": cwd,
                    "compare": {"mode": UNCOMMITTED},
                });
                if let Err(pending) =
                    self.request(message, Pending::CheckoutFiles { branch, reply })
                {
                    pending.fail(anyhow!("not connected"));
                }
            }
            Pending::CheckoutFiles { branch, reply } => {
                let result = match (rpc_error, files::checkout_error(payload)) {
                    (Some(error), _) => Err(anyhow!(error)),
                    (None, Some((code, message))) if code != "NOT_GIT_REPO" => {
                        Err(anyhow!(message))
                    }
                    (None, error) => Ok(CheckoutDetails {
                        is_git: true,
                        branch,
                        files: if error.is_some() {
                            Vec::new()
                        } else {
                            files::changed_files(&payload["files"])
                        },
                    }),
                };
                reply.send(result);
            }
            Pending::CheckoutWatch(cwd) => {
                let Some(subscription_id) = payload["subscriptionId"].as_str() else {
                    let error = rpc_error.or(files::checkout_error(payload).map(|(_, m)| m));
                    return log::warn!("checkout watch of {cwd} failed: {error:?}");
                };
                match self.checkout_watches.get_mut(&cwd) {
                    Some(slot) => {
                        *slot = Some(subscription_id.to_string());
                        self.directory_events
                            .push(DirectoryEvent::CheckoutChanged { cwd });
                    }
                    // Unwatched while the subscription was on its way.
                    None => {
                        self.send(json!({
                            "type": "unsubscribe_checkout_diff_request",
                            "subscriptionId": subscription_id,
                        }));
                    }
                }
            }
            Pending::FileDiff { path, reply } => {
                let result = match (rpc_error, files::checkout_error(payload)) {
                    (Some(error), _) => Err(anyhow!(error)),
                    (None, Some((_, message))) => Err(anyhow!(message)),
                    (None, None) => {
                        let file = payload["files"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .find(|file| file["path"] == path.as_str());
                        match file {
                            None => Ok(String::new()),
                            Some(file) => match file["status"].as_str().unwrap_or("ok") {
                                "ok" => Ok(files::unified_diff(file)),
                                status => Err(anyhow!("{path} has no diff: {status}")),
                            },
                        }
                    }
                };
                let _ = reply.send(result);
            }
            Pending::CheckoutDiff(reply) => {
                let result = match (rpc_error, files::checkout_error(payload)) {
                    (Some(error), _) => Err(anyhow!(error)),
                    (None, Some((code, _))) if code == "NOT_GIT_REPO" => Ok(Vec::new()),
                    (None, Some((_, message))) => Err(anyhow!(message)),
                    (None, None) => Ok(payload["files"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(files::file_diff)
                        .collect()),
                };
                let _ = reply.send(result);
            }
            Pending::Commits(reply) => {
                let result = match (rpc_error, files::checkout_error(payload)) {
                    (Some(error), _) => Err(anyhow!(error)),
                    (None, Some((_, message))) => Err(anyhow!(message)),
                    (None, None) => Ok(files::commit_list(payload)),
                };
                let _ = reply.send(result);
            }
            Pending::CommitFileDiff(reply) => {
                let result = match (rpc_error, files::checkout_error(payload)) {
                    (Some(error), _) => Err(anyhow!(error)),
                    (None, Some((_, message))) => Err(anyhow!(message)),
                    (None, None) => Ok(files::file_diff(&payload["file"])),
                };
                let _ = reply.send(result);
            }
            Pending::SetFeature(reply) => {
                let result = match error {
                    Some(error) => Err(anyhow!(error)),
                    None if payload["accepted"] == false => Err(anyhow!("feature change refused")),
                    None => Ok(()),
                };
                let _ = reply.send(result);
            }
            Pending::Commands(reply) => {
                let result = match error {
                    Some(error) => Err(anyhow!(error)),
                    None => Ok(payload["commands"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|command| {
                            Some(SlashCommand {
                                name: command["name"].as_str()?.to_string(),
                                description: command["description"]
                                    .as_str()
                                    .unwrap_or_default()
                                    .to_string(),
                            })
                        })
                        .collect()),
                };
                let _ = reply.send(result);
            }
            Pending::Queue(agent_id) => {
                if let Some(error) = error {
                    return log::warn!("queue request for {agent_id} failed: {error}");
                }
                if payload["queue"].is_object()
                    && let Some(extra) = self.extras.get_mut(&agent_id)
                {
                    extra.queue = queued_prompts(&payload["queue"]);
                    self.directory_changed = true;
                }
            }
            Pending::ReadConfig { root, reply } => {
                let result = config_result(&rpc_error, payload).map(|()| {
                    self.config_revisions
                        .insert(root, payload["revision"].clone());
                    payload["config"].clone()
                });
                let _ = reply.send(result);
            }
            Pending::WriteConfig { root, reply } => {
                let result = config_result(&rpc_error, payload).map(|()| {
                    self.config_revisions
                        .insert(root, payload["revision"].clone());
                });
                let _ = reply.send(result);
            }
            other => other.fail(anyhow!("unexpected response {kind}")),
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
        // The daemon can send an older snapshot after a newer one, for example `idle` after
        // `running` at agent start. ISO timestamps compare as strings.
        let updated_at = agent["updatedAt"].as_str().map(str::to_string);
        let stored = self
            .extras
            .get(id)
            .and_then(|extra| extra.updated_at.as_ref());
        if let (Some(new), Some(old)) = (&updated_at, stored)
            && new < old
        {
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
        extra.thinking = agent["thinkingOptionId"]
            .as_str()
            .or_else(|| runtime["thinkingOptionId"].as_str())
            .map(str::to_string);
        extra.queue = queued_prompts(&agent["queue"]);
        extra.updated_at = updated_at;
        extra.requires_attention = agent["requiresAttention"] == true;
        extra.attention_reason = agent["attentionReason"]
            .as_str()
            .and_then(AttentionReason::parse);
        extra.features = agent_features(&agent["features"]);
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
            self.workspace_meta.clear();
        }
        for workspace in payload["entries"].as_array().into_iter().flatten() {
            self.upsert_workspace(workspace);
        }
        // Projects with no active workspace come only on the first page.
        if first {
            self.empty_projects = payload["emptyProjects"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(project_info)
                .collect();
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
        if let Some(project) = project_info(workspace) {
            let meta = WorkspaceMeta {
                project,
                branch: workspace["gitRuntime"]["currentBranch"]
                    .as_str()
                    .map(str::to_string),
                pinned_at: workspace["pinnedAt"].as_str().map(str::to_string),
                diff_stat: workspace["diffStat"].is_object().then(|| DiffStat {
                    additions: workspace["diffStat"]["additions"]
                        .as_f64()
                        .unwrap_or_default() as u32,
                    deletions: workspace["diffStat"]["deletions"]
                        .as_f64()
                        .unwrap_or_default() as u32,
                }),
            };
            self.workspace_meta.insert(id.to_string(), meta);
        }
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

    // ---- project tree ----

    /// Rebuilds the tree in O(projects + workspaces + agents) and reports a change.
    fn rebuild_projects(&mut self) {
        let mut agents_by_workspace: HashMap<&str, Vec<String>> = HashMap::new();
        for agent in &self.agents {
            if let Some(workspace_id) = &agent.workspace_id {
                agents_by_workspace
                    .entry(workspace_id)
                    .or_default()
                    .push(agent.id.clone());
            }
        }
        let mut order: Vec<ProjectInfo> = self.project_list.clone();
        for project in &self.empty_projects {
            if !order.iter().any(|listed| listed.id == project.id) {
                order.push(project.clone());
            }
        }
        let mut workspaces_by_project: HashMap<String, Vec<WorkspaceNode>> = HashMap::new();
        for workspace in &self.workspaces {
            let Some(meta) = self.workspace_meta.get(&workspace.id) else {
                continue;
            };
            // A workspace whose project is not listed yet still shows, under its own project.
            if !order.iter().any(|project| project.id == meta.project.id) {
                order.push(meta.project.clone());
            }
            workspaces_by_project
                .entry(meta.project.id.clone())
                .or_default()
                .push(WorkspaceNode {
                    id: workspace.id.clone(),
                    name: workspace.name.clone(),
                    cwd: workspace.cwd.clone(),
                    branch: meta.branch.clone(),
                    pinned: meta.pinned_at.is_some(),
                    agent_ids: agents_by_workspace
                        .remove(workspace.id.as_str())
                        .unwrap_or_default(),
                });
        }
        let projects: Vec<ProjectNode> = order
            .into_iter()
            .map(|project| ProjectNode {
                workspaces: workspaces_by_project
                    .remove(&project.id)
                    .unwrap_or_default(),
                id: project.id,
                name: project.name,
                root: project.root,
            })
            .collect();
        if projects != self.projects {
            self.projects = projects;
            self.directory_events.push(DirectoryEvent::ProjectsChanged);
        }
    }
}

/// A project from a project or workspace descriptor; both carry the same project fields.
fn project_info(descriptor: &Value) -> Option<ProjectInfo> {
    Some(ProjectInfo {
        id: descriptor["projectId"].as_str()?.to_string(),
        name: descriptor["projectDisplayName"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        root: descriptor["projectRootPath"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        key: descriptor["projectKey"].as_str().map(str::to_string),
    })
}

/// The snapshot `features`, decoded one by one through the generated union, so a feature type
/// from a newer daemon (`Unrecognized`) or a malformed feature is skipped.
fn agent_features(features: &Value) -> Vec<AgentFeature> {
    features
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|feature| UpsertAgentFeaturesItem::deserialize(feature).ok())
        .filter_map(|feature| match feature {
            UpsertAgentFeaturesItem::Toggle {
                description,
                icon,
                id,
                label,
                tooltip,
                value,
            } => Some(AgentFeature {
                id,
                label,
                description,
                tooltip,
                icon,
                kind: AgentFeatureKind::Toggle { value },
            }),
            UpsertAgentFeaturesItem::Select {
                description,
                icon,
                id,
                label,
                options,
                tooltip,
                value,
            } => Some(AgentFeature {
                id,
                label,
                description,
                tooltip,
                icon,
                kind: AgentFeatureKind::Select {
                    value,
                    options: options
                        .into_iter()
                        .map(|option| AgentFeatureOption {
                            id: option.id,
                            label: option.label,
                            description: option.description,
                            is_default: option.is_default,
                            metadata: option.metadata,
                        })
                        .collect(),
                },
            }),
            UpsertAgentFeaturesItem::Unrecognized => None,
        })
        .collect()
}

fn queued_prompts(queue: &Value) -> Vec<QueuedPrompt> {
    queue["items"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| {
            Some(QueuedPrompt {
                id: item["id"].as_str()?.to_string(),
                text: item["text"].as_str().unwrap_or_default().to_string(),
            })
        })
        .collect()
}

/// The outcome of a project config response: `ok` or an error code such as
/// `stale_project_config`.
fn config_result(rpc_error: &Option<String>, payload: &Value) -> anyhow::Result<()> {
    if let Some(error) = rpc_error {
        anyhow::bail!("{error}");
    }
    if payload["ok"] == true {
        return Ok(());
    }
    let code = payload["error"]["code"]
        .as_str()
        .unwrap_or("project config request failed");
    anyhow::bail!("{code}")
}
