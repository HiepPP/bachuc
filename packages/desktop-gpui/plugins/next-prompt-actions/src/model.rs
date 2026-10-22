//! `PanelModel`: one per agent, shared by every mounted panel of that agent, as the shared React
//! queries of `panel.tsx:613-645` are. `Reads` decides when it calls `prompts.scope` and
//! `prompts.inspect`, without GPUI.

use crate::contracts::{
    INSPECT, SCOPE, START, Scope, ScopeInput, SendOutput, Snapshot, StartInput, StartOutput,
};
use crate::polling::{inspection_activity, inspection_interval};
use crate::rules::{SendStep, host_state_changed, panel_busy, send_landed, send_plan};
use bachuc_client::{AttentionReason, Client, ClientEvent, PluginEvent};
use bachuc_plugin_host::{OpenSurfaceOptions, PluginContext};
use gpui_kit::{App, AppContext as _, Context, Entity, Subscription, Task, WeakEntity};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

/// A read the model sends now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Read {
    Scope,
    Inspect,
    /// Inspect again after this delay.
    PollIn(Duration),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ScopeState {
    Pending,
    Ready,
    Failed,
}

/// When to read. The scope is read once per agent and workspace; the snapshot after the scope,
/// on a status change, after an action, after a dependency plugin change, and on a timer only
/// while Jev reviews.
#[derive(Debug, Default)]
pub(crate) struct Reads {
    connected: bool,
    /// The workspace the scope was asked for, and how that read went.
    scope: Option<(Option<String>, ScopeState)>,
    scope_generation: u64,
    status: Option<String>,
}

impl Reads {
    /// The agent as the client shows it now. A request sent before `Connected` fails, so
    /// nothing is read while disconnected, and a failed scope read retries after a reconnect.
    pub(crate) fn agent(
        &mut self,
        connected: bool,
        workspace_id: Option<&str>,
        status: Option<&str>,
    ) -> Vec<Read> {
        if !connected {
            self.connected = false;
            return Vec::new();
        }
        let reconnected = !self.connected;
        self.connected = true;
        // The GPUI client has no last activity time, so the status alone keys a read.
        let activity = status.map(|status| inspection_activity(status, None).status);
        let changed = self.status != activity;
        self.status = activity;
        match &self.scope {
            Some((asked, state)) if asked.as_deref() == workspace_id => match state {
                ScopeState::Pending => Vec::new(),
                ScopeState::Failed if reconnected => self.ask_scope(workspace_id),
                ScopeState::Failed => Vec::new(),
                ScopeState::Ready if changed || reconnected => vec![Read::Inspect],
                ScopeState::Ready => Vec::new(),
            },
            _ => self.ask_scope(workspace_id),
        }
    }

    fn ask_scope(&mut self, workspace_id: Option<&str>) -> Vec<Read> {
        self.scope = Some((workspace_id.map(str::to_string), ScopeState::Pending));
        self.scope_generation += 1;
        vec![Read::Scope]
    }

    /// The generation a scope answer must carry to apply.
    pub(crate) fn scope_generation(&self) -> u64 {
        self.scope_generation
    }

    /// A scope answer. A stale one (the workspace changed since) applies nothing.
    pub(crate) fn scope_landed(&mut self, generation: u64, ok: bool) -> Option<Vec<Read>> {
        if generation != self.scope_generation {
            return None;
        }
        let (_, state) = self.scope.as_mut()?;
        *state = if ok {
            ScopeState::Ready
        } else {
            ScopeState::Failed
        };
        Some(if ok { vec![Read::Inspect] } else { Vec::new() })
    }

    fn ready(&self) -> bool {
        matches!(self.scope, Some((_, ScopeState::Ready)))
    }

    /// A snapshot landed with this note.
    pub(crate) fn snapshot_landed(&self, note: Option<&str>) -> Vec<Read> {
        inspection_interval(note)
            .filter(|_| self.ready())
            .map(Read::PollIn)
            .into_iter()
            .collect()
    }

    pub(crate) fn poll_fired(&self) -> Vec<Read> {
        if self.ready() {
            vec![Read::Inspect]
        } else {
            Vec::new()
        }
    }

    pub(crate) fn action_done(&self) -> Vec<Read> {
        self.poll_fired()
    }

    pub(crate) fn plugin_changed(&self) -> Vec<Read> {
        self.poll_fired()
    }
}

pub(crate) struct PanelModel {
    ctx: PluginContext,
    /// Held strong: a subscription holds its emitter weakly.
    client: Option<Entity<Client>>,
    agent_id: String,
    reads: Reads,
    scope: Option<Scope>,
    snapshot: Option<Snapshot>,
    pending: bool,
    error: String,
    scope_error: Option<String>,
    inspect_error: Option<String>,
    inspect_generation: u64,
    busy_key: (Option<String>, Option<AttentionReason>),
    poll: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl PanelModel {
    fn new(ctx: PluginContext, agent_id: String, cx: &mut Context<Self>) -> Self {
        let client = ctx.client(cx);
        let mut subscriptions = Vec::new();
        if let Some(client) = &client {
            subscriptions.push(cx.subscribe(
                client,
                |model, _, event: &ClientEvent, cx| match event {
                    ClientEvent::Connected
                    | ClientEvent::Disconnected { .. }
                    | ClientEvent::AgentsChanged => model.refresh(cx),
                    ClientEvent::AgentFinished { agent_id }
                    | ClientEvent::PermissionRequested { agent_id }
                        if *agent_id == model.agent_id =>
                    {
                        model.refresh(cx)
                    }
                    _ => {}
                },
            ));
            subscriptions.push(cx.subscribe(client, |model, _, event: &PluginEvent, cx| {
                if let PluginEvent::CatalogChanged { plugin_id } = event
                    && host_state_changed(plugin_id)
                {
                    let reads = model.reads.plugin_changed();
                    model.run(reads, cx);
                }
            }));
        }
        Self {
            ctx,
            client,
            agent_id,
            reads: Reads::default(),
            scope: None,
            snapshot: None,
            pending: false,
            error: String::new(),
            scope_error: None,
            inspect_error: None,
            inspect_generation: 0,
            busy_key: (None, None),
            poll: None,
            _subscriptions: subscriptions,
        }
    }

    /// The agent's status and attention reason.
    fn agent_state(
        &self,
        cx: &App,
    ) -> (
        bool,
        Option<String>,
        Option<String>,
        Option<AttentionReason>,
    ) {
        let Some(client) = &self.client else {
            return (false, None, None, None);
        };
        let client = client.read(cx);
        let agent = client.agents().iter().find(|a| a.id == self.agent_id);
        (
            client.is_connected(),
            agent.and_then(|a| a.workspace_id.clone()),
            agent.map(|a| a.status.clone()),
            client
                .agent_attention(&self.agent_id)
                .and_then(|(_, reason)| reason),
        )
    }

    /// Reads the client again; redraws the panels when the busy inputs changed.
    fn refresh(&mut self, cx: &mut Context<Self>) {
        let (connected, workspace_id, status, attention) = self.agent_state(cx);
        let busy_key = (status.clone(), attention);
        if busy_key != self.busy_key {
            self.busy_key = busy_key;
            cx.notify();
        }
        let reads = self
            .reads
            .agent(connected, workspace_id.as_deref(), status.as_deref());
        self.run(reads, cx);
    }

    fn run(&mut self, reads: Vec<Read>, cx: &mut Context<Self>) {
        for read in reads {
            match read {
                Read::Scope => self.read_scope(cx),
                Read::Inspect => self.inspect(cx),
                Read::PollIn(delay) => {
                    self.poll = Some(cx.spawn(async move |model, cx| {
                        cx.background_executor().timer(delay).await;
                        model
                            .update(cx, |model, cx| {
                                let reads = model.reads.poll_fired();
                                model.run(reads, cx);
                            })
                            .ok();
                    }));
                }
            }
        }
    }

    fn read_scope(&mut self, cx: &mut Context<Self>) {
        let generation = self.reads.scope_generation();
        self.scope = None;
        self.scope_error = None;
        let input = ScopeInput {
            agent_id: &self.agent_id,
        };
        let task = self.ctx.rpc_as::<_, Scope>(SCOPE, &input, cx);
        cx.notify();
        cx.spawn(async move |model, cx| {
            let result = task.await;
            model
                .update(cx, |model, cx| {
                    let Some(reads) = model.reads.scope_landed(generation, result.is_ok()) else {
                        return;
                    };
                    match result {
                        Ok(scope) => model.scope = Some(scope),
                        Err(error) => model.scope_error = Some(format!("{error:#}")),
                    }
                    cx.notify();
                    model.run(reads, cx);
                })
                .ok();
        })
        .detach();
    }

    fn inspect(&mut self, cx: &mut Context<Self>) {
        let Some(scope) = self.scope.clone() else {
            return;
        };
        self.inspect_generation += 1;
        let generation = self.inspect_generation;
        let task = self.ctx.rpc_as::<_, Snapshot>(INSPECT, &scope, cx);
        cx.spawn(async move |model, cx| {
            let result = task.await;
            model
                .update(cx, |model, cx| {
                    if generation != model.inspect_generation {
                        return;
                    }
                    match result {
                        Ok(snapshot) => {
                            model.snapshot = Some(snapshot);
                            model.inspect_error = None;
                        }
                        // The last snapshot stays, as `keepPreviousData` keeps it.
                        Err(error) => model.inspect_error = Some(format!("{error:#}")),
                    }
                    let reads = model.reads.snapshot_landed(model.note());
                    // A newer snapshot replaces the timer; one without a review drops it.
                    model.poll = None;
                    cx.notify();
                    model.run(reads, cx);
                })
                .ok();
        })
        .detach();
    }

    pub(crate) fn snapshot(&self) -> Option<&Snapshot> {
        self.snapshot.as_ref()
    }

    pub(crate) fn note(&self) -> Option<&str> {
        self.snapshot.as_ref().map(|s| s.note.as_str())
    }

    pub(crate) fn pending(&self) -> bool {
        self.pending
    }

    pub(crate) fn load_error(&self) -> Option<&str> {
        self.scope_error
            .as_deref()
            .or(self.inspect_error.as_deref())
    }

    /// The notices under the cards: the action error, the load error, the note, the warning.
    pub(crate) fn notices(&self) -> Vec<String> {
        let snapshot = self.snapshot.as_ref();
        [
            Some(self.error.as_str()),
            self.load_error(),
            snapshot.map(|s| s.note.as_str()),
            snapshot.and_then(|s| s.warning.as_deref()),
        ]
        .into_iter()
        .flatten()
        .filter(|notice| !notice.is_empty())
        .map(str::to_string)
        .collect()
    }

    pub(crate) fn busy(&self) -> bool {
        panel_busy(
            self.pending,
            self.load_error().is_some(),
            self.busy_key.0.as_deref(),
            self.busy_key.1,
            self.note(),
        )
    }

    /// Runs one action: sets `pending`, clears the error, stores a failure as the error, and
    /// reads the snapshot after.
    fn act<T: 'static>(&mut self, task: Task<anyhow::Result<T>>, cx: &mut Context<Self>) {
        self.pending = true;
        self.error.clear();
        cx.notify();
        cx.spawn(async move |model, cx| {
            let result = task.await;
            model
                .update(cx, |model, cx| {
                    model.pending = false;
                    if let Err(error) = result {
                        model.error = format!("{error:#}");
                    }
                    cx.notify();
                    let reads = model.reads.action_done();
                    model.run(reads, cx);
                })
                .ok();
        })
        .detach();
    }

    pub(crate) fn send(&mut self, keys: Vec<String>, back_to_board: bool, cx: &mut Context<Self>) {
        let Some(scope) = self.scope.clone() else {
            return;
        };
        let mut rpc = None;
        for step in send_plan(back_to_board, scope, keys) {
            match step {
                SendStep::MarkPending(agent_id) => {
                    if let Some(client) = &self.client {
                        client.update(cx, |client, cx| client.mark_turn_pending(&agent_id, cx));
                    }
                }
                SendStep::OpenBoard => self.ctx.open_surface(
                    "board",
                    OpenSurfaceOptions {
                        plugin_id: Some("board".into()),
                        ..OpenSurfaceOptions::default()
                    },
                    cx,
                ),
                SendStep::Rpc(input) => {
                    rpc = Some(
                        self.ctx
                            .rpc_as::<_, SendOutput>(crate::contracts::SEND, &input, cx),
                    )
                }
            }
        }
        let Some(rpc) = rpc else {
            return;
        };
        let client = self.client.clone();
        let agent_id = self.agent_id.clone();
        // The mark clears when the send does not land; `act` still gets the result.
        let task = cx.spawn(async move |_, cx| {
            let result = rpc.await;
            if !send_landed(&result)
                && let Some(client) = client
            {
                client.update(cx, |client, cx| client.clear_turn_pending(&agent_id, cx));
            }
            result
        });
        self.act(task, cx);
    }

    pub(crate) fn start(&mut self, key: String, cx: &mut Context<Self>) {
        let Some(scope) = self.scope.clone() else {
            return;
        };
        let rpc = self
            .ctx
            .rpc_as::<_, StartOutput>(START, &StartInput { scope, key }, cx);
        self.act(rpc, cx);
    }
}

/// The models of the mounted panels by agent id. A model lives while a panel holds it.
#[derive(Default)]
pub(crate) struct Models {
    models: RefCell<Vec<(String, WeakEntity<PanelModel>)>>,
}

impl Models {
    pub(crate) fn get(
        self: &Rc<Self>,
        ctx: &PluginContext,
        agent_id: &str,
        cx: &mut App,
    ) -> Entity<PanelModel> {
        let mut models = self.models.borrow_mut();
        models.retain(|(_, model)| model.upgrade().is_some());
        if let Some(model) = models
            .iter()
            .find(|(id, _)| id == agent_id)
            .and_then(|(_, model)| model.upgrade())
        {
            return model;
        }
        let (ctx, id) = (ctx.clone(), agent_id.to_string());
        let model = cx.new(|cx| PanelModel::new(ctx, id, cx));
        models.push((agent_id.to_string(), model.downgrade()));
        drop(models);
        model.update(cx, |model, cx| model.refresh(cx));
        model
    }
}

#[cfg(test)]
mod tests {
    use super::{Read, Reads};
    use crate::polling::JEV_REVIEWING;
    use std::time::Duration;

    fn count(reads: &[Read], kind: &Read) -> usize {
        reads.iter().filter(|read| *read == kind).count()
    }

    #[test]
    fn idle_agent_no_timed_inspect() {
        let mut reads = Reads::default();
        let mut sent = Vec::new();
        sent.extend(reads.agent(true, Some("w"), Some("idle")));
        let generation = reads.scope_generation();
        sent.extend(reads.scope_landed(generation, true).unwrap());
        sent.extend(reads.snapshot_landed(Some("")));
        // Other agents' changes and repeated idle states read nothing.
        for _ in 0..50 {
            sent.extend(reads.agent(true, Some("w"), Some("idle")));
            sent.extend(reads.snapshot_landed(Some("Auto-run is off.")));
        }
        assert_eq!(count(&sent, &Read::Inspect), 1);
        assert!(!sent.iter().any(|read| matches!(read, Read::PollIn(_))));
        // A status change and an action each read once.
        sent.extend(reads.agent(true, Some("w"), Some("running")));
        sent.extend(reads.agent(true, Some("w"), Some("running")));
        sent.extend(reads.agent(true, Some("w"), Some("idle")));
        sent.extend(reads.action_done());
        assert_eq!(count(&sent, &Read::Inspect), 4);
        assert!(!sent.iter().any(|read| matches!(read, Read::PollIn(_))));
    }

    #[test]
    fn jev_review_polls_every_2500_ms() {
        let mut reads = Reads::default();
        reads.agent(true, Some("w"), Some("idle"));
        let generation = reads.scope_generation();
        reads.scope_landed(generation, true);
        let poll = vec![Read::PollIn(Duration::from_millis(2500))];
        assert_eq!(reads.snapshot_landed(Some(JEV_REVIEWING)), poll);
        assert_eq!(reads.poll_fired(), vec![Read::Inspect]);
        assert_eq!(reads.snapshot_landed(Some(JEV_REVIEWING)), poll);
        assert_eq!(reads.snapshot_landed(Some("")), vec![]);
        assert_eq!(reads.snapshot_landed(None), vec![]);
    }

    #[test]
    fn scope_once_per_workspace() {
        let mut reads = Reads::default();
        let mut sent = Vec::new();
        // Nothing is read before the client connects.
        sent.extend(reads.agent(false, Some("w"), Some("idle")));
        assert!(sent.is_empty());
        for status in ["idle", "running", "idle", "running"] {
            sent.extend(reads.agent(true, Some("w"), Some(status)));
        }
        assert_eq!(count(&sent, &Read::Scope), 1);
        let first = reads.scope_generation();
        sent.extend(reads.agent(true, Some("w2"), Some("idle")));
        assert_eq!(count(&sent, &Read::Scope), 2);
        // The answer for the old workspace is stale and applies nothing.
        assert_eq!(reads.scope_landed(first, true), None);
        let second = reads.scope_generation();
        assert_eq!(reads.scope_landed(second, true), Some(vec![Read::Inspect]));
        sent.extend(reads.agent(true, Some("w2"), Some("running")));
        assert_eq!(count(&sent, &Read::Scope), 2);
        // A failed read retries only after a reconnect.
        sent.extend(reads.agent(true, Some("w3"), Some("idle")));
        let third = reads.scope_generation();
        reads.scope_landed(third, false);
        sent.extend(reads.agent(true, Some("w3"), Some("running")));
        assert_eq!(count(&sent, &Read::Scope), 3);
        reads.agent(false, Some("w3"), Some("running"));
        sent.extend(reads.agent(true, Some("w3"), Some("running")));
        assert_eq!(count(&sent, &Read::Scope), 4);
    }

    #[test]
    fn plugin_change_inspects_once_the_scope_is_ready() {
        let mut reads = Reads::default();
        assert!(reads.plugin_changed().is_empty(), "no scope yet");
        reads.agent(true, Some("w"), Some("idle"));
        assert!(reads.plugin_changed().is_empty(), "scope still pending");
        let generation = reads.scope_generation();
        reads.scope_landed(generation, true);
        assert_eq!(reads.plugin_changed(), vec![Read::Inspect]);
    }
}
