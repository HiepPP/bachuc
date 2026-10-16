//! The window, laid out as Electron's `workspace-screen.tsx`: the sidebar column on the left, the
//! selected workspace (header, tabs, panes) in the middle, and the Explorer on the right. One
//! `Workspace` view lives per opened workspace; the sidebar picks which one shows. The command
//! center and the settings screen open over it.

use crate::actions::{
    BindingSpec, JumpToTab, RunAction, SHELL_CONTEXT, binding_specs, rebind, settings_key_context,
};
use crate::chrome::{ExplorerResize, HANDLE_WIDTH, NoPreview};
use crate::dock;
use crate::new_agent::{Created, DraftPane};
use crate::panes::{
    AgentPane, Launch, LaunchItem, LazyAgentPane, NewTabPane, PaneFrame, launch_hints,
};
use crate::state::{
    Connection, LayoutPrefs, LogoFill, absolute_path, agent_icon, agent_key, agent_of_key,
    agent_title, all_diffs_key, badge_count, badge_label, command_items, commit_diff_key,
    commit_diff_title, diff_key, explorer_width, file_key, file_name, file_of_key, find_workspace,
    initial_host, next_mode, next_theme, notification_for, step_index, step_workspace,
    workspace_agents, workspace_order,
};
use bachuc_client::{Client, ClientEvent, DirectoryEvent, HostConfig, load_hosts};
use bachuc_command::{CommandCenter, CommandEvent, Shortcut, data_dir, load_shortcuts};
use bachuc_composer::ComposerEvent;
use bachuc_diagram::DiagramRenderer;
use bachuc_explorer::{
    DiffView, Explorer, ExplorerDiffEvent, ExplorerEvent, ExplorerTab, FileView,
};
use bachuc_icons::{IconName, provider_logo};
use bachuc_settings::{Settings, SettingsEvent};
use bachuc_sidebar::model::archive_prompt;
use bachuc_sidebar::{Sidebar, SidebarEvent};
use bachuc_terminal::{TerminalEvent, TerminalView};
use bachuc_theme::tokens;
use bachuc_timeline::TimelineEvent;
use bachuc_workspace::model::Rect;
use bachuc_workspace::{PaneEvent, PaneItem, SplitAxis, TabKind, TabMenuEvent, Workspace};
use gpui_kit::component::button::ButtonVariant;
use gpui_kit::component::dialog::DialogButtonProps;
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::theme::Theme;
use gpui_kit::component::{WindowExt as _, h_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::time::Duration;

/// The old client lives this long after a host switch, so a queued terminal kill reaches the
/// old daemon before its socket closes.
const RETIRE_AFTER: Duration = Duration::from_secs(2);

/// The workspace view shown while no workspace is selected, for example with zero projects.
const HOME: &str = "";

/// What a tab shows. Dropping an agent pane stops its timeline sync.
pub(crate) enum PaneContent {
    Agent(Entity<AgentPane>),
    /// An agent tab not shown yet; `materialize` makes its `AgentPane`.
    LazyAgent(Entity<LazyAgentPane>),
    Draft(Entity<DraftPane>),
    NewTab(Entity<NewTabPane>),
    Terminal(Entity<TerminalView>),
    File(Entity<FileView>),
    Diff(Entity<DiffView>),
}

impl PaneContent {
    fn view(&self) -> AnyView {
        match self {
            PaneContent::Agent(view) => view.clone().into(),
            PaneContent::LazyAgent(view) => view.clone().into(),
            PaneContent::Draft(view) => view.clone().into(),
            PaneContent::NewTab(view) => view.clone().into(),
            PaneContent::Terminal(view) => view.clone().into(),
            PaneContent::File(view) => view.clone().into(),
            PaneContent::Diff(view) => view.clone().into(),
        }
    }

    /// Whether the pane has its own focusable element with a role: a composer, a prompt, or a
    /// terminal. Other panes get a `PaneFrame`.
    fn owns_focus(&self) -> bool {
        matches!(
            self,
            PaneContent::Agent(_) | PaneContent::Draft(_) | PaneContent::Terminal(_)
        )
    }

    /// What the tab shows, which decides its context menu.
    fn kind(&self, identity: Option<&str>, cx: &App) -> TabKind {
        match self {
            PaneContent::Agent(_) | PaneContent::LazyAgent(_) => identity
                .and_then(agent_of_key)
                .map_or(TabKind::Other, |id| TabKind::Agent { id: id.to_string() }),
            // The daemon id arrives with `TerminalEvent::Opened`, which sets the kind again.
            PaneContent::Terminal(view) => TabKind::Terminal {
                id: view
                    .read(cx)
                    .terminal_id()
                    .map(str::to_string)
                    .unwrap_or_default(),
            },
            PaneContent::File(_) => TabKind::File,
            _ => TabKind::Other,
        }
    }

    /// Kills a terminal's daemon terminal; other panes need nothing.
    fn release(self, cx: &mut App) {
        if let PaneContent::Terminal(view) = self {
            view.update(cx, |terminal, cx| terminal.close(cx));
        }
    }
}

/// One tab. Keys are `pane:N` and stay when a tab changes kind (new tab, draft, agent), so a
/// replacement keeps the tab's place, as Electron's `replace` launch does. `identity` names what
/// the tab shows, so a second open finds it.
pub(crate) struct Pane {
    identity: Option<String>,
    content: PaneContent,
    /// The focus target when the content has none (`PaneContent::owns_focus`).
    frame: Option<Entity<PaneFrame>>,
    title: SharedString,
    /// The provider of an agent tab whose logo is set; `None` while the agent is not in the
    /// list yet, so `complete_logos` can set it once.
    provider: Option<String>,
    _subscriptions: Vec<Subscription>,
}

pub(crate) struct OpenWorkspace {
    cwd: Option<String>,
    view: Entity<Workspace>,
    panes: HashMap<String, Pane>,
    _subscriptions: Vec<Subscription>,
}

pub struct Shell {
    pub(crate) hosts: Vec<HostConfig>,
    pub(crate) host_index: usize,
    pub(crate) client: Entity<Client>,
    retired: Option<Entity<Client>>,
    diagrams: Entity<DiagramRenderer>,
    pub(crate) connection: Connection,
    badge: usize,
    logo_fill: LogoFill,
    shortcuts: Vec<Shortcut>,
    bindings: Vec<BindingSpec>,
    pub(crate) sidebar: Entity<Sidebar>,
    explorer: Option<Entity<Explorer>>,
    pub(crate) layout: LayoutPrefs,
    /// The bounds right of the sidebar, written by a canvas each frame.
    area: Rc<Cell<Bounds<Pixels>>>,
    workspaces: HashMap<String, OpenWorkspace>,
    pub(crate) selected: String,
    /// The agents of the last list, so an agent that leaves it closes its tab.
    known_agents: HashSet<String>,
    next_pane: u64,
    pub(crate) hosts_open: bool,
    command: Option<(Entity<CommandCenter>, Subscription)>,
    settings: Option<(Entity<Settings>, Subscription)>,
    pub(crate) dragging_window: bool,
    focus: FocusHandle,
    /// Subscriptions to the client and the views that hold it; a host switch replaces them.
    host_subscriptions: Vec<Subscription>,
    _subscriptions: Vec<Subscription>,
}

impl Shell {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let hosts = load_hosts();
        let preferred = std::env::var("BACHUC_HOST").ok();
        let host_index = initial_host(&hosts, preferred.as_deref());
        let client = Client::connect(hosts[host_index].clone(), cx);
        let diagrams = cx.new(|cx| DiagramRenderer::new(window, cx));
        let sidebar = cx.new(|cx| Sidebar::new(client.clone(), window, cx));
        let shortcuts = load_shortcuts();
        let bindings = binding_specs(&shortcuts);
        cx.bind_keys(bindings.iter().map(BindingSpec::binding));
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        Self::open_agent_on_notification(window, cx);
        let subscriptions = vec![
            // set_theme and the font and syntax setters write this global; the chrome reads
            // tokens. The logos follow only a change of their fill.
            cx.observe_global::<Theme>(|this, cx| {
                this.refresh_logos(cx);
                cx.notify()
            }),
        ];
        let logo_fill = LogoFill::new(tokens(cx).foreground);
        let mut shell = Shell {
            hosts,
            host_index,
            client,
            retired: None,
            diagrams,
            connection: Connection::Connecting,
            badge: 0,
            logo_fill,
            shortcuts,
            bindings,
            sidebar,
            explorer: None,
            layout: LayoutPrefs::load_from(&data_dir()),
            area: Rc::default(),
            workspaces: HashMap::new(),
            selected: HOME.to_string(),
            known_agents: HashSet::new(),
            next_pane: 0,
            hosts_open: false,
            command: None,
            settings: None,
            dragging_window: false,
            focus,
            host_subscriptions: Vec::new(),
            _subscriptions: subscriptions,
        };
        shell.subscribe_host(window, cx);
        shell.ensure_home(window, cx);
        shell
    }

    fn subscribe_host(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.host_subscriptions = vec![
            cx.subscribe_in(&self.client, window, Self::on_client_event),
            cx.subscribe_in(
                &self.client,
                window,
                |this: &mut Shell, _, event: &DirectoryEvent, window, cx| {
                    if *event == DirectoryEvent::ProjectsChanged {
                        this.on_projects_changed(window, cx);
                    }
                },
            ),
            cx.subscribe_in(
                &self.sidebar,
                window,
                |this: &mut Shell, _, event: &SidebarEvent, window, cx| match event {
                    SidebarEvent::OpenWorkspace { workspace_id } => {
                        this.open_workspace(workspace_id.clone(), window, cx)
                    }
                    SidebarEvent::OpenSettings => this.open_settings(window, cx),
                    SidebarEvent::OpenCommandCenter => this.open_command_center(window, cx),
                },
            ),
        ];
    }

    /// A click on a system notification brings the window up and opens the agent.
    fn open_agent_on_notification(window: &Window, cx: &mut Context<Self>) {
        let shell = cx.weak_entity();
        let handle = window.window_handle();
        cx.on_system_notification_response(move |response, cx| {
            let Some(agent_id) = crate::state::agent_from_tag(&response.tag) else {
                return;
            };
            let agent_id = agent_id.to_string();
            cx.activate(true);
            let shell = shell.clone();
            let _ = handle.update(cx, move |_, window, cx| {
                let _ = shell.update(cx, |shell, cx| shell.open_agent(&agent_id, window, cx));
            });
        });
    }

    fn on_client_event(
        &mut self,
        client: &Entity<Client>,
        event: &ClientEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Timeline views follow their own changes; the shell has nothing to redraw.
        if matches!(event, ClientEvent::TimelineChanged { .. }) {
            return;
        }
        let before = self.connection.clone();
        self.connection.apply(event);
        if let Some(note) =
            notification_for(event, client.read(cx).agents(), window.is_window_active())
        {
            cx.show_system_notification(SystemNotification {
                tag: note.tag.into(),
                title: note.title.into(),
                body: note.body.into(),
                actions: Vec::new(),
            });
        }
        match event {
            ClientEvent::Connected => {
                self.known_agents = self.agent_ids(cx);
            }
            ClientEvent::AgentsChanged => self.on_agents_changed(window, cx),
            _ => {}
        }
        self.refresh_badge(cx);
        if before != self.connection {
            cx.notify();
        }
    }

    fn agent_ids(&self, cx: &App) -> HashSet<String> {
        self.client
            .read(cx)
            .agents()
            .iter()
            .map(|agent| agent.id.clone())
            .collect()
    }

    /// Updates agent tab titles and closes the tabs of agents that left the list (archived).
    fn on_agents_changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let current = self.agent_ids(cx);
        if self.connection == Connection::Connected {
            let removed: Vec<String> = self.known_agents.difference(&current).cloned().collect();
            for agent_id in removed {
                self.close_identity(&agent_key(&agent_id), window, cx);
            }
            self.known_agents = current;
        }
        let titles: Vec<(String, String, SharedString)> = self
            .workspaces
            .iter()
            .flat_map(|(workspace_id, open)| {
                open.panes.iter().filter_map(|(key, pane)| {
                    let agent_id = agent_of_key(pane.identity.as_deref()?)?;
                    let title: SharedString = self.agent_tab_title(agent_id, cx).into();
                    (title != pane.title).then(|| (workspace_id.clone(), key.clone(), title))
                })
            })
            .collect();
        for (workspace_id, key, title) in titles {
            self.retitle(&workspace_id, &key, title, cx);
        }
        self.complete_logos(cx);
    }

    fn agent_tab_title(&self, agent_id: &str, cx: &App) -> String {
        self.client
            .read(cx)
            .agents()
            .iter()
            .find(|agent| agent.id == agent_id)
            .map(agent_title)
            .unwrap_or_else(|| "Agent".to_string())
    }

    fn retitle(&mut self, workspace_id: &str, key: &str, title: SharedString, cx: &mut App) {
        let Some(open) = self.workspaces.get_mut(workspace_id) else {
            return;
        };
        let Some(pane) = open.panes.get_mut(key) else {
            return;
        };
        pane.title = title.clone();
        let view = open.view.clone();
        view.update(cx, |workspace, cx| {
            if let Some(mut item) = workspace.layout().item(key).cloned() {
                item.title = title;
                workspace.update(item, cx);
            }
        });
    }

    fn refresh_badge(&mut self, cx: &mut Context<Self>) {
        let count = badge_count(self.client.read(cx).agents());
        if count != self.badge {
            self.badge = count;
            dock::set_badge(badge_label(count).as_deref());
        }
    }

    /// A workspace the tree no longer has (archived) drops its view and its tabs.
    fn on_projects_changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let gone: Vec<String> = {
            let projects = self.client.read(cx).projects();
            self.workspaces
                .keys()
                .filter(|id| id.as_str() != HOME && find_workspace(projects, id).is_none())
                .cloned()
                .collect()
        };
        // An empty tree during a reconnect is not an archive.
        if gone.is_empty() || self.connection != Connection::Connected {
            cx.notify();
            return;
        }
        for id in gone {
            self.drop_workspace(&id, cx);
            if self.selected == id {
                self.select(HOME.to_string(), window, cx);
            }
        }
        cx.notify();
    }

    fn drop_workspace(&mut self, id: &str, cx: &mut App) {
        if let Some(open) = self.workspaces.remove(id) {
            for (_, pane) in open.panes {
                pane.content.release(cx);
            }
        }
    }

    // ----- host -----

    pub(crate) fn switch_host(&mut self, next: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.hosts_open = false;
        if next == self.host_index || next >= self.hosts.len() {
            cx.notify();
            return;
        }
        self.reconnect(next, window, cx);
    }

    /// Connects to `index` with a new client, closing every pane of the old host, as before.
    pub(crate) fn reconnect(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.host_index = index.min(self.hosts.len() - 1);
        let ids: Vec<String> = self.workspaces.keys().cloned().collect();
        for id in ids {
            self.drop_workspace(&id, cx);
        }
        self.selected = HOME.to_string();
        self.known_agents.clear();
        self.command = None;
        self.connection = Connection::Connecting;
        let client = Client::connect(self.hosts[self.host_index].clone(), cx);
        self.retired = Some(std::mem::replace(&mut self.client, client.clone()));
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(RETIRE_AFTER).await;
            let _ = this.update(cx, |shell, _| shell.retired = None);
        })
        .detach();
        self.sidebar = cx.new(|cx| Sidebar::new(client.clone(), window, cx));
        if let Some(explorer) = self.explorer.take() {
            explorer.update(cx, |explorer, cx| explorer.set_visible(false, cx));
        }
        // The settings screen holds the client too; a fresh one talks to the new host only.
        if self.settings.is_some() {
            self.settings = None;
            self.open_settings(window, cx);
        }
        self.subscribe_host(window, cx);
        self.ensure_home(window, cx);
        self.refresh_badge(cx);
        cx.notify();
    }

    /// `SettingsEvent::HostsChanged`: keeps the connection when its host is unchanged, else
    /// reconnects to the start host of the new list.
    fn reload_hosts(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let current = self.hosts[self.host_index].clone();
        self.hosts = load_hosts();
        match self.hosts.iter().position(|host| *host == current) {
            Some(index) => {
                self.host_index = index;
                cx.notify();
            }
            None => {
                let index = initial_host(&self.hosts, None);
                self.reconnect(index, window, cx);
            }
        }
    }

    // ----- workspaces -----

    pub(crate) fn selected_cwd(&self) -> Option<&str> {
        self.workspaces.get(&self.selected)?.cwd.as_deref()
    }

    fn ensure_home(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.workspaces.contains_key(HOME) {
            self.create_workspace(HOME.to_string(), None, window, cx);
            self.open_new_tab(HOME, None, window, cx);
        }
    }

    fn create_workspace(
        &mut self,
        id: String,
        cwd: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let view = cx.new(|cx| Workspace::new(window, cx));
        let workspace_id = id.clone();
        let menu_workspace_id = id.clone();
        let subscriptions = vec![
            cx.subscribe_in(
                &view,
                window,
                move |this: &mut Shell, _, event: &PaneEvent, window, cx| {
                    this.on_pane_event(&workspace_id, event, window, cx)
                },
            ),
            cx.subscribe_in(
                &view,
                window,
                move |this: &mut Shell, _, event: &TabMenuEvent, window, cx| {
                    this.on_tab_menu(&menu_workspace_id, event, window, cx)
                },
            ),
        ];
        self.workspaces.insert(
            id,
            OpenWorkspace {
                cwd,
                view,
                panes: HashMap::new(),
                _subscriptions: subscriptions,
            },
        );
    }

    /// Shows a workspace. The first open makes a tab for each of its agents, as
    /// `workspace-screen.tsx` does, or a new tab when it has none; a later open adds tabs for
    /// agents that started since.
    pub(crate) fn open_workspace(
        &mut self,
        id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let found = {
            let client = self.client.read(cx);
            find_workspace(client.projects(), &id).map(|(_, workspace)| {
                let agents: Vec<String> = workspace_agents(workspace, client.agents())
                    .into_iter()
                    .map(str::to_string)
                    .collect();
                (workspace.cwd.clone(), agents)
            })
        };
        let Some((cwd, agents)) = found else {
            return;
        };
        let first_open = !self.workspaces.contains_key(&id);
        if first_open {
            self.create_workspace(id.clone(), Some(cwd), window, cx);
        }
        let active = self.workspace_active_key(&id, cx);
        let mut opened = false;
        for agent_id in &agents {
            if self.find_identity(&id, &agent_key(agent_id)).is_none() {
                self.open_lazy_agent(&id, agent_id, window, cx);
                opened = true;
            }
        }
        if first_open {
            if agents.is_empty() {
                self.open_new_tab(&id, None, window, cx);
            } else {
                self.with_workspace(
                    &id,
                    cx,
                    |workspace, window, cx| workspace.activate_index(0, window, cx),
                    window,
                );
            }
        } else if opened && let Some(key) = active {
            self.activate_key(&id, &key, window, cx);
        }
        self.select(id, window, cx);
    }

    fn select(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        self.selected = id.clone();
        let selected = (id != HOME).then_some(id);
        self.sidebar.update(cx, |sidebar, cx| {
            sidebar.set_selected_workspace(selected, cx)
        });
        self.sync_explorer(window, cx);
        if let Some(key) = self.active_key(cx) {
            let selected = self.selected.clone();
            self.materialize(&selected, &key, window, cx);
            self.focus_pane(&selected, &key, window, cx);
        } else {
            window.focus(&self.focus, cx);
        }
        cx.notify();
    }

    /// Points the Explorer at the selected workspace and shows or hides it. The Explorer is made
    /// on its first show; a hidden one stops watching the checkout.
    fn sync_explorer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let cwd = self.selected_cwd().map(str::to_string);
        let visible = self.layout.explorer_open && cwd.is_some();
        match (&self.explorer, cwd) {
            (Some(explorer), Some(cwd)) => explorer.update(cx, |explorer, cx| {
                explorer.set_cwd(cwd, cx);
                explorer.set_visible(visible, cx);
            }),
            (Some(explorer), None) => {
                explorer.update(cx, |explorer, cx| explorer.set_visible(false, cx))
            }
            (None, Some(cwd)) if visible => {
                let client = self.client.clone();
                let explorer = cx.new(|cx| Explorer::new(client, cwd, window, cx));
                let subscription = cx.subscribe_in(
                    &explorer,
                    window,
                    |this: &mut Shell, _, event: &ExplorerEvent, window, cx| match event {
                        ExplorerEvent::OpenFile { cwd, path } => {
                            this.open_file(cwd, path, None, window, cx)
                        }
                        ExplorerEvent::OpenDiff { cwd, path } => {
                            this.open_diff(cwd, path, window, cx)
                        }
                    },
                );
                let diff_subscription = cx.subscribe_in(
                    &explorer,
                    window,
                    |this: &mut Shell, _, event: &ExplorerDiffEvent, window, cx| match event {
                        ExplorerDiffEvent::OpenAllDiffs { cwd, focus_path } => {
                            this.open_all_diffs(cwd, focus_path.clone(), window, cx)
                        }
                        ExplorerDiffEvent::OpenCommitDiff { cwd, sha, path } => {
                            this.open_commit_diff(cwd, sha, path, window, cx)
                        }
                    },
                );
                self.host_subscriptions.push(subscription);
                self.host_subscriptions.push(diff_subscription);
                self.explorer = Some(explorer);
            }
            (None, _) => {}
        }
    }

    fn with_workspace(
        &mut self,
        id: &str,
        cx: &mut Context<Self>,
        f: impl FnOnce(&mut Workspace, &mut Window, &mut Context<Workspace>),
        window: &mut Window,
    ) {
        if let Some(open) = self.workspaces.get(id) {
            let view = open.view.clone();
            view.update(cx, |workspace, cx| f(workspace, window, cx));
        }
    }

    fn workspace_active_key(&self, id: &str, cx: &App) -> Option<String> {
        let open = self.workspaces.get(id)?;
        open.view.read(cx).active_key().map(str::to_string)
    }

    /// The active key of the focused group of the selected workspace.
    pub(crate) fn active_key(&self, cx: &App) -> Option<String> {
        self.workspace_active_key(&self.selected, cx)
    }

    pub(crate) fn pane_identity(&self, key: &str) -> Option<String> {
        self.workspaces
            .get(&self.selected)?
            .panes
            .get(key)?
            .identity
            .clone()
    }

    fn find_identity(&self, workspace_id: &str, identity: &str) -> Option<String> {
        self.workspaces
            .get(workspace_id)?
            .panes
            .iter()
            .find(|(_, pane)| pane.identity.as_deref() == Some(identity))
            .map(|(key, _)| key.clone())
    }

    /// Activates an open tab and focuses its group.
    fn activate_key(
        &mut self,
        workspace_id: &str,
        key: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let key = key.to_string();
        self.with_workspace(
            workspace_id,
            cx,
            move |workspace, window, cx| {
                if let Some(item) = workspace.layout().item(&key).cloned() {
                    workspace.open(item, window, cx);
                }
            },
            window,
        );
    }

    fn close_identity(&mut self, identity: &str, window: &mut Window, cx: &mut Context<Self>) {
        let found: Vec<(String, String)> = self
            .workspaces
            .iter()
            .flat_map(|(id, open)| {
                open.panes
                    .iter()
                    .filter(|(_, pane)| pane.identity.as_deref() == Some(identity))
                    .map(|(key, _)| (id.clone(), key.clone()))
            })
            .collect();
        for (workspace_id, key) in found {
            self.with_workspace(
                &workspace_id,
                cx,
                |workspace, window, cx| workspace.close(&key, window, cx),
                window,
            );
        }
    }

    fn on_pane_event(
        &mut self,
        workspace_id: &str,
        event: &PaneEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            PaneEvent::Closed { key } => {
                if let Some(pane) = self
                    .workspaces
                    .get_mut(workspace_id)
                    .and_then(|open| open.panes.remove(key))
                {
                    pane.content.release(cx);
                }
                cx.notify();
            }
            PaneEvent::Activated { key } => {
                // Events arrive after a batch of opens; only the tab still active loads.
                if self.workspace_active_key(workspace_id, cx).as_deref() == Some(key.as_str()) {
                    self.materialize(workspace_id, key, window, cx);
                }
                if workspace_id == self.selected {
                    self.focus_pane(workspace_id, key, window, cx);
                }
                cx.notify();
            }
            PaneEvent::NewTab => self.open_new_tab(workspace_id, None, window, cx),
        }
    }

    fn focus_pane(
        &mut self,
        workspace_id: &str,
        key: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(pane) = self
            .workspaces
            .get(workspace_id)
            .and_then(|open| open.panes.get(key))
        else {
            return;
        };
        match &pane.content {
            PaneContent::Agent(view) => view.update(cx, |pane, cx| pane.focus(window, cx)),
            PaneContent::Draft(view) => view.update(cx, |pane, cx| pane.focus(window, cx)),
            // A terminal focuses itself when it opens and on a click.
            PaneContent::Terminal(_) => {}
            // Inside the workspace element, so the workspace shortcuts still reach it.
            _ => {
                if let Some(frame) = &pane.frame {
                    frame.update(cx, |frame, cx| frame.focus(window, cx));
                }
            }
        }
    }

    // ----- panes -----

    fn next_key(&mut self) -> String {
        self.next_pane += 1;
        format!("pane:{}", self.next_pane)
    }

    /// Opens `content` in a workspace: in place of `replace` when given, else as a new tab of
    /// the focused group.
    #[allow(clippy::too_many_arguments)]
    fn put_pane(
        &mut self,
        workspace_id: &str,
        key: String,
        identity: Option<String>,
        title: SharedString,
        icon: IconName,
        content: PaneContent,
        subscriptions: Vec<Subscription>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let kind = content.kind(identity.as_deref(), cx);
        let provider = identity
            .as_deref()
            .and_then(agent_of_key)
            .and_then(|agent_id| self.agent_provider(agent_id, cx));
        let logo = provider
            .as_deref()
            .and_then(|provider| provider_logo(provider, tokens(cx).foreground));
        let Some(open) = self.workspaces.get_mut(workspace_id) else {
            content.release(cx);
            return;
        };
        let frame = (!content.owns_focus())
            .then(|| cx.new(|cx| PaneFrame::new(title.clone(), content.view(), cx)));
        let item = PaneItem {
            key: key.clone(),
            title: title.clone(),
            icon,
            modified: false,
            view: frame.clone().map_or_else(|| content.view(), AnyView::from),
        };
        let old = open.panes.insert(
            key.clone(),
            Pane {
                identity,
                content,
                frame,
                title,
                provider,
                _subscriptions: subscriptions,
            },
        );
        let view = open.view.clone();
        if let Some(old) = old {
            old.content.release(cx);
        }
        view.update(cx, |workspace, cx| {
            workspace.open(item, window, cx);
            workspace.set_item_kind(&key, kind, cx);
            workspace.set_item_logo(&key, logo, cx);
        });
        cx.notify();
    }

    fn agent_provider(&self, agent_id: &str, cx: &App) -> Option<String> {
        self.client
            .read(cx)
            .agents()
            .iter()
            .find(|agent| agent.id == agent_id)
            .map(|agent| agent.provider.clone())
    }

    /// Rebuilds the logo of each agent tab when the theme changed its fill.
    fn refresh_logos(&mut self, cx: &mut Context<Self>) {
        let fill = tokens(cx).foreground;
        if !self.logo_fill.changed(fill) {
            return;
        }
        for open in self.workspaces.values() {
            for (key, pane) in &open.panes {
                let Some(provider) = &pane.provider else {
                    continue;
                };
                let logo = provider_logo(provider, fill);
                open.view
                    .update(cx, |workspace, cx| workspace.set_item_logo(key, logo, cx));
            }
        }
    }

    /// Sets the logo of an agent tab that opened before its agent was in the list.
    fn complete_logos(&mut self, cx: &mut Context<Self>) {
        let fill = tokens(cx).foreground;
        let pending: Vec<(String, String, String)> = self
            .workspaces
            .iter()
            .flat_map(|(workspace_id, open)| {
                open.panes.iter().filter_map(|(key, pane)| {
                    if pane.provider.is_some() {
                        return None;
                    }
                    let agent_id = agent_of_key(pane.identity.as_deref()?)?;
                    let provider = self.agent_provider(agent_id, cx)?;
                    Some((workspace_id.clone(), key.clone(), provider))
                })
            })
            .collect();
        for (workspace_id, key, provider) in pending {
            let Some(open) = self.workspaces.get_mut(&workspace_id) else {
                continue;
            };
            if let Some(pane) = open.panes.get_mut(&key) {
                pane.provider = Some(provider.clone());
            }
            let logo = provider_logo(&provider, fill);
            open.view
                .update(cx, |workspace, cx| workspace.set_item_logo(&key, logo, cx));
        }
    }

    fn pane(&self, workspace_id: &str, key: &str) -> Option<&Pane> {
        self.workspaces.get(workspace_id)?.panes.get(key)
    }

    fn set_pane_kind(&mut self, workspace_id: &str, key: &str, kind: TabKind, cx: &mut App) {
        if let Some(open) = self.workspaces.get(workspace_id) {
            open.view
                .update(cx, |workspace, cx| workspace.set_item_kind(key, kind, cx));
        }
    }

    /// A tab context menu item that the workspace leaves to the app.
    fn on_tab_menu(
        &mut self,
        workspace_id: &str,
        event: &TabMenuEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            TabMenuEvent::CopyAgentId { key } => {
                let agent_id = self
                    .pane(workspace_id, key)
                    .and_then(|pane| agent_of_key(pane.identity.as_deref()?))
                    .map(str::to_string);
                if let Some(agent_id) = agent_id {
                    copy_text(agent_id, "Agent id copied", window, cx);
                }
            }
            TabMenuEvent::CopyTerminalId { key } => {
                let id = match self.pane(workspace_id, key).map(|pane| &pane.content) {
                    Some(PaneContent::Terminal(view)) => {
                        view.read(cx).terminal_id().map(str::to_string)
                    }
                    _ => None,
                };
                match id {
                    Some(id) => copy_text(id, "Terminal id copied", window, cx),
                    None => window.push_notification("The terminal is not open yet", cx),
                }
            }
            TabMenuEvent::CopyFilePath { key } => {
                let path = self
                    .pane(workspace_id, key)
                    .and_then(|pane| file_of_key(pane.identity.as_deref()?))
                    .map(str::to_string);
                if let Some(path) = path {
                    copy_text(path, "File path copied", window, cx);
                }
            }
            TabMenuEvent::Rename { key } => {
                let agent_id = self
                    .pane(workspace_id, key)
                    .and_then(|pane| agent_of_key(pane.identity.as_deref()?))
                    .map(str::to_string);
                match agent_id {
                    Some(agent_id) => self.rename_agent(agent_id, window, cx),
                    // The client has no terminal rename request.
                    None => window.push_notification("This tab cannot be renamed yet", cx),
                }
            }
            // The client holds no provider session id and has no refresh request.
            TabMenuEvent::CopyResumeCommand { .. } => {
                window.push_notification("The resume command is not available yet", cx)
            }
            TabMenuEvent::ReloadAgent { .. } => {
                window.push_notification("Reloading an agent is not available yet", cx)
            }
        }
    }

    fn open_new_tab(
        &mut self,
        workspace_id: &str,
        replace: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let key = replace.unwrap_or_else(|| self.next_key());
        let has_workspace = workspace_id != HOME;
        let hints = launch_hints(&self.shortcuts);
        let pane_key = key.clone();
        let view = cx.new(|cx| NewTabPane::new(pane_key, has_workspace, hints, cx));
        let workspace = workspace_id.to_string();
        let subscription = cx.subscribe_in(
            &view,
            window,
            move |this: &mut Shell, _, launch: &Launch, window, cx| {
                this.launch(&workspace, launch, window, cx)
            },
        );
        self.put_pane(
            workspace_id,
            key,
            None,
            "New tab".into(),
            IconName::Plus,
            PaneContent::NewTab(view),
            vec![subscription],
            window,
            cx,
        );
    }

    fn launch(
        &mut self,
        workspace_id: &str,
        launch: &Launch,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let key = Some(launch.key.clone());
        match launch.item {
            LaunchItem::Agent => self.open_draft(workspace_id, key, window, cx),
            LaunchItem::Terminal => self.open_terminal(workspace_id, key, window, cx),
            LaunchItem::Changes => self.show_explorer(ExplorerTab::Changes, window, cx),
            LaunchItem::Files => self.show_explorer(ExplorerTab::Files, window, cx),
        }
    }

    fn open_draft(
        &mut self,
        workspace_id: &str,
        replace: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let key = replace.unwrap_or_else(|| self.next_key());
        let cwd = self
            .workspaces
            .get(workspace_id)
            .and_then(|open| open.cwd.clone());
        let client = self.client.clone();
        let pane_key = key.clone();
        let view = cx.new(|cx| DraftPane::new(client, pane_key, cwd, window, cx));
        let workspace = workspace_id.to_string();
        let subscription = cx.subscribe_in(
            &view,
            window,
            move |this: &mut Shell, _, created: &Created, window, cx| {
                this.open_agent_pane(
                    &workspace,
                    Some(created.key.clone()),
                    &created.agent_id,
                    window,
                    cx,
                );
                this.known_agents.insert(created.agent_id.clone());
            },
        );
        self.put_pane(
            workspace_id,
            key.clone(),
            None,
            "New agent".into(),
            IconName::SquarePen,
            PaneContent::Draft(view.clone()),
            vec![subscription],
            window,
            cx,
        );
        view.update(cx, |pane, cx| pane.focus(window, cx));
    }

    fn open_agent_pane(
        &mut self,
        workspace_id: &str,
        replace: Option<String>,
        agent_id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let key = replace.unwrap_or_else(|| self.next_key());
        let client = self.client.clone();
        let diagrams = self.diagrams.clone();
        let id = agent_id.to_string();
        let view = cx.new(|cx| AgentPane::new(client, id, diagrams, window, cx));
        let (timeline, composer) = {
            let pane = view.read(cx);
            (pane.timeline.clone(), pane.composer.clone())
        };
        let workspace = workspace_id.to_string();
        let subscriptions = vec![
            cx.subscribe_in(
                &timeline,
                window,
                |this: &mut Shell, _, event: &TimelineEvent, window, cx| {
                    let TimelineEvent::OpenFile { cwd, path, line } = event;
                    this.open_file(cwd, path, *line, window, cx);
                },
            ),
            cx.subscribe_in(&composer, window, {
                let key = key.clone();
                move |this: &mut Shell, _, event: &ComposerEvent, window, cx| {
                    if *event == ComposerEvent::ReplaceWithDraft {
                        this.open_draft(&workspace, Some(key.clone()), window, cx);
                    }
                }
            }),
        ];
        let title = self.agent_tab_title(agent_id, cx);
        let icon = agent_icon(self.agent_provider(agent_id, cx).as_deref());
        self.put_pane(
            workspace_id,
            key,
            Some(agent_key(agent_id)),
            title.into(),
            icon,
            PaneContent::Agent(view.clone()),
            subscriptions,
            window,
            cx,
        );
        view.update(cx, |pane, cx| pane.focus(window, cx));
    }

    /// An agent tab that loads when it first shows.
    fn open_lazy_agent(
        &mut self,
        workspace_id: &str,
        agent_id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let key = self.next_key();
        let title = self.agent_tab_title(agent_id, cx);
        let view = cx.new(|_| LazyAgentPane);
        let icon = agent_icon(self.agent_provider(agent_id, cx).as_deref());
        self.put_pane(
            workspace_id,
            key,
            Some(agent_key(agent_id)),
            title.into(),
            icon,
            PaneContent::LazyAgent(view),
            Vec::new(),
            window,
            cx,
        );
    }

    /// Gives a lazy agent tab its timeline and composer, in place.
    fn materialize(
        &mut self,
        workspace_id: &str,
        key: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let agent_id = self
            .workspaces
            .get(workspace_id)
            .and_then(|open| open.panes.get(key))
            .filter(|pane| matches!(pane.content, PaneContent::LazyAgent(_)))
            .and_then(|pane| agent_of_key(pane.identity.as_deref()?).map(str::to_string));
        if let Some(agent_id) = agent_id {
            self.open_agent_pane(workspace_id, Some(key.to_string()), &agent_id, window, cx);
        }
    }

    fn open_terminal(
        &mut self,
        workspace_id: &str,
        replace: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(cwd) = self
            .workspaces
            .get(workspace_id)
            .and_then(|open| open.cwd.clone())
        else {
            return;
        };
        let key = replace.unwrap_or_else(|| self.next_key());
        let client = self.client.clone();
        let title = format!("Terminal {}", file_name(&cwd));
        let name = title.clone();
        let view = cx.new(|cx| {
            let mut terminal = TerminalView::new(client, cwd, window, cx);
            terminal.set_name(name, cx);
            terminal
        });
        let workspace = workspace_id.to_string();
        let pane_key = key.clone();
        let subscription = cx.subscribe_in(
            &view,
            window,
            move |this: &mut Shell, _, event: &TerminalEvent, _, cx| {
                if let TerminalEvent::Opened { id } = event {
                    this.set_pane_kind(
                        &workspace,
                        &pane_key,
                        TabKind::Terminal { id: id.clone() },
                        cx,
                    );
                }
            },
        );
        self.put_pane(
            workspace_id,
            key,
            None,
            title.into(),
            IconName::SquareTerminal,
            PaneContent::Terminal(view),
            vec![subscription],
            window,
            cx,
        );
    }

    /// Opens a file in the selected workspace, or shows its open tab and scrolls to `line`.
    fn open_file(
        &mut self,
        cwd: &str,
        path: &str,
        line: Option<u32>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let workspace_id = self.selected.clone();
        let identity = file_key(cwd, path);
        if let Some(key) = self.find_identity(&workspace_id, &identity) {
            if let Some(line) = line
                && let Some(PaneContent::File(view)) = self
                    .workspaces
                    .get(&workspace_id)
                    .and_then(|open| open.panes.get(&key))
                    .map(|pane| &pane.content)
            {
                view.update(cx, |file, cx| file.scroll_to_line(line, cx));
            }
            self.activate_key(&workspace_id, &key, window, cx);
            return;
        }
        let key = self.next_key();
        let client = self.client.clone();
        let (cwd, path) = (cwd.to_string(), path.to_string());
        let title = file_name(&absolute_path(&cwd, &path));
        let view = cx.new(|cx| FileView::new(client, cwd, path, line, window, cx));
        self.put_pane(
            &workspace_id,
            key,
            Some(identity),
            title.into(),
            IconName::FileText,
            PaneContent::File(view),
            Vec::new(),
            window,
            cx,
        );
    }

    fn open_diff(&mut self, cwd: &str, path: &str, window: &mut Window, cx: &mut Context<Self>) {
        let (cwd, path) = (cwd.to_string(), path.to_string());
        let title = file_name(&absolute_path(&cwd, &path));
        self.open_diff_tab(
            diff_key(&cwd, &path),
            title,
            IconName::FileDiff,
            move |client, window, cx| DiffView::new(client, cwd, path, window, cx),
            window,
            cx,
        );
    }

    /// Opens the all-files diff of a folder, or shows its open tab.
    fn open_all_diffs(
        &mut self,
        cwd: &str,
        focus_path: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let cwd = cwd.to_string();
        self.open_diff_tab(
            all_diffs_key(&cwd),
            "Changes".to_string(),
            IconName::FileDiff,
            move |client, window, cx| DiffView::new_all(client, cwd, focus_path, window, cx),
            window,
            cx,
        );
    }

    /// Opens one file of a commit, or shows its open tab.
    fn open_commit_diff(
        &mut self,
        cwd: &str,
        sha: &str,
        path: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let identity = commit_diff_key(cwd, sha, path);
        let title = commit_diff_title(sha, path);
        let (cwd, sha, path) = (cwd.to_string(), sha.to_string(), path.to_string());
        self.open_diff_tab(
            identity,
            title,
            IconName::GitCommitHorizontal,
            move |client, window, cx| DiffView::new_commit(client, cwd, sha, path, window, cx),
            window,
            cx,
        );
    }

    fn open_diff_tab(
        &mut self,
        identity: String,
        title: String,
        icon: IconName,
        build: impl FnOnce(Entity<Client>, &mut Window, &mut Context<DiffView>) -> DiffView,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let workspace_id = self.selected.clone();
        if let Some(key) = self.find_identity(&workspace_id, &identity) {
            self.activate_key(&workspace_id, &key, window, cx);
            return;
        }
        let key = self.next_key();
        let client = self.client.clone();
        let view = cx.new(|cx| build(client, window, cx));
        self.put_pane(
            &workspace_id,
            key,
            Some(identity),
            title.into(),
            icon,
            PaneContent::Diff(view),
            Vec::new(),
            window,
            cx,
        );
    }

    /// Opens an agent's tab in its workspace, from the command center or a notification.
    pub(crate) fn open_agent(
        &mut self,
        agent_id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let workspace_id = {
            let client = self.client.read(cx);
            client
                .projects()
                .iter()
                .flat_map(|project| &project.workspaces)
                .find(|workspace| workspace.agent_ids.iter().any(|id| id == agent_id))
                .map(|workspace| workspace.id.clone())
        };
        let Some(workspace_id) = workspace_id else {
            return;
        };
        self.open_workspace(workspace_id.clone(), window, cx);
        match self.find_identity(&workspace_id, &agent_key(agent_id)) {
            Some(key) => self.activate_key(&workspace_id, &key, window, cx),
            None => self.open_agent_pane(&workspace_id, None, agent_id, window, cx),
        }
    }

    // ----- agent actions -----

    fn active_agent(&self, cx: &App) -> Option<String> {
        let identity = self.pane_identity(&self.active_key(cx)?)?;
        agent_of_key(&identity).map(str::to_string)
    }

    pub(crate) fn rename_agent(
        &mut self,
        agent_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let current = self.agent_tab_title(&agent_id, cx);
        let input = cx.new(|cx| InputState::new(window, cx).default_value(current));
        let client = self.client.clone();
        // The builder runs again whenever the dialog layer is rebuilt; focus once.
        let focus_once = Cell::new(true);
        window.open_dialog(cx, move |dialog, window, cx| {
            if focus_once.replace(false) {
                input.update(cx, |input, cx| input.focus(window, cx));
            }
            let client = client.clone();
            let agent_id = agent_id.clone();
            let field = input.clone();
            dialog
                .title("Rename agent")
                .w(px(420.))
                .child(Input::new(&input).aria_label("Agent name"))
                .button_props(
                    DialogButtonProps::default()
                        .ok_text("Rename")
                        .show_cancel(true)
                        .on_ok(move |_, _, cx| {
                            let title = field.read(cx).value().trim().to_string();
                            if !title.is_empty() {
                                client.update(cx, |client, cx| {
                                    client.rename_agent(&agent_id, &title, cx)
                                });
                            }
                            true
                        }),
                )
        });
    }

    pub(crate) fn archive_agent(&mut self, agent_id: &str, cx: &mut Context<Self>) {
        self.client
            .update(cx, |client, cx| client.archive_agent(agent_id, cx));
    }

    pub(crate) fn copy_workspace_path(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(cwd) = self.selected_cwd().map(str::to_string) {
            cx.write_to_clipboard(ClipboardItem::new_string(cwd));
            window.push_notification("Path copied", cx);
        }
    }

    /// `workspace.archive`: the sidebar's archive flow, with a confirm when files changed.
    fn archive_workspace(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((id, name, cwd)) = ({
            let client = self.client.read(cx);
            find_workspace(client.projects(), &self.selected)
                .map(|(_, w)| (w.id.clone(), w.name.clone(), w.cwd.clone()))
        }) else {
            return;
        };
        let status = self
            .client
            .update(cx, |client, cx| client.checkout_status(&cwd, cx));
        let client = self.client.clone();
        cx.spawn_in(window, async move |_, cx| {
            let files = status.await.map(|status| status.files).unwrap_or_default();
            let _ = cx.update(|window, cx| match archive_prompt(&name, &files) {
                None => client.update(cx, |client, cx| client.archive_workspace(&id, cx)),
                Some(prompt) => {
                    window.open_alert_dialog(cx, move |dialog, _, _| {
                        let client = client.clone();
                        let id = id.clone();
                        dialog
                            .title(prompt.title.clone())
                            .description(prompt.message.clone())
                            .confirm()
                            .ok_text("Archive")
                            .ok_variant(ButtonVariant::Danger)
                            .cancel_text("Cancel")
                            .on_ok(move |_, _, cx| {
                                client.update(cx, |client, cx| client.archive_workspace(&id, cx));
                                true
                            })
                    });
                }
            });
        })
        .detach();
    }

    // ----- explorer and sidebar -----

    fn show_explorer(&mut self, tab: ExplorerTab, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_cwd().is_none() {
            return;
        }
        if !self.layout.explorer_open {
            self.layout.explorer_open = true;
            self.save_layout();
        }
        self.sync_explorer(window, cx);
        if let Some(explorer) = &self.explorer {
            explorer.update(cx, |explorer, cx| explorer.select_tab(tab, cx));
        }
        cx.notify();
    }

    fn set_panels(
        &mut self,
        sidebar: bool,
        explorer: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.layout.sidebar_open = sidebar;
        self.layout.explorer_open = explorer;
        self.save_layout();
        self.sync_explorer(window, cx);
        cx.notify();
    }

    fn save_layout(&self) {
        if let Err(error) = self.layout.save_to(&data_dir()) {
            log::warn!("cannot save layout.json: {error}");
        }
    }

    // ----- command center and settings -----

    pub(crate) fn open_command_center(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.command.is_some() {
            return;
        }
        let items = {
            let client = self.client.read(cx);
            command_items(&self.shortcuts, client.projects(), client.agents())
        };
        let center = cx.new(|cx| CommandCenter::new(items, window, cx));
        let subscription = cx.subscribe_in(
            &center,
            window,
            |this: &mut Shell, _, event: &CommandEvent, window, cx| {
                this.command = None;
                match event {
                    CommandEvent::Run { id } => this.run_command(id, window, cx),
                    CommandEvent::Dismissed => this.refocus(window, cx),
                }
                cx.notify();
            },
        );
        self.command = Some((center, subscription));
        cx.notify();
    }

    fn run_command(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(action) = id.strip_prefix("action:") {
            self.run_action(action, window, cx);
        } else if let Some(workspace_id) = id.strip_prefix("workspace:") {
            self.open_workspace(workspace_id.to_string(), window, cx);
        } else if let Some(agent_id) = id.strip_prefix("agent:") {
            self.open_agent(agent_id, window, cx);
        }
    }

    fn refocus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.active_key(cx) {
            Some(key) => {
                let selected = self.selected.clone();
                self.focus_pane(&selected, &key, window, cx)
            }
            None => window.focus(&self.focus, cx),
        }
    }

    pub(crate) fn open_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.settings.is_some() {
            return;
        }
        let client = self.client.clone();
        let settings = cx.new(|cx| Settings::new(client, window, cx));
        let subscription = cx.subscribe_in(
            &settings,
            window,
            |this: &mut Shell, _, event: &SettingsEvent, window, cx| match event {
                SettingsEvent::Closed => {
                    this.settings = None;
                    this.refocus(window, cx);
                    cx.notify();
                }
                SettingsEvent::HostsChanged => this.reload_hosts(window, cx),
                SettingsEvent::ShortcutsChanged => this.reload_shortcuts(cx),
            },
        );
        self.settings = Some((settings, subscription));
        self.command = None;
        // The focused pane leaves the tree with the workspace. Without this, key dispatch
        // starts at the window root, outside the shell context, and Cmd+, and Escape do nothing.
        window.focus(&self.focus, cx);
        cx.notify();
    }

    fn reload_shortcuts(&mut self, cx: &mut Context<Self>) {
        self.shortcuts = load_shortcuts();
        let bindings = binding_specs(&self.shortcuts);
        cx.bind_keys(rebind(&self.bindings, &bindings));
        self.bindings = bindings;
        let hints = launch_hints(&self.shortcuts);
        let panes: Vec<Entity<NewTabPane>> = self
            .workspaces
            .values()
            .flat_map(|open| open.panes.values())
            .filter_map(|pane| match &pane.content {
                PaneContent::NewTab(view) => Some(view.clone()),
                _ => None,
            })
            .collect();
        for pane in panes {
            pane.update(cx, |pane, cx| pane.set_hints(hints.clone(), cx));
        }
    }

    // ----- actions -----

    /// Runs one shortcut action. Unknown ids do nothing.
    pub(crate) fn run_action(&mut self, action: &str, window: &mut Window, cx: &mut Context<Self>) {
        let selected = self.selected.clone();
        match action {
            "command-center.toggle" => match self.command.take() {
                Some(_) => self.refocus(window, cx),
                None => self.open_command_center(window, cx),
            },
            "settings.toggle" => match self.settings.take() {
                Some(_) => self.refocus(window, cx),
                None => self.open_settings(window, cx),
            },
            "theme.cycle" => {
                let dir = data_dir();
                let mut appearance = bachuc_settings::appearance::load_from(&dir);
                appearance.theme = next_theme(appearance.theme);
                if let Err(error) = bachuc_settings::appearance::save_to(&dir, &appearance) {
                    log::warn!("cannot save appearance.json: {error}");
                }
                bachuc_settings::appearance::apply(&appearance, cx);
            }
            "host.cycle" => {
                let next = (self.host_index + 1) % self.hosts.len();
                self.switch_host(next, window, cx);
            }
            "workspace.navigate.prev" | "workspace.navigate.next" => {
                let next = {
                    let projects = self.client.read(cx).projects();
                    let order = workspace_order(projects);
                    let current = (selected != HOME).then_some(selected.as_str());
                    step_workspace(&order, current, action.ends_with("next"))
                };
                if let Some(id) = next {
                    self.open_workspace(id, window, cx);
                }
            }
            "workspace.archive" => self.archive_workspace(window, cx),
            "workspace.pane.maximize" => self.with_workspace(
                &selected,
                cx,
                |workspace, window, cx| workspace.toggle_maximize(window, cx),
                window,
            ),
            "workspace.tab.menu.open" => self.open_new_tab(&selected, None, window, cx),
            "workspace.tab.target.agent" => self.open_draft(&selected, None, window, cx),
            "workspace.terminal.new" => self.open_terminal(&selected, None, window, cx),
            "workspace.tab.target.changes" => self.show_explorer(ExplorerTab::Changes, window, cx),
            "workspace.tab.target.files" => self.show_explorer(ExplorerTab::Files, window, cx),
            "workspace.tab.close.current" => self.with_workspace(
                &selected,
                cx,
                |workspace, window, cx| workspace.close_active(window, cx),
                window,
            ),
            "workspace.tab.navigate.prev" | "workspace.tab.navigate.next" => {
                let forward = action.ends_with("next");
                self.with_workspace(
                    &selected,
                    cx,
                    |workspace, window, cx| {
                        let layout = workspace.layout();
                        let next = layout.group(layout.focused()).and_then(|group| {
                            step_index(group.items.len(), Some(group.active), forward)
                        });
                        if let Some(index) = next {
                            workspace.activate_index(index, window, cx);
                        }
                    },
                    window,
                );
            }
            "workspace.pane.split.right" => self.with_workspace(
                &selected,
                cx,
                |workspace, window, cx| workspace.split(SplitAxis::Right, window, cx),
                window,
            ),
            "workspace.pane.split.down" => self.with_workspace(
                &selected,
                cx,
                |workspace, window, cx| workspace.split(SplitAxis::Down, window, cx),
                window,
            ),
            "workspace.pane.focus.left"
            | "workspace.pane.focus.right"
            | "workspace.pane.focus.up"
            | "workspace.pane.focus.down" => {
                let direction = action.rsplit('.').next().unwrap_or_default().to_string();
                self.with_workspace(
                    &selected,
                    cx,
                    move |workspace, window, cx| {
                        if let Some(key) = neighbor_active_key(workspace, &direction)
                            && let Some(item) = workspace.layout().item(&key).cloned()
                        {
                            workspace.open(item, window, cx);
                        }
                    },
                    window,
                );
            }
            "workspace.pane.close" => self.with_workspace(
                &selected,
                cx,
                |workspace, window, cx| {
                    let layout = workspace.layout();
                    let keys: Vec<String> = layout
                        .group(layout.focused())
                        .map(|group| group.items.iter().map(|item| item.key.clone()).collect())
                        .unwrap_or_default();
                    for key in keys {
                        workspace.close(&key, window, cx);
                    }
                },
                window,
            ),
            "sidebar.toggle.left" => {
                let explorer = self.layout.explorer_open;
                self.set_panels(!self.layout.sidebar_open, explorer, window, cx)
            }
            "sidebar.toggle.right" => {
                let sidebar = self.layout.sidebar_open;
                self.set_panels(sidebar, !self.layout.explorer_open, window, cx)
            }
            "sidebar.toggle.both" => {
                // Electron hides both when either shows, else shows both.
                let show = !(self.layout.sidebar_open || self.layout.explorer_open);
                self.set_panels(show, show, window, cx)
            }
            "message-input.focus" => {
                if let Some(key) = self.active_key(cx) {
                    self.focus_pane(&selected, &key, window, cx);
                }
            }
            "message-input.mode-cycle" => {
                if let Some(agent_id) = self.active_agent(cx) {
                    let next =
                        self.client
                            .read(cx)
                            .agent_controls(&agent_id)
                            .and_then(|controls| {
                                next_mode(&controls.modes, controls.current_mode.as_deref())
                            });
                    if let Some(mode) = next {
                        self.client
                            .update(cx, |client, cx| client.set_mode(&agent_id, &mode, cx));
                    }
                }
            }
            "agent.interrupt" => {
                let running = self.active_agent(cx).filter(|agent_id| {
                    self.client
                        .read(cx)
                        .agent_controls(agent_id)
                        .is_some_and(|controls| controls.running)
                });
                match running {
                    Some(agent_id) => self
                        .client
                        .update(cx, |client, cx| client.cancel_agent(&agent_id, cx)),
                    // Escape keeps its meaning in the pane when nothing runs.
                    None => cx.propagate(),
                }
            }
            other => log::warn!("unknown shortcut action {other}"),
        }
        cx.notify();
    }

    // ----- layout -----

    fn render_explorer(&mut self, cx: &mut Context<Self>) -> Option<AnyElement> {
        if !self.layout.explorer_open || self.selected_cwd().is_none() {
            return None;
        }
        let explorer = self.explorer.clone()?;
        let t = tokens(cx);
        let width = explorer_width(
            self.layout.explorer_width,
            f32::from(self.area.get().size.width),
        );
        Some(
            div()
                .id("explorer-column")
                .relative()
                .flex_none()
                .h_full()
                .w(px(width))
                .border_l_1()
                .border_color(t.border)
                .child(explorer)
                .child(
                    div()
                        .id("explorer-resize-handle")
                        .role(Role::Splitter)
                        .aria_label("Resize Explorer")
                        .aria_numeric_value(f64::from(width))
                        .group("explorer-resize")
                        .absolute()
                        .top_0()
                        .bottom_0()
                        .left(px(-HANDLE_WIDTH / 2.))
                        .w(px(HANDLE_WIDTH))
                        .cursor_col_resize()
                        .on_drag(ExplorerResize, |_, _, _, cx| cx.new(|_| NoPreview))
                        .child(
                            div()
                                .absolute()
                                .top_0()
                                .bottom_0()
                                .left(px(HANDLE_WIDTH / 2.))
                                .w(px(1.))
                                .bg(t.foreground.opacity(0.25))
                                .invisible()
                                .group_hover("explorer-resize", |style| style.visible()),
                        ),
                )
                .into_any_element(),
        )
    }

    fn render_center(&mut self, cx: &mut Context<Self>) -> AnyElement {
        if let Some(gate) = self.render_gate(cx) {
            return gate;
        }
        match self.workspaces.get(&self.selected) {
            Some(open) => open.view.clone().into_any_element(),
            None => div().into_any_element(),
        }
    }
}

fn copy_text(text: String, notice: &'static str, window: &mut Window, cx: &mut App) {
    cx.write_to_clipboard(ClipboardItem::new_string(text));
    window.push_notification(notice, cx);
}

/// The active key of the group next to the focused one in `direction` (`left`, `right`, `up`,
/// `down`), by the centers of the group rects.
fn neighbor_active_key(workspace: &Workspace, direction: &str) -> Option<String> {
    let layout = workspace.layout();
    let rects = layout.rects(Rect {
        x: 0.,
        y: 0.,
        w: 1.,
        h: 1.,
    });
    let focused = rects.get(&layout.focused())?;
    let center = |rect: &Rect| (rect.x + rect.w / 2., rect.y + rect.h / 2.);
    let (fx, fy) = center(focused);
    layout
        .groups()
        .into_iter()
        .filter(|group| group.id != layout.focused())
        .filter_map(|group| {
            let (x, y) = center(rects.get(&group.id)?);
            let (dx, dy) = (x - fx, y - fy);
            let along = match direction {
                "left" => -dx,
                "right" => dx,
                "up" => -dy,
                "down" => dy,
                _ => return None,
            };
            (along > 0.).then(|| (along + (dx.abs() + dy.abs() - along.abs()), group))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .and_then(|(_, group)| group.active_item().map(|item| item.key.clone()))
}

impl Render for Shell {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = tokens(cx);
        if let Some((settings, _)) = &self.settings {
            let settings: AnyView = settings.clone().into();
            return div()
                .id("shell")
                .role(Role::Group)
                .aria_label("Bachuc")
                .key_context(settings_key_context().as_str())
                .track_focus(&self.focus)
                .on_action(cx.listener(|this, action: &RunAction, window, cx| {
                    this.run_action(&action.0, window, cx)
                }))
                .size_full()
                .bg(t.surface0)
                .text_color(t.foreground)
                .child(self.render_settings(settings, cx))
                .into_any_element();
        }
        let sidebar = self
            .layout
            .sidebar_open
            .then(|| self.render_sidebar_column(cx));
        let header = self.render_header(cx);
        let center = self.render_center(cx);
        let explorer = self.render_explorer(cx);
        let area = self.area.clone();
        div()
            .id("shell")
            .role(Role::Group)
            .aria_label("Bachuc")
            .key_context(SHELL_CONTEXT)
            .track_focus(&self.focus)
            .on_action(cx.listener(|this, action: &RunAction, window, cx| {
                this.run_action(&action.0, window, cx)
            }))
            .on_action(cx.listener(|this, action: &JumpToTab, window, cx| {
                let index = action.0;
                let selected = this.selected.clone();
                this.with_workspace(
                    &selected,
                    cx,
                    |workspace, window, cx| workspace.activate_index(index, window, cx),
                    window,
                );
            }))
            .relative()
            .size_full()
            .flex()
            .flex_row()
            .bg(t.surface_workspace)
            .text_color(t.foreground)
            .children(sidebar)
            .child(
                h_flex()
                    .id("workspace-area")
                    .relative()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .on_drag_move::<ExplorerResize>(cx.listener(
                        |this, event: &DragMoveEvent<ExplorerResize>, _, cx| {
                            let bounds = event.bounds;
                            let width = f32::from(bounds.right() - event.event.position.x);
                            let width = explorer_width(width, f32::from(bounds.size.width));
                            if (width - this.layout.explorer_width).abs() >= 0.5 {
                                this.layout.explorer_width = width;
                                cx.notify();
                            }
                        },
                    ))
                    .on_drop::<ExplorerResize>(cx.listener(|this, _, _, _| this.save_layout()))
                    .child(
                        canvas(move |bounds, _, _| area.set(bounds), |_, _, _, _| {})
                            .absolute()
                            .size_full(),
                    )
                    .child(
                        div()
                            .id("main")
                            .role(Role::Main)
                            .aria_label("Workspace")
                            .flex_1()
                            .min_w_0()
                            .h_full()
                            .flex()
                            .flex_col()
                            .bg(t.surface0)
                            .child(header)
                            .child(div().flex_1().min_h_0().w_full().child(center)),
                    )
                    .children(explorer),
            )
            .when_some(self.command.as_ref(), |root, (center, _)| {
                root.child(center.clone())
            })
            .into_any_element()
    }
}
