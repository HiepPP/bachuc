//! The window: host switcher, sidebar, tab bar, panes, and status line.

use crate::actions::{
    CloseTabAction, NewAgentAction, NewTerminalAction, SHELL_CONTEXT, SelectTabAction,
};
use crate::chrome::{
    SIDEBAR_WIDTH, STATUS_HEIGHT, Sidebar, SidebarData, StatusLine, TAB_BAR_HEIGHT, TabBar,
    status_text, tab_title,
};
use crate::dock;
use crate::new_agent::{NewAgent, pick_model, pick_provider};
use crate::state::{
    Connection, TabKind, Tabs, badge_count, badge_label, folder_name, initial_host,
    notification_for, switch_host, terminal_folder,
};
use bachuc_client::{Client, ClientEvent, HostConfig, load_hosts};
use bachuc_composer::Composer;
use bachuc_diagram::DiagramRenderer;
use bachuc_terminal::TerminalView;
use bachuc_timeline::TimelineView;
use gpui_kit::component::{ActiveTheme as _, h_flex, v_flex};
use gpui_kit::*;
use std::collections::HashMap;
use std::time::Duration;

/// The old client lives this long after a host switch, so a queued terminal kill reaches the
/// old daemon before its socket closes.
const RETIRE_AFTER: Duration = Duration::from_secs(2);

/// What a tab shows. Dropping an agent tab stops its timeline sync.
enum TabContent {
    Agent {
        timeline: Entity<TimelineView>,
        composer: Entity<Composer>,
    },
    Terminal(Entity<TerminalView>),
}

pub struct Shell {
    hosts: Vec<HostConfig>,
    host_index: usize,
    pub(crate) client: Entity<Client>,
    retired: Option<Entity<Client>>,
    diagrams: Entity<DiagramRenderer>,
    connection: Connection,
    tabs: Tabs,
    content: HashMap<u64, TabContent>,
    hosts_open: bool,
    pub(crate) new_agent: Option<NewAgent>,
    /// The last problem to show in the status line, for example a terminal that did not open.
    message: Option<String>,
    badge: usize,
    focus: FocusHandle,
    sidebar: Entity<Sidebar>,
    tab_bar: Entity<TabBar>,
    status_line: Entity<StatusLine>,
    _client_events: Subscription,
}

impl Shell {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let hosts = load_hosts();
        let preferred = std::env::var("BACHUC_HOST").ok();
        let host_index = initial_host(&hosts, preferred.as_deref());
        let client = Client::connect(hosts[host_index].clone(), cx);
        let diagrams = cx.new(|cx| DiagramRenderer::new(window, cx));
        let events = Self::subscribe(&client, window, cx);
        let focus = cx.focus_handle();
        window.focus(&focus, cx);
        Self::open_agent_on_notification(window, cx);
        let shell = cx.weak_entity();
        let sidebar = cx.new(|_| Sidebar::new(shell.clone()));
        let tab_bar = cx.new(|_| TabBar::new(shell.clone()));
        let status_line = cx.new(|_| StatusLine::new(shell));
        Shell {
            hosts,
            host_index,
            client,
            retired: None,
            diagrams,
            connection: Connection::Connecting,
            tabs: Tabs::default(),
            content: HashMap::new(),
            hosts_open: false,
            new_agent: None,
            message: None,
            badge: 0,
            focus,
            sidebar,
            tab_bar,
            status_line,
            _client_events: events,
        }
    }

    fn subscribe(
        client: &Entity<Client>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Subscription {
        cx.subscribe_in(
            client,
            window,
            |this: &mut Shell, client, event: &ClientEvent, window, cx| {
                this.on_client_event(client, event, window, cx)
            },
        )
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
                let _ = shell.update(cx, |shell, cx| shell.open_agent(agent_id, window, cx));
            });
        });
    }

    /// Redraws the window after a state change.
    pub(crate) fn changed(&self, cx: &mut Context<Self>) {
        cx.notify();
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
        self.refresh_badge(cx);
        self.changed(cx);
    }

    fn refresh_badge(&mut self, cx: &mut Context<Self>) {
        let count = badge_count(self.client.read(cx).agents());
        if count != self.badge {
            self.badge = count;
            dock::set_badge(badge_label(count).as_deref());
        }
    }

    fn host_name(&self) -> &str {
        &self.hosts[self.host_index].name
    }

    // ----- data for the chrome views -----

    pub(crate) fn sidebar_data(&self, cx: &App) -> SidebarData {
        let client = self.client.read(cx);
        SidebarData {
            agents: client.agents().to_vec(),
            workspaces: client.workspaces().to_vec(),
            active_agent: match self.tabs.active().map(|tab| &tab.kind) {
                Some(TabKind::Agent(id)) => Some(id.clone()),
                _ => None,
            },
            hosts: self
                .hosts
                .iter()
                .map(|host| (host.name.clone(), host.url.clone()))
                .collect(),
            host_index: self.host_index,
            hosts_open: self.hosts_open,
            connection: self.connection.clone(),
        }
    }

    pub(crate) fn tab_labels(&self, cx: &App) -> Vec<(u64, String)> {
        let agents = self.client.read(cx).agents();
        self.tabs
            .tabs()
            .iter()
            .map(|tab| (tab.id, tab_title(tab, agents)))
            .collect()
    }

    pub(crate) fn active_tab_id(&self) -> Option<u64> {
        self.tabs.active_id()
    }

    /// The status line text and whether it shows a failure.
    pub(crate) fn status(&self, cx: &App) -> (String, bool) {
        let text = status_text(
            &self.connection,
            self.host_name(),
            self.client.read(cx).agents(),
            self.message.as_deref(),
        );
        (text, matches!(self.connection, Connection::Disconnected(_)))
    }

    // ----- host -----

    pub fn toggle_hosts(&mut self, cx: &mut Context<Self>) {
        self.hosts_open = !self.hosts_open;
        self.changed(cx);
    }

    pub fn switch_host(&mut self, next: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.hosts_open = false;
        let Some(closed) =
            switch_host(&mut self.tabs, &mut self.host_index, next, self.hosts.len())
        else {
            self.changed(cx);
            return;
        };
        for tab in closed {
            self.release(tab.id, cx);
        }
        self.new_agent = None;
        self.message = None;
        self.connection = Connection::Connecting;
        let client = Client::connect(self.hosts[self.host_index].clone(), cx);
        self._client_events = Self::subscribe(&client, window, cx);
        self.retired = Some(std::mem::replace(&mut self.client, client));
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(RETIRE_AFTER).await;
            let _ = this.update(cx, |shell, _| shell.retired = None);
        })
        .detach();
        self.refresh_badge(cx);
        self.changed(cx);
    }

    // ----- tabs -----

    pub fn open_agent(&mut self, agent_id: String, window: &mut Window, cx: &mut Context<Self>) {
        self.message = None;
        let (tab_id, is_new) = self.tabs.open_agent(&agent_id);
        if is_new {
            let client = self.client.clone();
            let diagrams = self.diagrams.clone();
            let timeline_id = agent_id.clone();
            let timeline_client = client.clone();
            let timeline =
                cx.new(|cx| TimelineView::new(timeline_client, timeline_id, diagrams, window, cx));
            let composer = cx.new(|cx| Composer::new(client, agent_id, window, cx));
            self.content
                .insert(tab_id, TabContent::Agent { timeline, composer });
        }
        self.focus_active(window, cx);
        self.changed(cx);
    }

    pub fn open_terminal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (connected, folder) = {
            let client = self.client.read(cx);
            let active_agent = match self.tabs.active().map(|tab| &tab.kind) {
                Some(TabKind::Agent(id)) => client.agents().iter().find(|a| a.id == *id),
                _ => None,
            };
            let folder = match active_agent {
                Some(agent) => Some(terminal_folder(agent, client.workspaces())),
                None => client.workspaces().first().map(|w| w.cwd.clone()),
            };
            (client.is_connected(), folder)
        };
        let Some(folder) = folder.filter(|_| connected) else {
            self.message = Some("A terminal needs a connected host with a workspace".into());
            self.changed(cx);
            return;
        };
        self.message = None;
        let client = self.client.clone();
        let view = cx.new(|cx| TerminalView::new(client, folder.clone(), window, cx));
        let tab_id = self
            .tabs
            .open_terminal(&format!("Terminal {}", folder_name(&folder)));
        self.content.insert(tab_id, TabContent::Terminal(view));
        self.changed(cx);
    }

    pub fn close_tab(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        if self.tabs.close(id).is_some() {
            self.release(id, cx);
            self.focus_active(window, cx);
            self.changed(cx);
        }
    }

    /// Drops a tab's views. A terminal tab also kills its daemon terminal.
    fn release(&mut self, id: u64, cx: &mut Context<Self>) {
        if let Some(TabContent::Terminal(view)) = self.content.remove(&id) {
            view.update(cx, |terminal, cx| terminal.close(cx));
        }
    }

    pub fn select_tab(&mut self, id: u64, window: &mut Window, cx: &mut Context<Self>) {
        if self.tabs.select(id) {
            self.focus_active(window, cx);
            self.changed(cx);
        }
    }

    fn select_nth(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.tabs.select_nth(index).is_some() {
            self.focus_active(window, cx);
            self.changed(cx);
        }
    }

    /// Moves keyboard focus into the active pane: the composer of an agent tab, else the root
    /// so the shortcuts keep working.
    fn focus_active(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let composer = self
            .tabs
            .active_id()
            .and_then(|id| self.content.get(&id))
            .and_then(|content| match content {
                TabContent::Agent { composer, .. } => Some(composer.clone()),
                TabContent::Terminal(_) => None,
            });
        match composer {
            Some(composer) => composer.update(cx, |composer, cx| composer.focus(window, cx)),
            None => window.focus(&self.focus, cx),
        }
    }

    // ----- new agent -----

    pub fn open_new_agent(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.new_agent.is_some() {
            return;
        }
        let folder = {
            let client = self.client.read(cx);
            let active = match self.tabs.active().map(|tab| &tab.kind) {
                Some(TabKind::Agent(id)) => client.agents().iter().find(|a| a.id == *id),
                _ => None,
            };
            active
                .map(|agent| agent.cwd.clone())
                .or_else(|| client.workspaces().first().map(|w| w.cwd.clone()))
                .unwrap_or_default()
        };
        self.hosts_open = false;
        self.new_agent = Some(NewAgent::new(folder, window, cx));
        self.changed(cx);
    }

    pub fn close_new_agent(&mut self, cx: &mut Context<Self>) {
        if self.new_agent.take().is_some() {
            self.changed(cx);
        }
    }

    pub fn pick_folder(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let picked = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Choose".into()),
        });
        cx.spawn_in(window, async move |this, cx| {
            let Ok(Ok(Some(paths))) = picked.await else {
                return;
            };
            let Some(path) = paths.first() else {
                return;
            };
            let path = path.to_string_lossy().to_string();
            let _ = this.update_in(cx, |shell, window, cx| {
                if let Some(dialog) = &shell.new_agent {
                    dialog
                        .folder
                        .update(cx, |input, cx| input.set_value(path, window, cx));
                }
            });
        })
        .detach();
    }

    pub fn create_agent(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let providers = self.client.read(cx).providers().to_vec();
        let Some(dialog) = self.new_agent.as_mut() else {
            return;
        };
        if dialog.creating {
            return;
        }
        let folder = dialog.folder.read(cx).value().trim().to_string();
        let prompt = dialog.prompt.read(cx).value().trim().to_string();
        let provider = pick_provider(dialog.provider.as_deref(), &providers);
        if !crate::new_agent::can_create(provider, &folder, &prompt, false) {
            return;
        }
        let Some(provider) = provider else {
            return;
        };
        let provider_id = provider.id.clone();
        let model = pick_model(dialog.model.as_deref(), provider).map(str::to_string);
        dialog.creating = true;
        dialog.error = None;
        let task = self.client.update(cx, |client, cx| {
            client.create_agent(&provider_id, model.as_deref(), &folder, &prompt, cx)
        });
        cx.spawn_in(window, async move |this, cx| {
            let result = task.await;
            let _ = this.update_in(cx, |shell, window, cx| match result {
                Ok(agent_id) => {
                    shell.new_agent = None;
                    shell.open_agent(agent_id, window, cx);
                }
                Err(error) => {
                    if let Some(dialog) = shell.new_agent.as_mut() {
                        dialog.creating = false;
                        dialog.error = Some(format!("{error:#}"));
                    }
                    shell.changed(cx);
                }
            });
        })
        .detach();
        self.changed(cx);
    }

    // ----- views -----

    fn render_pane(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme().clone();
        let content = self.tabs.active_id().and_then(|id| self.content.get(&id));
        match content {
            Some(TabContent::Agent { timeline, composer }) => v_flex()
                .id("pane-agent")
                .role(Role::TabPanel)
                .aria_label("Agent pane")
                .size_full()
                .child(div().flex_1().min_h_0().w_full().child(timeline.clone()))
                .child(composer.clone())
                .into_any_element(),
            Some(TabContent::Terminal(view)) => div()
                .id("pane-terminal")
                .role(Role::TabPanel)
                .aria_label("Terminal pane")
                .size_full()
                .child(view.clone())
                .into_any_element(),
            None => {
                let text = match &self.connection {
                    Connection::Connected => {
                        "Select an agent in the sidebar, or press Cmd+N.".into()
                    }
                    other => other.text(),
                };
                div()
                    .id("pane-empty")
                    .role(Role::Note)
                    .aria_label(text.clone())
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(theme.muted_foreground)
                    .child(text)
                    .into_any_element()
            }
        }
    }
}

impl Render for Shell {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let pane = self.render_pane(cx);
        let dialog = self.render_new_agent(cx);
        // GPUI can cache a view of fixed size, which saves about 2 points of CPU while a stream
        // redraws the window. A cached view drops out of the accessibility tree and loses its
        // actions, and an assistive client was connected on every launch here, so the chrome
        // views stay uncached.
        let sidebar = div()
            .w(px(SIDEBAR_WIDTH))
            .h_full()
            .flex_none()
            .child(self.sidebar.clone());
        let tab_bar = div()
            .w_full()
            .h(px(TAB_BAR_HEIGHT))
            .flex_none()
            .child(self.tab_bar.clone());
        let status = div()
            .w_full()
            .h(px(STATUS_HEIGHT))
            .flex_none()
            .child(self.status_line.clone());
        div()
            .id("shell")
            .role(Role::Group)
            .aria_label("Bachuc")
            .key_context(SHELL_CONTEXT)
            .track_focus(&self.focus)
            .on_action(
                cx.listener(|this, _: &NewAgentAction, window, cx| this.open_new_agent(window, cx)),
            )
            .on_action(
                cx.listener(|this, _: &NewTerminalAction, window, cx| {
                    this.open_terminal(window, cx)
                }),
            )
            .on_action(cx.listener(|this, _: &CloseTabAction, window, cx| {
                if let Some(id) = this.tabs.active_id() {
                    this.close_tab(id, window, cx);
                }
            }))
            .on_action(cx.listener(|this, action: &SelectTabAction, window, cx| {
                this.select_nth(action.0, window, cx)
            }))
            .relative()
            .size_full()
            .bg(theme.background)
            .text_color(theme.foreground)
            .child(
                v_flex()
                    .size_full()
                    .child(
                        h_flex().flex_1().min_h_0().child(sidebar).child(
                            v_flex()
                                .id("main")
                                .role(Role::Main)
                                .aria_label("Main")
                                .flex_1()
                                .min_w_0()
                                .h_full()
                                .child(tab_bar)
                                .child(div().flex_1().min_h_0().w_full().child(pane)),
                        ),
                    )
                    .child(status),
            )
            .children(dialog)
    }
}
