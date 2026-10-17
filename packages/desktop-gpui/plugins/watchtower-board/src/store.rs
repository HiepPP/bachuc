//! `WatchtowerStore`: one per activation, shared by every contribution. It loads boards and
//! overviews through the plugin RPCs, keeps them per workspace, and reads agents from the client
//! for builder matching and the prompt target.

use crate::model::{
    AgentCandidate, AgentTarget, Board, BuilderMatch, Cache, DecisionDetail, DecisionInput, Loaded,
    Overview, OverviewTask, ProjectInput, Want, WorkspaceInput, choose_agent, is_decision_id,
    live_task_ids, match_builders, now_ms,
};
use bachuc_client::{AttentionReason, Client, ClientEvent, OutgoingMessage, TurnBehavior};
use bachuc_plugin_host::PluginContext;
use gpui_kit::{App, AppContext as _, Context, Entity, Subscription, Task};
use std::collections::{BTreeSet, HashMap};

const READ: &str = "watchtower.read";
const PROJECT_READ: &str = "watchtower.project.read";
const OVERVIEW_READ: &str = "watchtower.overview.read";
const DECISION_READ: &str = "watchtower.decision.read";

pub(crate) struct WatchtowerStore {
    ctx: PluginContext,
    /// Held strong: a subscription holds its emitter weakly.
    client: Option<Entity<Client>>,
    boards: Cache<String, Board>,
    project_boards: Cache<String, Board>,
    overviews: Cache<String, Overview>,
    /// The live TASK IDs per workspace, so the pill only reads.
    live: HashMap<String, BTreeSet<String>>,
    /// The workspace the page shows next (`preselectWorkspace`).
    preselected: Option<String>,
    /// `params.task` of the last opened page.
    initial_task: Option<String>,
    /// Grows on every data change, so views know when to rebuild their rows.
    revision: u64,
    _subscriptions: Vec<Subscription>,
}

/// One agent of the client as the target and builder rules read it.
fn candidates(client: &Client) -> Vec<AgentCandidate> {
    client
        .agents()
        .iter()
        .map(|agent| AgentCandidate {
            id: agent.id.clone(),
            title: agent.title.clone(),
            status: agent.status.clone(),
            workspace_id: agent.workspace_id.clone(),
            provider: agent.provider.clone(),
            labels: client.agent_labels(&agent.id).unwrap_or_default(),
            pending_permissions: agent.pending_permissions,
            permission_attention: matches!(
                client.agent_attention(&agent.id),
                Some((true, Some(AttentionReason::Permission)))
            ),
            updated_at: client.agent_updated_at(&agent.id),
        })
        .collect()
}

impl WatchtowerStore {
    pub(crate) fn new(ctx: PluginContext, cx: &mut Context<Self>) -> Self {
        let client = ctx.client(cx);
        let mut subscriptions = Vec::new();
        if let Some(client) = &client {
            subscriptions.push(cx.subscribe(client, |store, _, event: &ClientEvent, cx| {
                match event {
                    // A request sent before `Connected` fails, so load again after each one.
                    ClientEvent::Connected => store.reload_all(cx),
                    ClientEvent::AgentsChanged => store.update_live(cx),
                    _ => {}
                }
            }));
        }
        // The host notifies when the active workspace changes.
        if let Some(host) = bachuc_plugin_host::host(cx) {
            subscriptions.push(cx.observe(&host, |store, _, cx| store.load_active(cx)));
        }
        Self {
            ctx,
            client,
            boards: Cache::default(),
            project_boards: Cache::default(),
            overviews: Cache::default(),
            live: HashMap::new(),
            preselected: None,
            initial_task: None,
            revision: 0,
            _subscriptions: subscriptions,
        }
    }

    pub(crate) fn board(&self, workspace_id: &str) -> Loaded<Board> {
        self.boards.get(&workspace_id.to_string())
    }

    pub(crate) fn project_board(&self, project_id: &str) -> Loaded<Board> {
        self.project_boards.get(&project_id.to_string())
    }

    pub(crate) fn overview(&self, workspace_id: &str) -> Loaded<Overview> {
        self.overviews.get(&workspace_id.to_string())
    }

    pub(crate) fn is_fetching(&self, workspace_id: &str) -> bool {
        let key = workspace_id.to_string();
        self.boards.is_fetching(&key) || self.overviews.is_fetching(&key)
    }

    pub(crate) fn is_fetching_project(&self, project_id: &str) -> bool {
        self.project_boards.is_fetching(&project_id.to_string())
    }

    pub(crate) fn revision(&self) -> u64 {
        self.revision
    }

    /// The TASKs whose builder runs now in this workspace.
    pub(crate) fn live(&self, workspace_id: &str) -> BTreeSet<String> {
        self.live.get(workspace_id).cloned().unwrap_or_default()
    }

    pub(crate) fn preselect(&mut self, workspace_id: String, cx: &mut Context<Self>) {
        if self.preselected.as_deref() != Some(workspace_id.as_str()) {
            self.preselected = Some(workspace_id);
            cx.notify();
        }
    }

    pub(crate) fn preselected(&self) -> Option<&str> {
        self.preselected.as_deref()
    }

    pub(crate) fn set_initial_task(&mut self, task: Option<String>) {
        self.initial_task = task;
    }

    pub(crate) fn initial_task(&self) -> Option<&str> {
        self.initial_task.as_deref()
    }

    /// Loads a workspace's board and overview once; the page calls this.
    pub(crate) fn ensure(&mut self, workspace_id: &str, cx: &mut Context<Self>) {
        self.load(workspace_id, Want::Missing, cx);
    }

    /// Loads a project's board once; the New workspace panel calls this.
    pub(crate) fn ensure_project(&mut self, project_id: &str, cx: &mut Context<Self>) {
        self.load_project(project_id, Want::Missing, cx);
    }

    /// Loads the board and overview again, for the Refresh button.
    pub(crate) fn refresh(&mut self, workspace_id: &str, cx: &mut Context<Self>) {
        self.load(workspace_id, Want::Force, cx);
    }

    pub(crate) fn refresh_project(&mut self, project_id: &str, cx: &mut Context<Self>) {
        self.load_project(project_id, Want::Force, cx);
    }

    pub(crate) fn decision(
        &mut self,
        workspace_id: &str,
        id: &str,
        cx: &mut Context<Self>,
    ) -> Task<anyhow::Result<DecisionDetail>> {
        if !is_decision_id(id) {
            return Task::ready(Err(anyhow::anyhow!("{id} is not an ADR ID.")));
        }
        self.ctx
            .rpc_as(DECISION_READ, &DecisionInput { workspace_id, id }, cx)
    }

    /// Builder matching for one workspace's agents, as the plan tree and the pill read it.
    pub(crate) fn builders(
        &self,
        workspace_id: &str,
        tasks: &[OverviewTask],
        now_ms: i64,
        cx: &App,
    ) -> BuilderMatch {
        let Some(client) = &self.client else {
            return BuilderMatch::default();
        };
        let agents: Vec<AgentCandidate> = candidates(client.read(cx))
            .into_iter()
            .filter(|agent| agent.workspace_id.as_deref() == Some(workspace_id))
            .collect();
        match_builders(&agents, tasks, now_ms)
    }

    pub(crate) fn agent_target(&self, workspace_id: &str, cx: &App) -> AgentTarget {
        match &self.client {
            Some(client) => choose_agent(&candidates(client.read(cx)), workspace_id),
            None => AgentTarget::NoAgent,
        }
    }

    /// Sends a prompt to the workspace's target agent, looked up again now, and resolves the
    /// agent's id and name.
    pub(crate) fn send(
        &mut self,
        workspace_id: &str,
        prompt: String,
        cx: &mut Context<Self>,
    ) -> Task<anyhow::Result<(String, String)>> {
        let (id, name) = match self.agent_target(workspace_id, cx) {
            AgentTarget::NoAgent => {
                return Task::ready(Err(anyhow::anyhow!("No agent in this workspace.")));
            }
            AgentTarget::Busy => return Task::ready(Err(anyhow::anyhow!("Agent busy."))),
            AgentTarget::Ready { id, name } => (id, name),
        };
        let Some(client) = self.client.clone() else {
            return Task::ready(Err(anyhow::anyhow!("No agent in this workspace.")));
        };
        let message = OutgoingMessage {
            text: prompt,
            images: vec![],
            attachments: vec![],
        };
        let sent = client.update(cx, |client, cx| {
            client.send_message(&id, message, TurnBehavior::Interrupt, cx)
        });
        cx.background_spawn(async move {
            sent.await?;
            Ok((id, name))
        })
    }

    fn connected(&self, cx: &App) -> bool {
        self.client
            .as_ref()
            .is_some_and(|client| client.read(cx).is_connected())
    }

    /// The pill's workspace: loads its board and overview when missing or older than 30 s.
    pub(crate) fn load_active(&mut self, cx: &mut Context<Self>) {
        if let Some(workspace_id) = self.ctx.active_workspace_id(cx) {
            self.load(&workspace_id, Want::Fresh, cx);
        }
    }

    fn load(&mut self, workspace_id: &str, want: Want, cx: &mut Context<Self>) {
        if !self.connected(cx) {
            return;
        }
        let key = workspace_id.to_string();
        let now = now_ms();
        let mut started = false;
        if let Some(generation) = self.boards.begin(&key, want, now) {
            started = true;
            let answer = self
                .ctx
                .rpc_as::<_, Board>(READ, &WorkspaceInput { workspace_id }, cx);
            let key = key.clone();
            cx.spawn(async move |this, cx| {
                let answer = answer.await;
                this.update(cx, |store, cx| {
                    if store.boards.finish(&key, generation, answer, now_ms()) {
                        store.changed(cx);
                    }
                })
                .ok();
            })
            .detach();
        }
        if let Some(generation) = self.overviews.begin(&key, want, now) {
            started = true;
            let answer =
                self.ctx
                    .rpc_as::<_, Overview>(OVERVIEW_READ, &WorkspaceInput { workspace_id }, cx);
            cx.spawn(async move |this, cx| {
                let answer = answer.await;
                this.update(cx, |store, cx| {
                    if store.overviews.finish(&key, generation, answer, now_ms()) {
                        store.update_live(cx);
                        store.changed(cx);
                    }
                })
                .ok();
            })
            .detach();
        }
        // Refresh shows "Loading…" while it runs; a first load already shows Loading.
        if started && want == Want::Force {
            self.changed(cx);
        }
    }

    fn load_project(&mut self, project_id: &str, want: Want, cx: &mut Context<Self>) {
        if !self.connected(cx) {
            return;
        }
        let key = project_id.to_string();
        let Some(generation) = self.project_boards.begin(&key, want, now_ms()) else {
            return;
        };
        let answer = self
            .ctx
            .rpc_as::<_, Board>(PROJECT_READ, &ProjectInput { project_id }, cx);
        cx.spawn(async move |this, cx| {
            let answer = answer.await;
            this.update(cx, |store, cx| {
                if store
                    .project_boards
                    .finish(&key, generation, answer, now_ms())
                {
                    store.changed(cx);
                }
            })
            .ok();
        })
        .detach();
        if want == Want::Force {
            self.changed(cx);
        }
    }

    /// After a connect or reconnect: loads every cached board and overview again, and the
    /// active workspace's.
    fn reload_all(&mut self, cx: &mut Context<Self>) {
        let workspaces: BTreeSet<String> = self
            .boards
            .keys()
            .into_iter()
            .chain(self.overviews.keys())
            .collect();
        for workspace_id in workspaces {
            self.load(&workspace_id, Want::Force, cx);
        }
        for project_id in self.project_boards.keys() {
            self.load_project(&project_id, Want::Force, cx);
        }
        self.load_active(cx);
    }

    /// Recomputes the live TASKs of each workspace with an overview, and redraws the pill when
    /// a set changed.
    fn update_live(&mut self, cx: &mut Context<Self>) {
        let now = now_ms();
        let mut live = HashMap::new();
        for workspace_id in self.overviews.keys() {
            if let Loaded::Ready(overview) = self.overviews.get(&workspace_id) {
                let found = self.builders(&workspace_id, &overview.tasks, now, cx);
                live.insert(workspace_id, live_task_ids(&found).into_iter().collect());
            }
        }
        if live != self.live {
            self.live = live;
            self.changed(cx);
        }
    }

    fn changed(&mut self, cx: &mut Context<Self>) {
        self.revision += 1;
        cx.notify();
        // The pill reads the store; this redraws it.
        self.ctx.notify(cx);
    }
}
