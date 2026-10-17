//! The window, laid out as Electron's `workspace-screen.tsx`: the sidebar column on the left, the
//! selected workspace (header, tabs, panes) in the middle, and the Explorer on the right. One
//! `Workspace` view lives per opened workspace; the sidebar picks which one shows. The command
//! center and the settings screen open over it.

use crate::actions::{
    BindingSpec, JumpToTab, RunAction, SHELL_CONTEXT, binding_specs, rebind, settings_key_context,
};
use crate::chrome::{ExplorerResize, HANDLE_WIDTH, HeaderMenu, NoPreview};
use crate::dock;
use crate::new_workspace::{self, NewWorkspacePreset, NewWorkspaceScreen, SidePanelChanged};
use crate::panes::{
    AgentPane, Launch, LaunchItem, LazyAgentPane, NewTabPane, PaneFrame, launch_hints,
};
use crate::plugin_views::{
    OpenSurface, PluginViews, surface_body, surface_header, surface_overlay, surface_page,
};
use crate::plugins::{
    keystroke_text, panel_identity, panel_location, plugin_shortcut, surface_heading,
};
use crate::state::{
    Connection, EditorTarget, LayoutPrefs, LogoFill, absolute_path, agent_icon, agent_key,
    agent_of_key, agent_title, all_diffs_key, badge_count, badge_label, command_items,
    commit_diff_key, commit_diff_title, diff_key, editor_targets, explorer_width, file_key,
    file_name, file_of_key, find_workspace, initial_host, load_ship_default, next_mode,
    notification_for, pull_request_key, save_ship_default, step_index, step_workspace,
    workspace_agents, workspace_order,
};
use crate::toast::{TOAST_DURATION, ToastStack, render_toasts};
use crate::workspace_draft::{self, WorkspaceDraftPane};
use bachuc_client::{
    AgentControls, AgentFeature, AgentFeatureKind, AgentSummary, Client, ClientEvent,
    CreatedWorkspace, DirectoryEvent, GitStatusEvent, HostConfig, PrStatus, ProjectNode,
    ThinkingControls, TimelineState, load_hosts, resume_command,
};
use bachuc_command::{
    CommandCenter, CommandEvent, CommandFileEvent, FileResult, Shortcut, ShortcutsDialog,
    ShortcutsDialogEvent, data_dir, load_shortcuts,
};
use bachuc_composer::{ComposerEvent, DraftSeed};
use bachuc_diagram::DiagramRenderer;
use bachuc_explorer::{
    DiffView, Explorer, ExplorerDiffEvent, ExplorerEvent, ExplorerFileEvent, ExplorerNoticeEvent,
    ExplorerTab, FileView, GitActionsButton, GitActionsEvent, PullRequestEvent,
    PullRequestNoticeEvent, PullRequestView, file_icons::file_icon,
};
use bachuc_history::{HistoryEvent, HistoryScreen};
use bachuc_icons::{IconName, provider_logo};
use bachuc_plugin_host::{
    AgentAnchor, PanelInput, PanelTarget, PluginHost, PluginHostEvent, SurfacePresentation,
};
use bachuc_settings::{Settings, SettingsEvent};
use bachuc_sidebar::model::archive_prompt;
use bachuc_sidebar::{Sidebar, SidebarEvent, SidebarNavEvent, SidebarPluginEvent};
use bachuc_terminal::{TerminalEvent, TerminalView};
use bachuc_theme::metrics::TRAFFIC_LIGHT_WIDTH;
use bachuc_theme::tokens;
use bachuc_timeline::grouping::{item_type, message_key};
use bachuc_timeline::{ForkTarget, TimelineEvent, TimelineForkEvent, TimelineView};
use bachuc_workspace::model::Rect;
use bachuc_workspace::{PaneEvent, PaneItem, SplitAxis, TabKind, TabMenuEvent, Workspace};
use gpui_kit::component::button::ButtonVariant;
use gpui_kit::component::dialog::DialogButtonProps;
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::notification::Notification;
use gpui_kit::component::theme::Theme;
use gpui_kit::component::{WindowExt as _, h_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::cell::Cell;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::rc::Rc;
use std::time::{Duration, Instant};

/// The old client lives this long after a host switch, so a queued terminal kill reaches the
/// old daemon before its socket closes.
const RETIRE_AFTER: Duration = Duration::from_secs(2);

/// The workspace view shown while no workspace is selected, for example with zero projects.
const HOME: &str = "";

/// The identity of the History tab in the home view, so a second open shows the same tab.
const HISTORY: &str = "history";

/// A git status change checks the PR of a workspace at most this often, so a busy agent's
/// edits do not ask the daemon for the PR on every file write.
const PR_RECHECK: Duration = Duration::from_secs(10);

/// What a tab shows. Dropping an agent pane stops its timeline sync.
pub(crate) enum PaneContent {
    Agent(Entity<AgentPane>),
    /// An agent tab not shown yet; `materialize` makes its `AgentPane`.
    LazyAgent(Entity<LazyAgentPane>),
    /// The workspace draft tab.
    Draft(Entity<WorkspaceDraftPane>),
    /// The New workspace screen, in a tab of the home view.
    NewWorkspace(Entity<NewWorkspaceScreen>),
    /// The History screen, in a tab of the home view. It has no focus handle, so it gets a
    /// `PaneFrame`.
    History(Entity<HistoryScreen>),
    /// The Pull request tab of a workspace folder. It has no focus handle, so it gets a
    /// `PaneFrame`.
    PullRequest(Entity<PullRequestView>),
    NewTab(Entity<NewTabPane>),
    Terminal(Entity<TerminalView>),
    File(Entity<FileView>),
    Diff(Entity<DiffView>),
    /// A plugin workspace panel, built once when its tab opens.
    Plugin(AnyView),
}

impl PaneContent {
    fn view(&self) -> AnyView {
        match self {
            PaneContent::Agent(view) => view.clone().into(),
            PaneContent::LazyAgent(view) => view.clone().into(),
            PaneContent::Draft(view) => view.clone().into(),
            PaneContent::NewWorkspace(view) => view.clone().into(),
            PaneContent::History(view) => view.clone().into(),
            PaneContent::PullRequest(view) => view.clone().into(),
            PaneContent::NewTab(view) => view.clone().into(),
            PaneContent::Terminal(view) => view.clone().into(),
            PaneContent::File(view) => view.clone().into(),
            PaneContent::Diff(view) => view.clone().into(),
            PaneContent::Plugin(view) => view.clone(),
        }
    }

    /// Whether the pane has its own focusable element with a role: a composer, a prompt, or a
    /// terminal. Other panes, such as History and Pull request, get a `PaneFrame`.
    fn owns_focus(&self) -> bool {
        matches!(
            self,
            PaneContent::Agent(_)
                | PaneContent::Draft(_)
                | PaneContent::NewWorkspace(_)
                | PaneContent::Terminal(_)
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
            // History, Pull request, and the other panes have no tab menu of their own.
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

/// A workspace the New workspace screen created, and the tab of that screen.
#[derive(Clone, Debug)]
struct PendingOpen {
    screen_workspace: String,
    screen_key: String,
    created: CreatedWorkspace,
}

pub(crate) struct OpenWorkspace {
    cwd: Option<String>,
    view: Entity<Workspace>,
    /// The header's git split button, made on the first open of a workspace with a folder.
    git_actions: Option<Entity<GitActionsButton>>,
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
    command: Option<(Entity<CommandCenter>, Vec<Subscription>)>,
    /// The generation of the last Files scope query, so an answer to an older one is dropped.
    file_query: u64,
    shortcuts_dialog: Option<(Entity<ShortcutsDialog>, Subscription)>,
    /// A file that "Open to the side" puts in the group a split just made: (workspace, cwd,
    /// path). The split's `PaneEvent::NewTab` opens it in place of a new tab.
    side_file: Option<(String, String, String)>,
    settings: Option<(Entity<Settings>, Subscription)>,
    pub(crate) dragging_window: bool,
    /// A workspace the New workspace screen created, opened once the client lists it.
    pending_open: Option<PendingOpen>,
    /// The plugin host that `main` started; it follows the active host and tab.
    plugin_host: Option<Entity<PluginHost>>,
    /// The workspace and agent last given to `PluginHost::set_active`.
    plugin_active: (Option<String>, Option<String>),
    /// The plugin surface on screen, as a page or an overlay.
    surface: Option<OpenSurface>,
    /// An agent opened at its latest prompt before its timeline loaded.
    pending_prompt: Option<String>,
    plugin_views: PluginViews,
    /// The New workspace side panel choice, kept for the session.
    side_panel_open: bool,
    toasts: ToastStack,
    /// Workspaces whose PR was found once this session; their PR tab never opens by itself
    /// again, so a closed one stays closed.
    pr_tab_opened: HashSet<String>,
    /// Workspaces with a PR request in flight, and when each was last asked.
    pr_inflight: HashSet<String>,
    pr_checked: HashMap<String, Instant>,
    /// The header menu that shows, if any.
    pub(crate) header_menu: Option<HeaderMenu>,
    /// The Open in editor apps, read once at start, and the last pick of the session.
    pub(crate) editor_targets: Vec<EditorTarget>,
    pub(crate) editor_choice: Option<EditorTarget>,
    /// The GitHub page of a folder, read when the Open in editor menu opens: (cwd, url).
    pub(crate) editor_github: Option<(String, String)>,
    /// Scripts with a start or stop in flight, as `workspace_id/name`.
    pub(crate) scripts_busy: HashSet<String>,
    /// Scripts that Restart stopped, as `workspace_id/name`; each starts again once the daemon
    /// reports it stopped.
    pub(crate) scripts_restart: HashSet<String>,
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
        let mut subscriptions = vec![
            // set_theme and the font and syntax setters write this global; the chrome reads
            // tokens. The logos follow only a change of their fill.
            cx.observe_global::<Theme>(|this, cx| {
                this.refresh_logos(cx);
                cx.notify()
            }),
        ];
        let plugin_host = bachuc_plugin_host::host(cx);
        if let Some(host) = &plugin_host {
            host.update(cx, |host, cx| host.set_client(client.clone(), cx));
            subscriptions.push(cx.subscribe_in(host, window, Self::on_plugin_event));
            // Plugin themes join the picker and the theme registry once the host lists them.
            bachuc_settings::watch_plugin_themes(cx);
        }
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
            file_query: 0,
            shortcuts_dialog: None,
            side_file: None,
            settings: None,
            dragging_window: false,
            pending_open: None,
            plugin_host,
            plugin_active: (None, None),
            surface: None,
            pending_prompt: None,
            plugin_views: PluginViews::default(),
            side_panel_open: false,
            toasts: ToastStack::default(),
            pr_tab_opened: HashSet::new(),
            pr_inflight: HashSet::new(),
            pr_checked: HashMap::new(),
            header_menu: None,
            editor_targets: editor_targets(|path| std::path::Path::new(path).exists()),
            editor_choice: None,
            editor_github: None,
            scripts_busy: HashSet::new(),
            scripts_restart: HashSet::new(),
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
                        this.open_pending(window, cx);
                    }
                },
            ),
            cx.subscribe_in(
                &self.client,
                window,
                |this: &mut Shell, _, event: &GitStatusEvent, window, cx| match event {
                    GitStatusEvent::ScriptsChanged { workspace_id } => {
                        this.restart_stopped_scripts(workspace_id, cx);
                        if *workspace_id == this.selected {
                            cx.notify();
                        }
                    }
                    GitStatusEvent::Changed { cwd } => this.on_git_changed(cwd, window, cx),
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
            cx.subscribe_in(
                &self.sidebar,
                window,
                |this: &mut Shell, _, event: &SidebarNavEvent, window, cx| match event {
                    SidebarNavEvent::OpenHistory => this.open_history(window, cx),
                    SidebarNavEvent::NewWorkspace { project_id } => {
                        this.new_workspace(project_id.clone(), window, cx)
                    }
                },
            ),
            cx.subscribe_in(
                &self.sidebar,
                window,
                |this: &mut Shell, _, event: &SidebarPluginEvent, window, cx| match event {
                    SidebarPluginEvent::OpenSurface {
                        plugin_id,
                        surface_id,
                    } => this.open_surface(
                        OpenSurface {
                            plugin_id: plugin_id.clone(),
                            surface_id: surface_id.clone(),
                            presentation: SurfacePresentation::Page,
                            params: BTreeMap::new(),
                        },
                        window,
                        cx,
                    ),
                    // The project list follows the daemon; the filters already ran.
                    SidebarPluginEvent::ProjectAdded { .. } => {}
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
        if let Some(agent_id) = self.pending_prompt.clone()
            && matches!(event, ClientEvent::TimelineChanged { agent_id: id } if *id == agent_id)
        {
            self.pending_prompt = None;
            // The shell subscribed to the client before the timeline view, so the view has not
            // synced this change yet; the reveal runs once every subscriber has.
            cx.defer_in(window, move |this, window, cx| {
                this.reveal_latest_prompt(&agent_id, window, cx)
            });
        }
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
                // A PR request sent while the client connected failed; ask again.
                let open: Vec<String> = self.workspaces.keys().cloned().collect();
                for workspace_id in open {
                    self.check_pull_request(&workspace_id, window, cx);
                }
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

    /// A live agent's title, else an archived one's, which is never in `agents`.
    fn agent_tab_title(&self, agent_id: &str, cx: &App) -> String {
        let client = self.client.read(cx);
        client
            .agents()
            .iter()
            .find(|agent| agent.id == agent_id)
            .or_else(|| client.archived_agent(agent_id))
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
        self.pending_open = None;
        self.pending_prompt = None;
        self.pr_tab_opened.clear();
        self.pr_inflight.clear();
        self.pr_checked.clear();
        self.header_menu = None;
        self.editor_github = None;
        self.scripts_busy.clear();
        self.connection = Connection::Connecting;
        let client = Client::connect(self.hosts[self.host_index].clone(), cx);
        self.retired = Some(std::mem::replace(&mut self.client, client.clone()));
        // Plugins follow the new host: the host deactivates them and lists the new host's.
        self.set_surface(None, cx);
        if let Some(host) = self.plugin_host.clone() {
            host.update(cx, |host, cx| host.set_client(client.clone(), cx));
        }
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
        self.sync_plugin_active(cx);
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

    /// The header's git split button of an open workspace.
    pub(crate) fn git_actions(&self, workspace_id: &str) -> Option<Entity<GitActionsButton>> {
        self.workspaces.get(workspace_id)?.git_actions.clone()
    }

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
                git_actions: None,
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
            self.attach_git_actions(&id, window, cx);
            self.check_pull_request(&id, window, cx);
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
        self.close_page_surface(cx);
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
        self.sync_plugin_active(cx);
        cx.notify();
    }

    /// Tells the plugin host the selected workspace and the focused agent tab, on a change.
    fn sync_plugin_active(&mut self, cx: &mut Context<Self>) {
        self.sync_chat_target(cx);
        let workspace = (self.selected != HOME).then(|| self.selected.clone());
        let active = (workspace, self.active_agent(cx));
        if active == self.plugin_active {
            return;
        }
        self.plugin_active = active.clone();
        if let Some(center) = self.command.as_ref().map(|(center, _)| center.clone()) {
            let (workspace, agent) = active.clone();
            center.update(cx, |center, cx| {
                center.set_plugin_context(workspace, agent, cx)
            });
        }
        if let Some(host) = self.plugin_host.clone() {
            host.update(cx, |host, cx| host.set_active(active.0, active.1, cx));
        }
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
                let file_subscription = cx.subscribe_in(
                    &explorer,
                    window,
                    |this: &mut Shell, _, event: &ExplorerFileEvent, window, cx| match event {
                        ExplorerFileEvent::OpenToSide { cwd, path } => {
                            this.open_file_to_side(cwd, path, window, cx)
                        }
                        ExplorerFileEvent::AddToChat { cwd, path } => {
                            this.add_file_to_chat(cwd, path, window, cx)
                        }
                    },
                );
                let notice_subscription = cx.subscribe(
                    &explorer,
                    |this: &mut Shell, _, event: &ExplorerNoticeEvent, cx| {
                        this.show_toast(event.ok, event.message.clone(), cx)
                    },
                );
                self.host_subscriptions.push(subscription);
                self.host_subscriptions.push(diff_subscription);
                self.host_subscriptions.push(file_subscription);
                self.host_subscriptions.push(notice_subscription);
                self.explorer = Some(explorer);
                self.sync_chat_target(cx);
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
                self.sync_plugin_active(cx);
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
                self.sync_plugin_active(cx);
                cx.notify();
            }
            PaneEvent::NewTab => match self.side_file.take() {
                Some((workspace, cwd, path)) if workspace == workspace_id => {
                    self.open_file(&cwd, &path, None, window, cx)
                }
                _ => self.open_new_tab(workspace_id, None, window, cx),
            },
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
            PaneContent::NewWorkspace(view) => view.update(cx, |pane, cx| pane.focus(window, cx)),
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
        // A tab replaced in place (a new tab that becomes a draft) emits no `Activated`.
        self.sync_chat_target(cx);
        cx.notify();
    }

    /// A live agent's provider, else an archived one's.
    fn agent_provider(&self, agent_id: &str, cx: &App) -> Option<String> {
        let client = self.client.read(cx);
        client
            .agents()
            .iter()
            .find(|agent| agent.id == agent_id)
            .or_else(|| client.archived_agent(agent_id))
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
                let pane = self.pane(workspace_id, key);
                let agent_id = pane.and_then(|pane| agent_of_key(pane.identity.as_deref()?));
                let terminal_id = match pane.map(|pane| &pane.content) {
                    Some(PaneContent::Terminal(view)) => {
                        Some(view.read(cx).terminal_id().map(str::to_string))
                    }
                    _ => None,
                };
                match rename_target(agent_id, terminal_id) {
                    RenameTarget::Agent(agent_id) => self.rename_agent(agent_id, window, cx),
                    RenameTarget::Terminal(terminal_id) => {
                        self.rename_terminal(workspace_id, key, terminal_id, window, cx)
                    }
                    RenameTarget::Notice(notice) => window.push_notification(notice, cx),
                }
            }
            TabMenuEvent::CopyResumeCommand { key } => {
                let Some(agent_id) = self
                    .pane(workspace_id, key)
                    .and_then(|pane| agent_of_key(pane.identity.as_deref()?))
                    .map(str::to_string)
                else {
                    return;
                };
                let provider = self.agent_provider(&agent_id, cx);
                let session = self.client.read(cx).agent_provider_session_id(&agent_id);
                match resume_text(provider.as_deref(), session.as_deref()) {
                    Ok(command) => copy_text(command, "Resume command copied", window, cx),
                    Err(notice) => window.push_notification(Notification::error(notice), cx),
                }
            }
            TabMenuEvent::ReloadAgent { key } => {
                let agent_id = self
                    .pane(workspace_id, key)
                    .and_then(|pane| agent_of_key(pane.identity.as_deref()?))
                    .map(str::to_string);
                if let Some(agent_id) = agent_id {
                    self.reload_agent(agent_id, window, cx);
                }
            }
        }
    }

    /// `handleReloadAgent`: refreshes the agent, then views it again, so the timeline loads
    /// the new epoch.
    fn reload_agent(&mut self, agent_id: String, window: &mut Window, cx: &mut Context<Self>) {
        window.push_notification("Reloading agent...", cx);
        let task = self
            .client
            .update(cx, |client, cx| client.refresh_agent(&agent_id, cx));
        let client = self.client.clone();
        cx.spawn_in(window, async move |_, cx| {
            let result = task.await;
            let _ = cx.update(|window, cx| match result {
                Ok(()) => {
                    // `view_agent` does nothing for a viewed agent, so the sync starts over.
                    client.update(cx, |client, cx| {
                        client.unview_agent(&agent_id, cx);
                        client.view_agent(&agent_id, cx);
                    });
                    window.push_notification("Reloaded agent", cx);
                }
                Err(error) => {
                    let notice = reload_error(&format!("{error:#}"));
                    window.push_notification(Notification::error(notice), cx);
                }
            });
        })
        .detach();
    }

    /// The `rename_agent` dialog for a terminal. The tab keeps its title until the daemon
    /// accepts the new one.
    fn rename_terminal(
        &mut self,
        workspace_id: &str,
        key: &str,
        terminal_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let current = self
            .pane(workspace_id, key)
            .map(|pane| pane.title.to_string())
            .unwrap_or_default();
        let input = cx.new(|cx| InputState::new(window, cx).default_value(current));
        let client = self.client.clone();
        let shell = cx.weak_entity();
        let (workspace_id, key) = (workspace_id.to_string(), key.to_string());
        let focus_once = Cell::new(true);
        window.open_dialog(cx, move |dialog, window, cx| {
            if focus_once.replace(false) {
                input.update(cx, |input, cx| input.focus(window, cx));
            }
            let client = client.clone();
            let shell = shell.clone();
            let field = input.clone();
            let terminal_id = terminal_id.clone();
            let workspace_id = workspace_id.clone();
            let key = key.clone();
            dialog
                .title("Rename terminal")
                .w(px(420.))
                .child(Input::new(&input).aria_label("Terminal name"))
                .button_props(
                    DialogButtonProps::default()
                        .ok_text("Rename")
                        .show_cancel(true)
                        .on_ok(move |_, window, cx| {
                            let title = field.read(cx).value().trim().to_string();
                            if title.is_empty() {
                                return true;
                            }
                            let task = client.update(cx, |client, cx| {
                                client.rename_terminal(&terminal_id, &title, cx)
                            });
                            let shell = shell.clone();
                            let workspace_id = workspace_id.clone();
                            let key = key.clone();
                            window
                                .spawn(cx, async move |cx| {
                                    let result = task.await;
                                    let _ = shell.update_in(cx, |shell, window, cx| match result {
                                        Ok(()) => {
                                            shell.retitle(&workspace_id, &key, title.into(), cx)
                                        }
                                        Err(error) => window.push_notification(
                                            Notification::error(format!("{error:#}")),
                                            cx,
                                        ),
                                    });
                                })
                                .detach();
                            true
                        }),
                )
        });
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

    /// The Agent launch: the workspace draft tab in a workspace, else the New workspace screen,
    /// as Electron does.
    fn open_draft(
        &mut self,
        workspace_id: &str,
        replace: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let cwd = self
            .workspaces
            .get(workspace_id)
            .and_then(|open| open.cwd.clone());
        let Some(cwd) = cwd else {
            let preset = NewWorkspacePreset::default();
            self.open_new_workspace_screen(workspace_id, replace, preset, window, cx);
            return;
        };
        self.open_draft_pane(workspace_id, replace, cwd, window, cx);
    }

    /// A workspace draft tab in `cwd`; its submit replaces it with the agent tab.
    fn open_draft_pane(
        &mut self,
        workspace_id: &str,
        replace: Option<String>,
        cwd: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<WorkspaceDraftPane> {
        let key = replace.unwrap_or_else(|| self.next_key());
        let client = self.client.clone();
        let pane_key = key.clone();
        let view = cx.new(|cx| WorkspaceDraftPane::new(client, pane_key, cwd, window, cx));
        let workspace = workspace_id.to_string();
        let subscription = cx.subscribe_in(
            &view,
            window,
            move |this: &mut Shell, _, created: &workspace_draft::Created, window, cx| {
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
        view
    }

    /// Opens the New workspace screen in a new tab of the home view and shows it.
    pub(crate) fn open_new_workspace(
        &mut self,
        preset: NewWorkspacePreset,
        window: &mut Window,
        cx: &mut Context<Shell>,
    ) -> Entity<NewWorkspaceScreen> {
        self.ensure_home(window, cx);
        let screen = self.open_new_workspace_screen(HOME, None, preset, window, cx);
        self.select(HOME.to_string(), window, cx);
        screen
    }

    /// Shows the History screen in a tab of the home view, as `open_new_workspace` does. A
    /// second open shows the same tab. A page surface closes, as for a route change.
    fn open_history(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.ensure_home(window, cx);
        match self.find_identity(HOME, HISTORY) {
            Some(key) => self.activate_key(HOME, &key, window, cx),
            None => {
                let client = self.client.clone();
                let view = cx.new(|cx| HistoryScreen::new(client, window, cx));
                let subscription = cx.subscribe_in(
                    &view,
                    window,
                    |this: &mut Shell, _, event: &HistoryEvent, window, cx| {
                        let HistoryEvent::OpenAgent {
                            agent_id,
                            workspace_id,
                        } = event;
                        this.open_agent_in(agent_id, workspace_id.as_deref(), window, cx);
                    },
                );
                let key = self.next_key();
                self.put_pane(
                    HOME,
                    key,
                    Some(HISTORY.to_string()),
                    "History".into(),
                    IconName::History,
                    PaneContent::History(view),
                    vec![subscription],
                    window,
                    cx,
                );
            }
        }
        self.close_page_surface(cx);
        self.select(HOME.to_string(), window, cx);
    }

    fn open_new_workspace_screen(
        &mut self,
        workspace_id: &str,
        replace: Option<String>,
        preset: NewWorkspacePreset,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<NewWorkspaceScreen> {
        let key = replace.unwrap_or_else(|| self.next_key());
        let client = self.client.clone();
        let pane_key = key.clone();
        let view = cx.new(|cx| NewWorkspaceScreen::new(client, pane_key, preset, window, cx));
        let workspace = workspace_id.to_string();
        let subscription = cx.subscribe_in(
            &view,
            window,
            move |this: &mut Shell, _, created: &new_workspace::Created, window, cx| {
                this.pending_open = Some(PendingOpen {
                    screen_workspace: workspace.clone(),
                    screen_key: created.key.clone(),
                    created: created.created.clone(),
                });
                this.open_pending(window, cx);
            },
        );
        let subscriptions = vec![
            subscription,
            cx.subscribe(
                &view,
                |this: &mut Shell, _, changed: &SidePanelChanged, cx| {
                    this.side_panel_open = changed.0;
                    cx.notify();
                },
            ),
            // The header's side panel toggle follows the chosen project.
            cx.observe(&view, |_, _, cx| cx.notify()),
        ];
        let open = self.side_panel_open;
        view.update(cx, |screen, cx| screen.set_side_panel_open(open, cx));
        self.put_pane(
            workspace_id,
            key,
            None,
            "New workspace".into(),
            IconName::FolderPlus,
            PaneContent::NewWorkspace(view.clone()),
            subscriptions,
            window,
            cx,
        );
        view.update(cx, |screen, cx| screen.focus(window, cx));
        view
    }

    /// Opens the workspace the New workspace screen created once the client lists it, then its
    /// first agent's tab. A new tab takes the spent screen's place.
    fn open_pending(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(pending) = self.pending_open.clone() else {
            return;
        };
        let workspace_id = pending.created.workspace_id.clone();
        if find_workspace(self.client.read(cx).projects(), &workspace_id).is_none() {
            return;
        }
        self.pending_open = None;
        let spent = self
            .pane(&pending.screen_workspace, &pending.screen_key)
            .is_some_and(|pane| matches!(pane.content, PaneContent::NewWorkspace(_)));
        if spent {
            let key = Some(pending.screen_key.clone());
            self.open_new_tab(&pending.screen_workspace, key, window, cx);
        }
        self.open_workspace(workspace_id.clone(), window, cx);
        let Some(agent_id) = pending.created.agent_id else {
            return;
        };
        self.known_agents.insert(agent_id.clone());
        match self.find_identity(&workspace_id, &agent_key(&agent_id)) {
            Some(key) => self.activate_key(&workspace_id, &key, window, cx),
            None => {
                // The agent may not be listed yet; its tab takes the empty new tab's place.
                let replace = self.workspace_active_key(&workspace_id, cx).filter(|key| {
                    self.pane(&workspace_id, key)
                        .is_some_and(|pane| matches!(pane.content, PaneContent::NewTab(_)))
                });
                self.open_agent_pane(&workspace_id, replace, &agent_id, window, cx);
            }
        }
    }

    /// The diff stat pill: the all-files diff tab of that workspace's folder.
    fn open_workspace_changes(
        &mut self,
        workspace_id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let cwd = find_workspace(self.client.read(cx).projects(), workspace_id)
            .map(|(_, workspace)| workspace.cwd.clone());
        let Some(cwd) = cwd else {
            return;
        };
        if self.selected != workspace_id {
            self.open_workspace(workspace_id.to_string(), window, cx);
        }
        self.open_all_diffs(&cwd, None, window, cx);
    }

    // ----- toasts, git actions, and the Pull request tab -----

    /// Shows a success or error note at the bottom right for `TOAST_DURATION`.
    pub(crate) fn show_toast(&mut self, ok: bool, message: String, cx: &mut Context<Self>) {
        let Some(id) = self.toasts.push(ok, message) else {
            return;
        };
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(TOAST_DURATION).await;
            let _ = this.update(cx, |shell, cx| {
                if shell.toasts.dismiss(id) {
                    cx.notify();
                }
            });
        })
        .detach();
        cx.notify();
    }

    /// A fork picked in a turn footer, as `useForkAgent`: asks the chat history, then opens a
    /// draft seeded from the source agent, in a new tab of the same workspace or on the New
    /// workspace screen. Nothing runs until the user sends.
    fn fork(
        &mut self,
        workspace_id: &str,
        timeline: Entity<TimelineView>,
        event: &TimelineForkEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let TimelineForkEvent::Fork {
            agent_id,
            target,
            boundary,
        } = event;
        let client = self.client.read(cx);
        let agent = client
            .agents()
            .iter()
            .find(|agent| agent.id == *agent_id)
            .or_else(|| client.archived_agent(agent_id));
        let seed = fork_draft_seed(
            agent,
            client.agent_controls(agent_id).as_ref(),
            client.agent_thinking(agent_id).as_ref(),
            &client.agent_features(agent_id),
        );
        let agent_cwd = agent.map(|agent| agent.cwd.clone()).unwrap_or_default();
        let open = match target {
            ForkTarget::Tab => {
                let workspace_cwd = self
                    .workspaces
                    .get(workspace_id)
                    .and_then(|open| open.cwd.as_deref());
                match fork_tab_cwd(workspace_id, &agent_cwd, workspace_cwd) {
                    Ok(cwd) => ForkOpen::Tab(cwd),
                    Err(message) => {
                        self.show_toast(false, message.to_string(), cx);
                        return;
                    }
                }
            }
            ForkTarget::Workspace => {
                let source_workspace = agent
                    .and_then(|agent| agent.workspace_id.clone())
                    .or_else(|| (workspace_id != HOME).then(|| workspace_id.to_string()));
                ForkOpen::Workspace(fork_workspace_preset(
                    client.projects(),
                    source_workspace.as_deref(),
                    &agent_cwd,
                ))
            }
        };
        timeline.update(cx, |timeline, cx| timeline.set_fork_pending(true, cx));
        let task = self.client.update(cx, |client, cx| {
            client.fork_context(agent_id, boundary.clone(), cx)
        });
        let workspace = workspace_id.to_string();
        cx.spawn_in(window, async move |this, cx| {
            let result = task.await;
            let _ = this.update_in(cx, |shell, window, cx| {
                timeline.update(cx, |timeline, cx| timeline.set_fork_pending(false, cx));
                let context = match result {
                    Ok(context) => context,
                    Err(error) => {
                        shell.show_toast(false, fork_error(&format!("{error:#}")), cx);
                        return;
                    }
                };
                let label = fork_attachment_label(context.title.as_deref());
                let attachment = context.attachment;
                match open {
                    ForkOpen::Tab(cwd) => {
                        let draft = shell.open_draft_pane(&workspace, None, cwd, window, cx);
                        draft.update(cx, |draft, cx| {
                            draft.seed_fork(seed, label, attachment, window, cx)
                        });
                    }
                    ForkOpen::Workspace(preset) => {
                        let screen = shell.open_new_workspace(preset, window, cx);
                        screen.update(cx, |screen, cx| {
                            screen.seed_fork(seed, label, attachment, window, cx)
                        });
                    }
                }
            });
        })
        .detach();
    }

    /// Makes the header's git split button of a workspace with a folder, with the repo's saved
    /// ship default (Q-111).
    fn attach_git_actions(
        &mut self,
        workspace_id: &str,
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
        let saved = load_ship_default(&data_dir(), &cwd);
        let client = self.client.clone();
        let button = cx.new(|cx| {
            let mut button = GitActionsButton::new(client, cwd, cx);
            button.set_ship_default(saved, cx);
            button
        });
        let id = workspace_id.to_string();
        let subscription = cx.subscribe_in(
            &button,
            window,
            move |this: &mut Shell, _, event: &GitActionsEvent, window, cx| {
                this.on_git_actions_event(&id, event, window, cx)
            },
        );
        if let Some(open) = self.workspaces.get_mut(workspace_id) {
            open.git_actions = Some(button);
            open._subscriptions.push(subscription);
        }
    }

    fn on_git_actions_event(
        &mut self,
        workspace_id: &str,
        event: &GitActionsEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            GitActionsEvent::Finished { ok, message } => {
                self.show_toast(*ok, message.clone(), cx);
                // Create PR finds the PR at once, without waiting for the status throttle.
                if *ok {
                    self.check_pull_request(workspace_id, window, cx);
                }
            }
            GitActionsEvent::OpenUrl { url } => cx.open_url(url),
            GitActionsEvent::ArchiveWorkspace => {
                self.archive_workspace_id(workspace_id, window, cx)
            }
            GitActionsEvent::ShipDefaultChanged { cwd, default } => {
                if let Err(error) = save_ship_default(&data_dir(), cwd, *default) {
                    log::warn!("cannot save the git ship default: {error}");
                }
            }
        }
    }

    /// A git status change may come with a new PR; checks each open workspace on that folder
    /// whose PR was not found yet, at most once per `PR_RECHECK`.
    fn on_git_changed(&mut self, cwd: &str, window: &mut Window, cx: &mut Context<Self>) {
        let now = Instant::now();
        let due: Vec<String> = self
            .workspaces
            .iter()
            .filter(|(id, open)| {
                open.cwd.as_deref() == Some(cwd)
                    && !self.pr_tab_opened.contains(*id)
                    && self
                        .pr_checked
                        .get(*id)
                        .is_none_or(|last| now.duration_since(*last) >= PR_RECHECK)
            })
            .map(|(id, _)| id.clone())
            .collect();
        for workspace_id in due {
            self.check_pull_request(&workspace_id, window, cx);
        }
    }

    /// Asks for the workspace's PR, then opens its Pull request tab once by itself when the
    /// rule allows, as `usePullRequestAutoAdd` does.
    fn check_pull_request(
        &mut self,
        workspace_id: &str,
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
        if self.pr_tab_opened.contains(workspace_id)
            || !self.client.read(cx).is_connected()
            || !self.pr_inflight.insert(workspace_id.to_string())
        {
            return;
        }
        self.pr_checked
            .insert(workspace_id.to_string(), Instant::now());
        let task = self
            .client
            .update(cx, |client, cx| client.pr_status(&cwd, cx));
        let id = workspace_id.to_string();
        cx.spawn_in(window, async move |this, cx| {
            let status = task.await.ok().and_then(|reply| reply.status);
            let _ = this.update_in(cx, |shell, window, cx| {
                shell.pr_inflight.remove(&id);
                let has_tab = shell.find_identity(&id, &pull_request_key(&cwd)).is_some();
                let opened = shell.pr_tab_opened.contains(&id);
                if has_pull_request(status.as_ref()) {
                    shell.pr_tab_opened.insert(id.clone());
                }
                if auto_opens_pr_tab(status.as_ref(), opened, has_tab) {
                    shell.open_pull_request(&id, &cwd, window, cx);
                }
            });
        })
        .detach();
    }

    /// Opens the Pull request tab of a folder behind the active tab, as Electron's background
    /// open does.
    fn open_pull_request(
        &mut self,
        workspace_id: &str,
        cwd: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let previous = self.workspace_active_key(workspace_id, cx);
        let client = self.client.clone();
        let folder = cwd.to_string();
        let view = cx.new(|cx| PullRequestView::new(client, folder, cx));
        let subscription =
            cx.subscribe(
                &view,
                |_: &mut Shell, _, event: &PullRequestEvent, cx| match event {
                    PullRequestEvent::OpenUrl { url } => cx.open_url(url),
                },
            );
        let notice_subscription = cx.subscribe(
            &view,
            |this: &mut Shell, _, event: &PullRequestNoticeEvent, cx| {
                this.show_toast(event.ok, event.message.clone(), cx)
            },
        );
        let key = self.next_key();
        self.put_pane(
            workspace_id,
            key,
            Some(pull_request_key(cwd)),
            "Pull request".into(),
            IconName::GitPullRequest,
            PaneContent::PullRequest(view),
            vec![subscription, notice_subscription],
            window,
            cx,
        );
        if let Some(previous) = previous {
            self.activate_key(workspace_id, &previous, window, cx);
        }
    }

    // ----- plugins -----

    /// Routes the plugin host's UI requests. The composer handles `SetComposerText` and
    /// `AddComposerAttachment`, and the timeline the transforms, so those redraw nothing here.
    fn on_plugin_event(
        &mut self,
        host: &Entity<PluginHost>,
        event: &PluginHostEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            PluginHostEvent::ContributionsChanged => {
                let index = host.read(cx).index().clone();
                self.plugin_views.prune(&index);
                cx.notify();
            }
            PluginHostEvent::PluginStateChanged { .. } => cx.notify(),
            PluginHostEvent::OpenSurface {
                plugin_id,
                surface_id,
                presentation,
                params,
            } => self.open_surface(
                OpenSurface {
                    plugin_id: plugin_id.clone(),
                    surface_id: surface_id.clone(),
                    presentation: *presentation,
                    params: params.clone(),
                },
                window,
                cx,
            ),
            PluginHostEvent::OpenPanel {
                plugin_id,
                panel_id,
                target,
            } => self.open_plugin_panel(plugin_id, panel_id, target, window, cx),
            PluginHostEvent::OpenSettings {
                plugin_id,
                settings_id,
            } => {
                self.open_settings(window, cx);
                if let Some(settings) = self.settings.as_ref().map(|(settings, _)| settings.clone())
                {
                    settings.update(cx, |settings, cx| {
                        settings.open_plugin_screen(plugin_id, settings_id, window, cx)
                    });
                }
            }
            // Settings has no plugins page, so this opens Settings on its current page.
            PluginHostEvent::OpenPluginsPage => self.open_settings(window, cx),
            PluginHostEvent::OpenNewWorkspace(request) => {
                let preset = NewWorkspacePreset {
                    cwd: Some(request.cwd.clone()),
                    project_id: request.project_id.clone(),
                    title: request.name.clone(),
                };
                self.open_new_workspace(preset, window, cx);
            }
            PluginHostEvent::OpenAgent { agent_id }
            | PluginHostEvent::OpenAgentAt {
                agent_id,
                anchor: AgentAnchor::Latest,
            } => self.open_agent(agent_id, window, cx),
            // Electron opens a Board card at `latest-prompt`.
            PluginHostEvent::OpenAgentAt {
                agent_id,
                anchor: AgentAnchor::LatestPrompt,
            } => {
                self.open_agent(agent_id, window, cx);
                self.reveal_latest_prompt(agent_id, window, cx);
            }
            PluginHostEvent::OpenWorkspace { workspace_id } => {
                self.open_workspace(workspace_id.clone(), window, cx)
            }
            // The app opens the agent tab; its timeline scrolls to the passage.
            PluginHostEvent::RevealTimelinePassage(passage) => {
                self.open_agent(&passage.agent_id, window, cx);
                if let Some(timeline) = self.agent_timeline(&passage.agent_id, cx) {
                    timeline.update(cx, |timeline, cx| {
                        timeline.reveal_passage(
                            &passage.message_id,
                            passage.text.as_deref(),
                            window,
                            cx,
                        )
                    });
                }
            }
            _ => {}
        }
    }

    /// Shows a plugin surface and marks its sidebar row.
    fn open_surface(&mut self, surface: OpenSurface, window: &mut Window, cx: &mut Context<Self>) {
        self.settings = None;
        self.set_surface(Some(surface), cx);
        window.focus(&self.focus, cx);
        cx.notify();
    }

    /// Sets the open surface. The sidebar marks the row of a page surface, as Electron marks the
    /// row whose route is open; an overlay is not a route.
    fn set_surface(&mut self, surface: Option<OpenSurface>, cx: &mut Context<Self>) {
        let active = surface
            .as_ref()
            .filter(|open| open.presentation == SurfacePresentation::Page)
            .map(|open| (open.plugin_id.clone(), open.surface_id.clone()));
        self.surface = surface;
        self.sidebar
            .update(cx, |sidebar, cx| sidebar.set_active_surface(active, cx));
    }

    /// Closes a page surface, so a workspace or agent that opens shows in front of it.
    fn close_page_surface(&mut self, cx: &mut Context<Self>) {
        if self
            .surface
            .as_ref()
            .is_some_and(|open| closes_surface_on_open(open.presentation))
        {
            self.set_surface(None, cx);
        }
    }

    /// Scrolls the agent's open tab to its last user message. Before the tab is built or its
    /// timeline loads, the reveal waits for the agent's next timeline change; with no user
    /// message the tail stays.
    fn reveal_latest_prompt(
        &mut self,
        agent_id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // A lazy tab is built on the activation event, after this update.
        let Some(timeline) = self.agent_timeline(agent_id, cx) else {
            let identity = agent_key(agent_id);
            let has_tab = self.workspaces.values().any(|open| {
                open.panes
                    .values()
                    .any(|pane| pane.identity.as_deref() == Some(identity.as_str()))
            });
            if has_tab {
                self.pending_prompt = Some(agent_id.to_string());
            }
            return;
        };
        match latest_prompt_reveal(self.client.read(cx).timeline(agent_id)) {
            PromptReveal::Wait => self.pending_prompt = Some(agent_id.to_string()),
            PromptReveal::Tail => {}
            PromptReveal::Reveal(message_id) => timeline.update(cx, |timeline, cx| {
                timeline.reveal_passage(&message_id, None, window, cx)
            }),
        }
    }

    /// The timeline of the open tab for an agent, in any workspace.
    fn agent_timeline(&self, agent_id: &str, cx: &App) -> Option<Entity<TimelineView>> {
        let identity = agent_key(agent_id);
        self.workspaces
            .values()
            .flat_map(|open| open.panes.values())
            .find_map(|pane| match &pane.content {
                PaneContent::Agent(view) if pane.identity.as_deref() == Some(identity.as_str()) => {
                    Some(view.read(cx).timeline.clone())
                }
                _ => None,
            })
    }

    /// `openPanel`: a workspace tab with the panel identity, focused when already open. The
    /// explorer has no plugin slot, so the `Explorer` location also opens a workspace tab.
    fn open_plugin_panel(
        &mut self,
        plugin_id: &'static str,
        panel_id: &str,
        target: &PanelTarget,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(host) = self.plugin_host.clone() else {
            return;
        };
        let panel = host
            .read(cx)
            .index()
            .workspace_panels()
            .iter()
            .find(|entry| entry.plugin_id == plugin_id && entry.value.id == panel_id)
            .map(|entry| entry.value.clone());
        let Some(panel) = panel else {
            let notice = format!("Workspace panel {panel_id} is unavailable");
            window.push_notification(Notification::error(notice), cx);
            return;
        };
        if let Err(notice) = panel_location(panel.id, &panel.locations, target.location) {
            window.push_notification(Notification::error(notice), cx);
            return;
        }
        let workspace_id = target.workspace_id.clone();
        if self.selected != workspace_id {
            self.open_workspace(workspace_id.clone(), window, cx);
        }
        if !self.workspaces.contains_key(&workspace_id) {
            return;
        }
        let identity = panel_identity(
            plugin_id,
            panel.id,
            panel.context,
            target.agent_id.as_deref(),
        );
        if let Some(key) = self.find_identity(&workspace_id, &identity) {
            self.activate_key(&workspace_id, &key, window, cx);
            return;
        }
        let input = PanelInput {
            workspace_id: workspace_id.clone(),
            agent_id: target.agent_id.clone(),
        };
        let view = (panel.view)(input, window, cx);
        let key = self.next_key();
        self.put_pane(
            &workspace_id,
            key,
            Some(identity),
            panel.title.clone(),
            panel.icon,
            PaneContent::Plugin(view),
            Vec::new(),
            window,
            cx,
        );
    }

    fn close_surface(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.surface.is_some() {
            self.set_surface(None, cx);
            self.refocus(window, cx);
            cx.notify();
        }
    }

    /// Escape closes a surface; a free key runs the first plugin shortcut with that combo, or
    /// closes the shortcut's surface when it shows. Host bindings run before this listener.
    fn on_key_capture(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let keystroke = &event.keystroke;
        let modifiers = &keystroke.modifiers;
        if self.surface.is_some() && keystroke.key == "escape" && !modifiers.modified() {
            self.close_surface(window, cx);
            cx.stop_propagation();
            return;
        }
        let Some(host) = self.plugin_host.clone() else {
            return;
        };
        let hit = {
            let shortcuts = host.read(cx).index().shortcuts();
            if shortcuts.is_empty() {
                return;
            }
            let pressed = keystroke_text(
                modifiers.control,
                modifiers.alt,
                modifiers.shift,
                modifiers.platform,
                &keystroke.key,
            );
            let combos: Vec<&str> = shortcuts.iter().map(|entry| entry.value.combo).collect();
            plugin_shortcut(&self.shortcuts, &combos, &pressed).map(|ix| shortcuts[ix].clone())
        };
        let Some(entry) = hit else {
            return;
        };
        cx.stop_propagation();
        let shows = entry.value.surface.is_some_and(|surface| {
            self.surface
                .as_ref()
                .is_some_and(|open| open.plugin_id == entry.plugin_id && open.surface_id == surface)
        });
        if shows {
            self.close_surface(window, cx);
        } else {
            let workspace = (self.selected != HOME).then(|| self.selected.clone());
            (entry.value.on_press)(workspace, window, cx);
        }
    }

    /// The open surface as a page or an overlay, with its view built once per input.
    fn render_surface(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<(SurfacePresentation, AnyElement)> {
        let open = self.surface.clone()?;
        let surface = self.plugin_host.as_ref().and_then(|host| {
            host.read(cx)
                .index()
                .surface(&open.plugin_id, &open.surface_id)
                .map(|entry| entry.value.clone())
        });
        let title = surface_heading(
            surface.as_ref().map(|surface| {
                (
                    surface.title.as_ref().map(|title| title.as_ref()),
                    surface.id,
                )
            }),
            &open.plugin_id,
        );
        let glyph = surface.as_ref().and_then(|surface| surface.icon);
        let view = surface
            .as_ref()
            .map(|surface| self.plugin_views.surface(surface, &open, window, cx));
        let weak = cx.weak_entity();
        let close = move |window: &mut Window, cx: &mut App| {
            let _ = weak.update(cx, |shell, cx| shell.close_surface(window, cx));
        };
        let page = open.presentation == SurfacePresentation::Page;
        let inset = if page && !self.layout.sidebar_open {
            TRAFFIC_LIGHT_WIDTH
        } else {
            0.
        };
        let header = surface_header(title.into(), glyph, close.clone(), inset, cx);
        let body = surface_body(view, cx);
        let element = if page {
            let header = div()
                .relative()
                .flex_shrink_0()
                .child(self.drag_area("plugin-surface-drag", cx))
                .child(header)
                .into_any_element();
            surface_page(header, body, cx)
        } else {
            surface_overlay(header.into_any_element(), body, close, cx)
        };
        Some((open.presentation, element))
    }

    /// The New workspace screen of the active tab, if it shows.
    pub(crate) fn active_new_workspace(&self, cx: &App) -> Option<Entity<NewWorkspaceScreen>> {
        let key = self.active_key(cx)?;
        match &self.pane(&self.selected, &key)?.content {
            PaneContent::NewWorkspace(view) => Some(view.clone()),
            _ => None,
        }
    }

    /// The header toggle: shows or hides the side panel of every New workspace screen.
    pub(crate) fn toggle_side_panel(&mut self, cx: &mut Context<Self>) {
        self.side_panel_open = !self.side_panel_open;
        let open = self.side_panel_open;
        let screens: Vec<Entity<NewWorkspaceScreen>> = self
            .workspaces
            .values()
            .flat_map(|workspace| workspace.panes.values())
            .filter_map(|pane| match &pane.content {
                PaneContent::NewWorkspace(view) => Some(view.clone()),
                _ => None,
            })
            .collect();
        for screen in screens {
            screen.update(cx, |screen, cx| screen.set_side_panel_open(open, cx));
        }
        cx.notify();
    }

    /// The first header subtitle contribution, when the workspace has a project id.
    pub(crate) fn plugin_subtitle(
        &mut self,
        input: Option<(String, String, String, String)>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyView> {
        let host = self.plugin_host.clone()?;
        let entry = host.read(cx).index().header_subtitles().first().cloned();
        let (workspace_id, project_id, name, root) = input?;
        let input = crate::plugins::subtitle_input(
            entry.is_some(),
            &workspace_id,
            &project_id,
            &name,
            &root,
        )?;
        let entry = entry?;
        Some(
            self.plugin_views
                .subtitle(entry.plugin_id, &entry.value, input, window, cx),
        )
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
            cx.subscribe_in(&timeline, window, {
                let workspace = workspace.clone();
                move |this: &mut Shell, timeline, event: &TimelineForkEvent, window, cx| {
                    this.fork(&workspace, timeline.clone(), event, window, cx)
                }
            }),
            cx.subscribe_in(&composer, window, {
                let key = key.clone();
                move |this: &mut Shell, _, event: &ComposerEvent, window, cx| match event {
                    ComposerEvent::ReplaceWithDraft => {
                        this.open_draft(&workspace, Some(key.clone()), window, cx)
                    }
                    ComposerEvent::OpenChanges { workspace_id } => {
                        this.open_workspace_changes(workspace_id, window, cx)
                    }
                    ComposerEvent::OpenAgent { agent_id } => this.open_agent(agent_id, window, cx),
                    _ => {}
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
            move |this: &mut Shell, _, event: &TerminalEvent, window, cx| match event {
                TerminalEvent::Opened { id } => this.set_pane_kind(
                    &workspace,
                    &pane_key,
                    TabKind::Terminal { id: id.clone() },
                    cx,
                ),
                TerminalEvent::OpenFile { cwd, path, line } => {
                    this.open_file(cwd, path, *line, window, cx)
                }
                TerminalEvent::Exited => {}
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

    /// Scripts "View terminal": shows the tab of the script's daemon terminal, or attaches a new
    /// tab to it in the selected workspace.
    pub(crate) fn view_script_terminal(
        &mut self,
        name: &str,
        terminal_id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let workspace_id = self.selected.clone();
        self.header_menu = None;
        let open_key = self.workspaces.get(&workspace_id).and_then(|open| {
            open.panes
                .iter()
                .find_map(|(key, pane)| match &pane.content {
                    PaneContent::Terminal(view)
                        if view.read(cx).terminal_id() == Some(terminal_id) =>
                    {
                        Some(key.clone())
                    }
                    _ => None,
                })
        });
        if let Some(key) = open_key {
            self.activate_key(&workspace_id, &key, window, cx);
            cx.notify();
            return;
        }
        let key = self.next_key();
        let client = self.client.clone();
        let (id, title) = (terminal_id.to_string(), name.to_string());
        let view = cx.new(|cx| {
            let mut terminal = TerminalView::attach(client, id, window, cx);
            terminal.set_name(title, cx);
            terminal
        });
        let workspace = workspace_id.clone();
        let pane_key = key.clone();
        let subscription = cx.subscribe_in(
            &view,
            window,
            move |this: &mut Shell, _, event: &TerminalEvent, window, cx| match event {
                TerminalEvent::Opened { id } => this.set_pane_kind(
                    &workspace,
                    &pane_key,
                    TabKind::Terminal { id: id.clone() },
                    cx,
                ),
                TerminalEvent::OpenFile { cwd, path, line } => {
                    this.open_file(cwd, path, *line, window, cx)
                }
                TerminalEvent::Exited => {}
            },
        );
        self.put_pane(
            &workspace_id,
            key,
            None,
            name.to_string().into(),
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

    /// Opens an agent's tab in its workspace, from the command center, a notification, or a
    /// plugin such as the Board. An open page surface closes, as `open_workspace` does.
    pub(crate) fn open_agent(
        &mut self,
        agent_id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (live, archived) = {
            let client = self.client.read(cx);
            (
                agent_workspace(client.projects(), client.agents(), agent_id),
                client
                    .archived_agent(agent_id)
                    .map(|agent| agent.workspace_id.clone()),
            )
        };
        match agent_open(live, archived) {
            AgentOpen::In(workspace_id) => {
                self.open_agent_in(agent_id, workspace_id.as_deref(), window, cx)
            }
            AgentOpen::Fetch => {
                self.pending_prompt = None;
                self.open_fetched_agent(agent_id, window, cx);
            }
        }
    }

    /// Fetches an agent the client does not know and opens it when it is archived. The agent
    /// subscription has no `includeArchived`, so the daemon sends `remove` on an archive and the
    /// client never sees the archived agent; a stale Board card of that agent lands here.
    fn open_fetched_agent(&mut self, agent_id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let task = self
            .client
            .update(cx, |client, cx| client.fetch_agent(agent_id, cx));
        let agent_id = agent_id.to_string();
        cx.spawn_in(window, async move |this, cx| {
            if let Err(error) = task.await {
                return log::warn!("cannot open agent {agent_id}: {error:#}");
            }
            let _ = this.update_in(cx, |shell, window, cx| {
                let archived = shell
                    .client
                    .read(cx)
                    .archived_agent(&agent_id)
                    .map(|agent| agent.workspace_id.clone());
                if let Some(workspace_id) = archived {
                    shell.open_agent_in(&agent_id, workspace_id.as_deref(), window, cx);
                }
            });
        })
        .detach();
    }

    /// Opens an agent's tab in `workspace_id` without the tree lookup, as a History row does
    /// in Electron. A workspace the tree does not list, or none, opens the tab in the home view.
    pub(crate) fn open_agent_in(
        &mut self,
        agent_id: &str,
        workspace_id: Option<&str>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // A later open cancels a reveal that still waits.
        self.pending_prompt = None;
        let target = agent_target(self.client.read(cx).projects(), workspace_id);
        let workspace_id = match target {
            Some(workspace_id) => {
                self.open_workspace(workspace_id.clone(), window, cx);
                workspace_id
            }
            None => {
                self.ensure_home(window, cx);
                self.close_page_surface(cx);
                HOME.to_string()
            }
        };
        match self.find_identity(&workspace_id, &agent_key(agent_id)) {
            Some(key) => self.activate_key(&workspace_id, &key, window, cx),
            None => self.open_agent_pane(&workspace_id, None, agent_id, window, cx),
        }
        if workspace_id == HOME {
            self.select(HOME.to_string(), window, cx);
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
        let selected = self.selected.clone();
        self.archive_workspace_id(&selected, window, cx);
    }

    /// The archive flow for one workspace, also run by the git split button.
    fn archive_workspace_id(
        &mut self,
        workspace_id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((id, name, cwd)) = ({
            let client = self.client.read(cx);
            find_workspace(client.projects(), workspace_id)
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
        let (workspace, agent) = self.plugin_active.clone();
        center.update(cx, |center, cx| {
            center.set_plugin_context(workspace, agent, cx)
        });
        let subscriptions = vec![
            cx.subscribe_in(
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
            ),
            cx.subscribe_in(
                &center,
                window,
                |this: &mut Shell, center, event: &CommandFileEvent, window, cx| match event {
                    CommandFileEvent::FileQuery { query } => {
                        this.query_files(center.clone(), query, cx)
                    }
                    CommandFileEvent::OpenFile { path } => {
                        this.command = None;
                        if let Some(cwd) = this.selected_cwd().map(str::to_string) {
                            this.open_file(&cwd, path, None, window, cx);
                        }
                        cx.notify();
                    }
                },
            ),
        ];
        self.command = Some((center, subscriptions));
        cx.notify();
    }

    /// Answers a file query with the files of the selected workspace, each with its file icon.
    /// An answer to an older query, or to a closed command center, is dropped.
    fn query_files(&mut self, center: Entity<CommandCenter>, query: &str, cx: &mut Context<Self>) {
        self.file_query += 1;
        let generation = self.file_query;
        let Some(cwd) = self.selected_cwd().map(str::to_string) else {
            center.update(cx, |center, cx| center.set_file_rows(Vec::new(), cx));
            return;
        };
        let task = self.client.update(cx, |client, cx| {
            client.directory_suggestions(&cwd, query, true, false, 100, cx)
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |shell, cx| {
                let open = shell
                    .command
                    .as_ref()
                    .is_some_and(|(open, _)| *open == center);
                if shell.file_query != generation || !open {
                    return;
                }
                center.update(cx, |center, cx| match result {
                    Ok(found) => {
                        let rows = found
                            .into_iter()
                            .filter(|entry| !entry.is_directory)
                            .map(|entry| FileResult {
                                icon: file_icon(&file_name(&entry.path)),
                                path: entry.path,
                            })
                            .collect();
                        center.set_file_rows(rows, cx)
                    }
                    Err(error) => center.set_file_error(Some(format!("{error:#}")), cx),
                });
            });
        })
        .detach();
    }

    /// `shortcuts.dialog.toggle`: shows the shortcuts dialog, or hides it when it shows.
    fn toggle_shortcuts_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.shortcuts_dialog.take().is_some() {
            self.refocus(window, cx);
            return;
        }
        let dialog = cx.new(|cx| ShortcutsDialog::new(window, cx));
        let subscription = cx.subscribe_in(
            &dialog,
            window,
            |this: &mut Shell, _, event: &ShortcutsDialogEvent, window, cx| {
                let ShortcutsDialogEvent::Dismissed = event;
                this.shortcuts_dialog = None;
                this.refocus(window, cx);
                cx.notify();
            },
        );
        self.shortcuts_dialog = Some((dialog, subscription));
    }

    /// The New workspace screen, on the project when one is given. A page surface closes, as
    /// for a route change.
    fn new_workspace(
        &mut self,
        project_id: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let preset = NewWorkspacePreset {
            project_id,
            ..NewWorkspacePreset::default()
        };
        self.open_new_workspace(preset, window, cx);
        self.close_page_surface(cx);
    }

    /// `agent.new` ("Open project"): a folder dialog, then `add_project`, then the New
    /// workspace screen on the added project, as Electron's add project flow ends.
    fn open_project(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let picked = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Open project".into()),
        });
        let client = self.client.clone();
        cx.spawn_in(window, async move |this, cx| {
            let Ok(Ok(Some(paths))) = picked.await else {
                return;
            };
            let Some(path) = paths.first().map(|path| path.to_string_lossy().to_string()) else {
                return;
            };
            let Ok(task) = cx.update(|_, cx| client.update(cx, |c, cx| c.add_project(&path, cx)))
            else {
                return;
            };
            let result = task.await;
            let _ = this.update_in(cx, |shell, window, cx| match result {
                Ok(added) => shell.new_workspace(Some(added.id), window, cx),
                Err(error) => shell.show_toast(false, format!("{error:#}"), cx),
            });
        })
        .detach();
    }

    /// `workspace.pin`: pins the selected workspace, or unpins it.
    fn toggle_pin(&mut self, cx: &mut Context<Self>) {
        let pinned = find_workspace(self.client.read(cx).projects(), &self.selected)
            .map(|(_, workspace)| workspace.pinned);
        if let Some(pinned) = pinned {
            let id = self.selected.clone();
            self.client
                .update(cx, |client, cx| client.pin_workspace(&id, !pinned, cx));
        }
    }

    /// Explorer "Open to the side": the file in the group right of the focused one, or in a
    /// new group split to the right. An open file tab only shows.
    fn open_file_to_side(
        &mut self,
        cwd: &str,
        path: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let selected = self.selected.clone();
        if self
            .find_identity(&selected, &file_key(cwd, path))
            .is_some()
        {
            self.open_file(cwd, path, None, window, cx);
            return;
        }
        let right = self
            .workspaces
            .get(&selected)
            .and_then(|open| neighbor_active_key(open.view.read(cx), "right"));
        match right {
            Some(key) => {
                self.activate_key(&selected, &key, window, cx);
                self.open_file(cwd, path, None, window, cx);
            }
            None => {
                self.side_file = Some((selected.clone(), cwd.to_string(), path.to_string()));
                self.with_workspace(
                    &selected,
                    cx,
                    |workspace, window, cx| workspace.split(SplitAxis::Right, window, cx),
                    window,
                );
            }
        }
    }

    /// Explorer "Add to chat", as Electron's `useAddFileToChat`: a workspace-file chip in the
    /// composer of the chat target (`chat_target`), then that tab takes the focus. A lazy agent
    /// tab gets its composer first. With no chat target the menu hides the item.
    fn add_file_to_chat(
        &mut self,
        _cwd: &str,
        path: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let workspace_id = self.selected.clone();
        let Some(key) = self.chat_target_key(cx) else {
            return;
        };
        self.materialize(&workspace_id, &key, window, cx);
        let (label, attachment) = workspace_file_attachment(path);
        match self.pane(&workspace_id, &key).map(|pane| &pane.content) {
            Some(PaneContent::Agent(view)) => {
                let composer = view.read(cx).composer.clone();
                composer.update(cx, |composer, cx| {
                    composer.add_raw_attachment(label, attachment, cx)
                });
            }
            // The draft pane keeps its composer private; `seed_fork` without a seed only adds
            // the chip through `Composer::add_raw_attachment`.
            Some(PaneContent::Draft(view)) => view.update(cx, |draft, cx| {
                draft.seed_fork(None, label, attachment, window, cx)
            }),
            _ => return,
        }
        self.activate_key(&workspace_id, &key, window, cx);
        self.focus_pane(&workspace_id, &key, window, cx);
    }

    /// The tab key of the selected workspace's chat target, from its focused tab and the active
    /// tab of each group.
    fn chat_target_key(&self, cx: &App) -> Option<String> {
        let open = self.workspaces.get(&self.selected)?;
        let workspace = open.view.read(cx);
        let layout = workspace.layout();
        let actives: Vec<&str> = layout
            .groups()
            .into_iter()
            .filter_map(|group| group.active_item().map(|item| item.key.as_str()))
            .collect();
        let is_chat = |key: &str| {
            matches!(
                open.panes.get(key).map(|pane| &pane.content),
                Some(PaneContent::Agent(_) | PaneContent::LazyAgent(_) | PaneContent::Draft(_))
            )
        };
        chat_target(layout.active_key(), &actives, is_chat).map(str::to_string)
    }

    /// Tells the Explorer whether "Add to chat" has a target; it ignores an unchanged value.
    fn sync_chat_target(&mut self, cx: &mut Context<Self>) {
        let Some(explorer) = self.explorer.clone() else {
            return;
        };
        let available = self.chat_target_key(cx).is_some();
        explorer.update(cx, |explorer, cx| explorer.set_chat_target(available, cx));
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
            "workspace.new" => self.new_workspace(None, window, cx),
            "agent.new" => self.open_project(window, cx),
            "command-center.files" => {
                self.open_command_center(window, cx);
                if let Some(center) = self.command.as_ref().map(|(center, _)| center.clone()) {
                    center.update(cx, |center, cx| center.open_files_scope(window, cx));
                }
            }
            "workspace.pin" => self.toggle_pin(cx),
            "shortcuts.dialog.toggle" => self.toggle_shortcuts_dialog(window, cx),
            "settings.toggle" => match self.settings.take() {
                Some(_) => self.refocus(window, cx),
                None => self.open_settings(window, cx),
            },
            "theme.cycle" => {
                // The cycle also clears a selected plugin theme, so the shortcut always shows
                // a change (Q-103).
                let choice = bachuc_theme::cycle_builtin_theme(cx);
                let dir = data_dir();
                let stored = bachuc_settings::appearance::load_from(&dir);
                let appearance = bachuc_settings::appearance::with_builtin_theme(&stored, choice);
                if let Err(error) = bachuc_settings::appearance::save_to(&dir, &appearance) {
                    log::warn!("cannot save appearance.json: {error}");
                }
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
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
                .capture_key_down(cx.listener(Self::on_key_capture))
                .size_full()
                .bg(t.surface0)
                .text_color(t.foreground)
                .child(self.render_settings(settings, cx))
                .when_some(self.shortcuts_dialog.as_ref(), |root, (dialog, _)| {
                    root.child(dialog.clone())
                })
                .into_any_element();
        }
        let sidebar = self
            .layout
            .sidebar_open
            .then(|| self.render_sidebar_column(cx));
        let (page, overlay) = match self.render_surface(window, cx) {
            Some((SurfacePresentation::Page, page)) => (Some(page), None),
            Some((_, overlay)) => (None, Some(overlay)),
            None => (None, None),
        };
        // A page replaces the workspace, as Settings does, so the workspace views stay unbuilt.
        let main = match page {
            Some(page) => page,
            None => {
                let header = self.render_header(window, cx);
                let center = self.render_center(cx);
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
                    .child(div().flex_1().min_h_0().w_full().child(center))
                    .into_any_element()
            }
        };
        let explorer = if self.surface.is_some() && overlay.is_none() {
            None
        } else {
            self.render_explorer(cx)
        };
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
            .capture_key_down(cx.listener(Self::on_key_capture))
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
                    .child(main)
                    .children(explorer),
            )
            .children(overlay)
            .when_some(self.command.as_ref(), |root, (center, _)| {
                root.child(center.clone())
            })
            .when_some(self.shortcuts_dialog.as_ref(), |root, (dialog, _)| {
                root.child(dialog.clone())
            })
            .children(render_toasts(&self.toasts, cx))
            .into_any_element()
    }
}

pub(crate) const TERMINAL_NOT_OPEN: &str = "The terminal is not open yet";
pub(crate) const CANNOT_RENAME: &str = "This tab cannot be renamed";
pub(crate) const RESUME_ID_UNAVAILABLE: &str = "Resume ID not available";
pub(crate) const RESUME_COMMAND_UNAVAILABLE: &str = "Resume command not available";
pub(crate) const RELOAD_FAILED: &str = "Failed to reload agent";

/// What Rename does on a tab.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RenameTarget {
    Agent(String),
    Terminal(String),
    Notice(&'static str),
}

/// An agent tab renames the agent; a terminal tab renames its daemon terminal once it is open.
/// `terminal` is `Some` for a terminal tab, holding its id once known.
pub(crate) fn rename_target(agent: Option<&str>, terminal: Option<Option<String>>) -> RenameTarget {
    match (agent, terminal) {
        (Some(agent_id), _) => RenameTarget::Agent(agent_id.to_string()),
        (None, Some(Some(terminal_id))) => RenameTarget::Terminal(terminal_id),
        (None, Some(None)) => RenameTarget::Notice(TERMINAL_NOT_OPEN),
        (None, None) => RenameTarget::Notice(CANNOT_RENAME),
    }
}

/// `handleCopyResumeCommand`: the command, or the notice for a missing session id or template.
pub(crate) fn resume_text(
    provider: Option<&str>,
    session_id: Option<&str>,
) -> Result<String, &'static str> {
    let session_id = session_id.ok_or(RESUME_ID_UNAVAILABLE)?;
    provider
        .and_then(|provider| resume_command(provider, session_id))
        .ok_or(RESUME_COMMAND_UNAVAILABLE)
}

/// The notice of a failed reload: its text, or the Electron fallback when it is empty.
pub(crate) fn reload_error(message: &str) -> String {
    let message = message.trim();
    if message.is_empty() {
        RELOAD_FAILED.to_string()
    } else {
        message.to_string()
    }
}

/// A page surface replaces the workspace, so opening a workspace or an agent closes it, as an
/// Electron route change does. An overlay floats over the workspace and stays.
pub(crate) fn closes_surface_on_open(presentation: SurfacePresentation) -> bool {
    presentation == SurfacePresentation::Page
}

/// The workspace that holds an agent: the tree's, else the agent's own `workspace_id` when the
/// tree lists that workspace.
pub(crate) fn agent_workspace(
    projects: &[ProjectNode],
    agents: &[AgentSummary],
    agent_id: &str,
) -> Option<String> {
    let workspaces = || projects.iter().flat_map(|project| &project.workspaces);
    if let Some(workspace) =
        workspaces().find(|workspace| workspace.agent_ids.iter().any(|id| id == agent_id))
    {
        return Some(workspace.id.clone());
    }
    let own = agents
        .iter()
        .find(|agent| agent.id == agent_id)?
        .workspace_id
        .as_deref()?;
    workspaces()
        .any(|workspace| workspace.id == own)
        .then(|| own.to_string())
}

/// How `open_agent` opens an agent.
#[derive(Debug, PartialEq)]
pub(crate) enum AgentOpen {
    /// In this workspace; `None` or a workspace the tree lacks is the home view.
    In(Option<String>),
    /// The client knows neither a live nor an archived agent: fetch it first.
    Fetch,
}

/// A live agent opens in its tree workspace; an archived agent opens in its own workspace. An
/// agent the client does not know is fetched, because an archived one may only be stale.
pub(crate) fn agent_open(live: Option<String>, archived: Option<Option<String>>) -> AgentOpen {
    match (live, archived) {
        (Some(workspace_id), _) => AgentOpen::In(Some(workspace_id)),
        (None, Some(workspace_id)) => AgentOpen::In(workspace_id),
        (None, None) => AgentOpen::Fetch,
    }
}

/// Where `open_agent_in` opens a tab: the given workspace when the tree lists it, else the
/// home view (`None`), the Q-114 rule.
pub(crate) fn agent_target(projects: &[ProjectNode], workspace_id: Option<&str>) -> Option<String> {
    let workspace_id = workspace_id?;
    find_workspace(projects, workspace_id).map(|_| workspace_id.to_string())
}

/// `getPullRequestIdentity`: a status with a URL or a number is a PR.
pub(crate) fn has_pull_request(status: Option<&PrStatus>) -> bool {
    status.is_some_and(|status| !status.url.trim().is_empty() || status.number.is_some())
}

/// The Pull request tab opens by itself only for the first PR found in a workspace this
/// session, and only when no PR tab is open there. A closed tab does not come back.
pub(crate) fn auto_opens_pr_tab(
    status: Option<&PrStatus>,
    found_before: bool,
    has_tab: bool,
) -> bool {
    has_pull_request(status) && !found_before && !has_tab
}

/// `message.actions.forkFailed`.
pub(crate) const FORK_FAILED: &str = "Failed to fork chat";
/// `message.actions.forkMissingWorkspace`.
pub(crate) const FORK_MISSING_WORKSPACE: &str = "This agent is not in a workspace.";
/// The chip label of a chat history without a title, as Electron's text attachment.
pub(crate) const FORK_ATTACHMENT_LABEL: &str = "Text attachment";

/// Where a fork draft opens, decided before the chat history request.
enum ForkOpen {
    /// A draft tab in this folder, in the source tab's workspace.
    Tab(String),
    Workspace(NewWorkspacePreset),
}

/// `buildForkDraftSetup`: the draft settings of a fork, from the source agent. An agent with no
/// provider gives none, and the draft keeps its own.
pub(crate) fn fork_draft_seed(
    agent: Option<&AgentSummary>,
    controls: Option<&AgentControls>,
    thinking: Option<&ThinkingControls>,
    features: &[AgentFeature],
) -> Option<DraftSeed> {
    let agent = agent.filter(|agent| !agent.provider.is_empty())?;
    let values = features
        .iter()
        .map(|feature| {
            let value = match &feature.kind {
                AgentFeatureKind::Toggle { value } => serde_json::json!(value),
                AgentFeatureKind::Select { value, .. } => serde_json::json!(value),
            };
            (feature.id.clone(), value)
        })
        .collect();
    Some(DraftSeed {
        provider: Some(agent.provider.clone()),
        model: agent
            .model
            .clone()
            .or_else(|| controls.and_then(|controls| controls.current_model.clone())),
        mode: controls.and_then(|controls| controls.current_mode.clone()),
        thinking_option_id: thinking.and_then(|thinking| thinking.current.clone()),
        features: Some(serde_json::Value::Object(values)),
    })
}

/// The folder of a fork draft tab: the agent's own, as the draft setup's `cwd`, else the
/// workspace's. A tab outside a workspace gets `forkMissingWorkspace`.
pub(crate) fn fork_tab_cwd(
    workspace_id: &str,
    agent_cwd: &str,
    workspace_cwd: Option<&str>,
) -> Result<String, &'static str> {
    if workspace_id == HOME {
        return Err(FORK_MISSING_WORKSPACE);
    }
    [Some(agent_cwd), workspace_cwd]
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|cwd| !cwd.is_empty())
        .map(str::to_string)
        .ok_or(FORK_MISSING_WORKSPACE)
}

/// The New workspace screen of a fork: the source workspace's folder and project, else the
/// agent's folder (`checkout.cwd || cwd`). The route's display name is the project's name, which
/// the screen shows from the project, so the title stays unset.
pub(crate) fn fork_workspace_preset(
    projects: &[ProjectNode],
    workspace_id: Option<&str>,
    agent_cwd: &str,
) -> NewWorkspacePreset {
    let found = workspace_id.and_then(|id| find_workspace(projects, id));
    let cwd = [
        found.map(|(_, workspace)| workspace.cwd.as_str()),
        Some(agent_cwd),
    ]
    .into_iter()
    .flatten()
    .map(str::trim)
    .find(|cwd| !cwd.is_empty())
    .map(str::to_string);
    NewWorkspacePreset {
        cwd,
        project_id: found.map(|(project, _)| project.id.clone()),
        title: None,
    }
}

/// The chip label of a fork's chat history: the attachment title, else the fallback.
pub(crate) fn fork_attachment_label(title: Option<&str>) -> String {
    title
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .unwrap_or(FORK_ATTACHMENT_LABEL)
        .to_string()
}

/// The tab that "Add to chat" attaches to, as Electron's `resolveFocusedChatTarget`: the focused
/// tab when it is a chat (agent, lazy agent, or draft), else the active tab of the first group in
/// reading order that is a chat. GPUI tabs record no parent tab, so Electron's middle step (the
/// chat a file was opened beside) has nothing to read.
pub(crate) fn chat_target<'a>(
    focused: Option<&'a str>,
    group_actives: &[&'a str],
    is_chat: impl Fn(&str) -> bool,
) -> Option<&'a str> {
    focused
        .filter(|key| is_chat(key))
        .or_else(|| group_actives.iter().copied().find(|key| is_chat(key)))
}

/// The chip label and attachment of a workspace file, as Electron's
/// `workspaceFileAttachmentToAgentAttachment` for a whole file.
pub(crate) fn workspace_file_attachment(path: &str) -> (String, serde_json::Value) {
    let name = path.rsplit('/').next().unwrap_or(path).to_string();
    let attachment = serde_json::json!({
        "type": "text",
        "mimeType": "text/plain",
        "title": name,
        "text": format!("Workspace file: {path}"),
    });
    (name, attachment)
}

/// The toast of a failed fork: the error, as `toErrorMessage`, but `forkFailed` for an empty
/// context (the client's "Nothing to fork") or an empty text.
pub(crate) fn fork_error(message: &str) -> String {
    let message = message.trim();
    if message.is_empty() || message == "Nothing to fork" {
        FORK_FAILED.to_string()
    } else {
        message.to_string()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PromptReveal {
    /// The timeline has no entries yet; try again on its next change.
    Wait,
    /// No user message is loaded; keep the tail view.
    Tail,
    /// Scroll to this message key.
    Reveal(String),
}

/// Where `AgentAnchor::LatestPrompt` scrolls: the last loaded user message. A state with no
/// entries waits, because a permission change can come before the first page.
pub(crate) fn latest_prompt_reveal(state: Option<&TimelineState>) -> PromptReveal {
    let Some(state) = state.filter(|state| state.version > 0 && !state.entries.is_empty()) else {
        return PromptReveal::Wait;
    };
    state
        .entries
        .iter()
        .rev()
        .find(|entry| item_type(entry) == "user_message")
        .map_or(PromptReveal::Tail, |entry| {
            PromptReveal::Reveal(message_key(entry))
        })
}

#[cfg(test)]
mod tests {
    // Not `super::*`: the gpui glob brings a `test` macro that shadows the std one.
    use super::{
        AgentOpen, CANNOT_RENAME, FORK_ATTACHMENT_LABEL, FORK_FAILED, FORK_MISSING_WORKSPACE,
        PromptReveal, RELOAD_FAILED, RESUME_COMMAND_UNAVAILABLE, RESUME_ID_UNAVAILABLE,
        RenameTarget, TERMINAL_NOT_OPEN, agent_open, agent_target, agent_workspace,
        auto_opens_pr_tab, chat_target, closes_surface_on_open, fork_attachment_label,
        fork_draft_seed, fork_error, fork_tab_cwd, fork_workspace_preset, has_pull_request,
        latest_prompt_reveal, reload_error, rename_target, resume_text, workspace_file_attachment,
    };
    use bachuc_client::{
        AgentControls, AgentFeature, AgentFeatureKind, AgentSummary, PrMergeCapability, PrStatus,
        ProjectNode, ThinkingControls, TimelineState, WorkspaceNode,
    };
    use bachuc_composer::DraftSeed;

    fn pr(url: &str, number: Option<u64>) -> PrStatus {
        PrStatus {
            number,
            url: url.into(),
            title: "t".into(),
            state: "open".into(),
            is_merged: false,
            is_draft: false,
            mergeable: None,
            checks: Vec::new(),
            review_decision: None,
            repo_owner: "o".into(),
            repo_name: "r".into(),
            merge: PrMergeCapability::default(),
        }
    }

    #[test]
    fn the_pr_tab_opens_once_for_the_first_pr_found() {
        let found = pr("https://github.com/o/r/pull/1", Some(1));
        assert!(auto_opens_pr_tab(Some(&found), false, false));
        // Found before this session: a closed tab stays closed.
        assert!(!auto_opens_pr_tab(Some(&found), true, false));
        // A PR tab is already open in the workspace.
        assert!(!auto_opens_pr_tab(Some(&found), false, true));
        assert!(!auto_opens_pr_tab(None, false, false));
        // A URL or a number alone names a PR; neither is no PR.
        assert!(has_pull_request(Some(&pr("", Some(3)))));
        assert!(has_pull_request(Some(&pr("https://x/pull/3", None))));
        assert!(!has_pull_request(Some(&pr("  ", None))));
        assert!(!auto_opens_pr_tab(Some(&pr("", None)), false, false));
    }
    use bachuc_plugin_host::SurfacePresentation;
    use serde_json::json;

    #[test]
    fn only_a_page_surface_closes_when_an_agent_or_workspace_opens() {
        assert!(closes_surface_on_open(SurfacePresentation::Page));
        assert!(!closes_surface_on_open(SurfacePresentation::Overlay));
    }

    fn workspace(id: &str, agent_ids: &[&str]) -> WorkspaceNode {
        WorkspaceNode {
            id: id.into(),
            name: id.into(),
            cwd: format!("/{id}"),
            branch: None,
            pinned: false,
            agent_ids: agent_ids.iter().map(|id| id.to_string()).collect(),
        }
    }

    fn agent(id: &str, workspace_id: Option<&str>) -> AgentSummary {
        AgentSummary {
            id: id.into(),
            title: id.into(),
            status: "idle".into(),
            cwd: "/w".into(),
            workspace_id: workspace_id.map(str::to_string),
            provider: "claude".into(),
            model: None,
            pending_permissions: 0,
        }
    }

    #[test]
    fn an_agent_opens_in_the_tree_workspace_else_its_own() {
        let projects = vec![ProjectNode {
            id: "p".into(),
            name: "p".into(),
            root: "/p".into(),
            workspaces: vec![workspace("w1", &["a1"]), workspace("w2", &[])],
        }];
        let agents = vec![
            agent("a1", Some("w2")),
            agent("a2", Some("w2")),
            agent("a3", Some("gone")),
            agent("a4", None),
        ];
        assert_eq!(
            agent_workspace(&projects, &agents, "a1").as_deref(),
            Some("w1")
        );
        assert_eq!(
            agent_workspace(&projects, &agents, "a2").as_deref(),
            Some("w2")
        );
        assert_eq!(agent_workspace(&projects, &agents, "a3"), None);
        assert_eq!(agent_workspace(&projects, &agents, "a4"), None);
        assert_eq!(agent_workspace(&projects, &agents, "missing"), None);
    }

    #[test]
    fn a_history_row_opens_in_its_tree_workspace_else_the_home_view() {
        let projects = vec![ProjectNode {
            id: "p".into(),
            name: "p".into(),
            root: "/p".into(),
            workspaces: vec![workspace("w1", &[])],
        }];
        assert_eq!(agent_target(&projects, Some("w1")).as_deref(), Some("w1"));
        assert_eq!(agent_target(&projects, Some("gone")), None);
        assert_eq!(agent_target(&projects, None), None);
        assert_eq!(agent_target(&[], Some("w1")), None);
    }

    #[test]
    fn an_unknown_agent_is_fetched_before_it_opens() {
        let ws = |id: &str| Some(id.to_string());
        assert_eq!(
            agent_open(ws("w1"), Some(ws("w2"))),
            AgentOpen::In(ws("w1"))
        );
        assert_eq!(agent_open(None, Some(ws("w2"))), AgentOpen::In(ws("w2")));
        assert_eq!(agent_open(None, Some(None)), AgentOpen::In(None));
        // An agent archived while its Board card still shows is in neither map.
        assert_eq!(agent_open(None, None), AgentOpen::Fetch);
    }

    #[test]
    fn the_latest_prompt_is_the_last_user_message() {
        assert_eq!(latest_prompt_reveal(None), PromptReveal::Wait);
        let mut state = TimelineState::default();
        assert_eq!(latest_prompt_reveal(Some(&state)), PromptReveal::Wait);
        // A permission can change the state before the first page arrives.
        state.version = 1;
        assert_eq!(latest_prompt_reveal(Some(&state)), PromptReveal::Wait);
        state.entries = vec![json!({"seqStart": 1, "item": {"type": "assistant_message"}})];
        assert_eq!(latest_prompt_reveal(Some(&state)), PromptReveal::Tail);
        state.entries = vec![
            json!({"seqStart": 1, "item": {"type": "user_message", "messageId": "m1"}}),
            json!({"seqStart": 2, "item": {"type": "assistant_message"}}),
            json!({"seqStart": 3, "item": {"type": "user_message"}}),
            json!({"seqStart": 4, "item": {"type": "tool_call"}}),
        ];
        assert_eq!(
            latest_prompt_reveal(Some(&state)),
            PromptReveal::Reveal("seq:3".into())
        );
    }

    #[test]
    fn rename_follows_the_tab_kind() {
        assert_eq!(
            rename_target(Some("a1"), None),
            RenameTarget::Agent("a1".into())
        );
        assert_eq!(
            rename_target(None, Some(Some("t1".into()))),
            RenameTarget::Terminal("t1".into())
        );
        assert_eq!(
            rename_target(None, Some(None)),
            RenameTarget::Notice(TERMINAL_NOT_OPEN)
        );
        // A file, diff, or new tab.
        assert_eq!(
            rename_target(None, None),
            RenameTarget::Notice(CANNOT_RENAME)
        );
    }

    #[test]
    fn the_resume_command_needs_a_session_and_a_template() {
        assert_eq!(
            resume_text(Some("claude"), Some("s-1")).as_deref(),
            Ok("claude --resume s-1")
        );
        assert_eq!(
            resume_text(Some("claude"), None),
            Err(RESUME_ID_UNAVAILABLE)
        );
        assert_eq!(
            resume_text(Some("mock"), Some("s-1")),
            Err(RESUME_COMMAND_UNAVAILABLE)
        );
        assert_eq!(
            resume_text(None, Some("s-1")),
            Err(RESUME_COMMAND_UNAVAILABLE)
        );
    }

    #[test]
    fn a_reload_error_shows_its_text_or_the_fallback() {
        assert_eq!(reload_error("not connected"), "not connected");
        assert_eq!(reload_error(""), RELOAD_FAILED);
        assert_eq!(reload_error("  "), RELOAD_FAILED);
    }

    fn feature(id: &str, kind: AgentFeatureKind) -> AgentFeature {
        AgentFeature {
            id: id.into(),
            label: id.into(),
            description: None,
            tooltip: None,
            icon: None,
            kind,
        }
    }

    #[test]
    fn a_fork_seed_carries_the_source_agent_settings() {
        let mut source = agent("a", Some("w1"));
        source.model = Some("opus".into());
        let controls = AgentControls {
            modes: Vec::new(),
            current_mode: Some("plan".into()),
            models: Vec::new(),
            current_model: Some("sonnet".into()),
            running: false,
        };
        let thinking = ThinkingControls {
            options: Vec::new(),
            current: Some("high".into()),
        };
        let features = vec![
            feature("fast", AgentFeatureKind::Toggle { value: true }),
            feature(
                "tier",
                AgentFeatureKind::Select {
                    value: Some("pro".into()),
                    options: Vec::new(),
                },
            ),
            feature(
                "unset",
                AgentFeatureKind::Select {
                    value: None,
                    options: Vec::new(),
                },
            ),
        ];
        let seed =
            fork_draft_seed(Some(&source), Some(&controls), Some(&thinking), &features).unwrap();
        assert_eq!(
            seed,
            DraftSeed {
                provider: Some("claude".into()),
                // The agent's model wins over the runtime one, as `agent.model ?? runtimeInfo`.
                model: Some("opus".into()),
                mode: Some("plan".into()),
                thinking_option_id: Some("high".into()),
                features: Some(json!({"fast": true, "tier": "pro", "unset": null})),
            }
        );
        // No agent model: the runtime model.
        source.model = None;
        let seed = fork_draft_seed(Some(&source), Some(&controls), None, &[]).unwrap();
        assert_eq!(seed.model.as_deref(), Some("sonnet"));
        assert_eq!(seed.thinking_option_id, None);
        assert_eq!(seed.features, Some(json!({})));
        // No controls loaded yet: provider only.
        let seed = fork_draft_seed(Some(&source), None, None, &[]).unwrap();
        assert_eq!((seed.model, seed.mode), (None, None));
        // No provider or no agent: no seed, so the draft keeps its own settings.
        source.provider = String::new();
        assert_eq!(
            fork_draft_seed(Some(&source), Some(&controls), None, &[]),
            None
        );
        assert_eq!(fork_draft_seed(None, None, None, &[]), None);
    }

    #[test]
    fn a_fork_tab_needs_a_workspace_and_a_folder() {
        assert_eq!(fork_tab_cwd("w1", "/agent", Some("/ws")).unwrap(), "/agent");
        assert_eq!(fork_tab_cwd("w1", "  ", Some("/ws")).unwrap(), "/ws");
        assert_eq!(
            fork_tab_cwd("w1", "", None).unwrap_err(),
            FORK_MISSING_WORKSPACE
        );
        // The home view is no workspace.
        assert_eq!(
            fork_tab_cwd("", "/agent", None).unwrap_err(),
            FORK_MISSING_WORKSPACE
        );
    }

    #[test]
    fn a_fork_workspace_opens_on_the_source_folder_and_project() {
        let projects = vec![ProjectNode {
            id: "p1".into(),
            name: "Project".into(),
            root: "/p1".into(),
            workspaces: vec![workspace("w1", &["a"])],
        }];
        let preset = fork_workspace_preset(&projects, Some("w1"), "/agent");
        assert_eq!(preset.cwd.as_deref(), Some("/w1"));
        assert_eq!(preset.project_id.as_deref(), Some("p1"));
        assert_eq!(preset.title, None);
        // An unknown workspace: the agent's folder, no project.
        let preset = fork_workspace_preset(&projects, Some("gone"), " /agent ");
        assert_eq!(preset.cwd.as_deref(), Some("/agent"));
        assert_eq!(preset.project_id, None);
        assert_eq!(fork_workspace_preset(&projects, None, "").cwd, None);
    }

    #[test]
    fn a_fork_labels_its_chip_and_names_its_failure() {
        assert_eq!(fork_attachment_label(Some("Chat history")), "Chat history");
        assert_eq!(fork_attachment_label(Some(" ")), FORK_ATTACHMENT_LABEL);
        assert_eq!(fork_attachment_label(None), FORK_ATTACHMENT_LABEL);
        // An empty context fails with `forkFailed`; other errors show their text.
        assert_eq!(fork_error("Nothing to fork"), FORK_FAILED);
        assert_eq!(fork_error(""), FORK_FAILED);
        assert_eq!(
            fork_error("Update the host to use this."),
            "Update the host to use this."
        );
    }

    #[test]
    fn the_chat_target_is_the_focused_chat_else_the_first_group_with_a_chat() {
        let chats = ["agent", "draft"];
        let is_chat = |key: &str| chats.contains(&key);
        // The focused tab wins when it is a chat.
        assert_eq!(
            chat_target(Some("draft"), &["agent", "draft"], is_chat),
            Some("draft")
        );
        // A focused file falls back to the first group whose active tab is a chat.
        assert_eq!(
            chat_target(Some("file"), &["terminal", "agent", "draft"], is_chat),
            Some("agent")
        );
        assert_eq!(
            chat_target(Some("file"), &["file", "terminal"], is_chat),
            None
        );
        assert_eq!(chat_target(None, &[], is_chat), None);
    }

    #[test]
    fn a_workspace_file_attaches_as_electron_text_attachment() {
        let (label, attachment) = workspace_file_attachment("src/lib/main.rs");
        assert_eq!(label, "main.rs");
        assert_eq!(
            attachment,
            serde_json::json!({
                "type": "text",
                "mimeType": "text/plain",
                "title": "main.rs",
                "text": "Workspace file: src/lib/main.rs",
            })
        );
        assert_eq!(workspace_file_attachment("README.md").0, "README.md");
    }
}
