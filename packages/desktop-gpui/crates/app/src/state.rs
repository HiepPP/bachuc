//! App state that needs no window: tabs, sidebar groups, the dock badge count, connection text,
//! notification choice, and the shortcut table. The views in `shell` read these.

use bachuc_client::{AgentSummary, ClientEvent, HostConfig, WorkspaceSummary};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TabKind {
    Agent(String),
    Terminal,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tab {
    pub id: u64,
    pub kind: TabKind,
    /// The label of a terminal tab. An agent tab takes its title from the agent list.
    pub title: String,
}

/// The open tabs of the current host and which one shows.
#[derive(Default)]
pub struct Tabs {
    tabs: Vec<Tab>,
    active: Option<u64>,
    next_id: u64,
}

impl Tabs {
    pub fn tabs(&self) -> &[Tab] {
        &self.tabs
    }

    pub fn active(&self) -> Option<&Tab> {
        let id = self.active?;
        self.tabs.iter().find(|tab| tab.id == id)
    }

    pub fn active_id(&self) -> Option<u64> {
        self.active
    }

    /// Opens an agent in a tab, or shows its tab when it is open. Returns the tab id and
    /// whether the tab is new.
    pub fn open_agent(&mut self, agent_id: &str) -> (u64, bool) {
        let existing = self
            .tabs
            .iter()
            .find(|tab| tab.kind == TabKind::Agent(agent_id.to_string()));
        if let Some(tab) = existing {
            let id = tab.id;
            self.active = Some(id);
            return (id, false);
        }
        (
            self.push(TabKind::Agent(agent_id.to_string()), String::new()),
            true,
        )
    }

    pub fn open_terminal(&mut self, title: &str) -> u64 {
        self.push(TabKind::Terminal, title.to_string())
    }

    fn push(&mut self, kind: TabKind, title: String) -> u64 {
        self.next_id += 1;
        let id = self.next_id;
        self.tabs.push(Tab { id, kind, title });
        self.active = Some(id);
        id
    }

    pub fn select(&mut self, id: u64) -> bool {
        let known = self.tabs.iter().any(|tab| tab.id == id);
        if known {
            self.active = Some(id);
        }
        known
    }

    /// Shows the tab at `index` (0-based). Cmd+1 is index 0.
    pub fn select_nth(&mut self, index: usize) -> Option<u64> {
        let id = self.tabs.get(index)?.id;
        self.active = Some(id);
        Some(id)
    }

    /// Closes a tab. When it was active, the tab to its right shows, else the one to its left.
    pub fn close(&mut self, id: u64) -> Option<Tab> {
        let index = self.tabs.iter().position(|tab| tab.id == id)?;
        let closed = self.tabs.remove(index);
        if self.active == Some(id) {
            let next = self.tabs.get(index).or_else(|| self.tabs.last());
            self.active = next.map(|tab| tab.id);
        }
        Some(closed)
    }

    /// Closes every tab and returns them, so the caller can release their views.
    pub fn clear(&mut self) -> Vec<Tab> {
        self.active = None;
        std::mem::take(&mut self.tabs)
    }
}

/// Switches the host index. A switch to another host closes every tab and returns them. A
/// switch to the same host, or to an unknown index, changes nothing.
pub fn switch_host(
    tabs: &mut Tabs,
    current: &mut usize,
    next: usize,
    host_count: usize,
) -> Option<Vec<Tab>> {
    if next == *current || next >= host_count {
        return None;
    }
    *current = next;
    Some(tabs.clear())
}

/// The host to open on. `preferred` is a host name (the `BACHUC_HOST` variable); without it
/// the host named `live` opens, then the first one.
pub fn initial_host(hosts: &[HostConfig], preferred: Option<&str>) -> usize {
    let named = |name: &str| hosts.iter().position(|host| host.name == name);
    preferred
        .and_then(named)
        .or_else(|| named("live"))
        .unwrap_or(0)
}

/// Agents of one workspace, as the sidebar lists them. It borrows the client's lists, because
/// the sidebar groups them on every frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SidebarGroup<'a> {
    pub name: &'a str,
    pub agents: Vec<&'a AgentSummary>,
}

/// Groups agents by workspace name, in workspace order. Agents with no known workspace come
/// last under "Other". A workspace with no agent is left out.
pub fn group_agents<'a>(
    agents: &'a [AgentSummary],
    workspaces: &'a [WorkspaceSummary],
) -> Vec<SidebarGroup<'a>> {
    let mut groups: Vec<SidebarGroup> = Vec::new();
    let mut other = Vec::new();
    for workspace in workspaces {
        let members: Vec<&AgentSummary> = agents
            .iter()
            .filter(|agent| agent.workspace_id.as_deref() == Some(workspace.id.as_str()))
            .collect();
        if !members.is_empty() {
            groups.push(SidebarGroup {
                name: &workspace.name,
                agents: members,
            });
        }
    }
    for agent in agents {
        let known = agent
            .workspace_id
            .as_deref()
            .is_some_and(|id| workspaces.iter().any(|workspace| workspace.id == id));
        if !known {
            other.push(agent);
        }
    }
    if !other.is_empty() {
        groups.push(SidebarGroup {
            name: "Other",
            agents: other,
        });
    }
    groups
}

/// The dock badge number: pending permissions over all agents.
pub fn badge_count(agents: &[AgentSummary]) -> usize {
    agents.iter().map(|agent| agent.pending_permissions).sum()
}

/// The badge text. Zero clears the badge, so it has no text.
pub fn badge_label(count: usize) -> Option<String> {
    (count > 0).then(|| count.to_string())
}

/// The title of an agent tab or row.
pub fn agent_title(agent: &AgentSummary) -> String {
    let title = agent.title.trim();
    if title.is_empty() {
        format!("Agent {}", short_id(&agent.id))
    } else {
        title.to_string()
    }
}

pub fn short_id(id: &str) -> &str {
    id.get(..8).unwrap_or(id)
}

/// The folder for a new terminal next to an agent. The daemon accepts only workspace folders,
/// so the agent folder wins when a workspace has it, then the folder of the agent's workspace.
pub fn terminal_folder(agent: &AgentSummary, workspaces: &[WorkspaceSummary]) -> String {
    if workspaces
        .iter()
        .any(|workspace| workspace.cwd == agent.cwd)
    {
        return agent.cwd.clone();
    }
    agent
        .workspace_id
        .as_deref()
        .and_then(|id| workspaces.iter().find(|workspace| workspace.id == id))
        .map(|workspace| workspace.cwd.clone())
        .unwrap_or_else(|| agent.cwd.clone())
}

/// The last folder name, for a terminal tab label.
pub fn folder_name(cwd: &str) -> String {
    cwd.trim_end_matches('/')
        .rsplit('/')
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or("/")
        .to_string()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Connection {
    Connecting,
    Connected,
    Disconnected(String),
}

impl Connection {
    /// The text of the status line and the empty pane.
    pub fn text(&self) -> String {
        match self {
            Connection::Connecting => "Connecting...".into(),
            Connection::Connected => "Connected".into(),
            Connection::Disconnected(reason) if reason.is_empty() => {
                "Disconnected, retrying".into()
            }
            Connection::Disconnected(reason) => format!("Disconnected: {reason}, retrying"),
        }
    }

    pub fn apply(&mut self, event: &ClientEvent) {
        match event {
            ClientEvent::Connected => *self = Connection::Connected,
            ClientEvent::Disconnected { reason } => {
                *self = Connection::Disconnected(reason.clone())
            }
            _ => {}
        }
    }
}

/// One system notification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Note {
    pub tag: String,
    pub title: String,
    pub body: String,
    pub agent_id: String,
}

/// The notification for an event, or none. Only an unfocused window notifies, because a
/// focused window shows the change already.
pub fn notification_for(
    event: &ClientEvent,
    agents: &[AgentSummary],
    window_active: bool,
) -> Option<Note> {
    if window_active {
        return None;
    }
    let name = |agent_id: &str| {
        agents
            .iter()
            .find(|agent| agent.id == agent_id)
            .map(agent_title)
            .unwrap_or_else(|| format!("Agent {}", short_id(agent_id)))
    };
    match event {
        ClientEvent::PermissionRequested { agent_id } => Some(Note {
            tag: format!("permission-{agent_id}"),
            title: "Permission requested".into(),
            body: format!("{} waits for your approval", name(agent_id)),
            agent_id: agent_id.clone(),
        }),
        ClientEvent::AgentFinished { agent_id } => Some(Note {
            tag: format!("finished-{agent_id}"),
            title: "Agent finished".into(),
            body: format!("{} finished", name(agent_id)),
            agent_id: agent_id.clone(),
        }),
        _ => None,
    }
}

/// The agent id inside a notification tag, so a click on the notification opens that agent.
pub fn agent_from_tag(tag: &str) -> Option<&str> {
    tag.strip_prefix("permission-")
        .or_else(|| tag.strip_prefix("finished-"))
}

/// The window size in points. `spec` is `WIDTHxHEIGHT` (the `BACHUC_WINDOW` variable); anything
/// else gives the default.
pub fn window_size(spec: Option<&str>) -> (f32, f32) {
    const DEFAULT: (f32, f32) = (1100., 760.);
    let parsed = spec.and_then(|spec| {
        let (width, height) = spec.split_once('x')?;
        Some((
            width.trim().parse::<f32>().ok()?,
            height.trim().parse::<f32>().ok()?,
        ))
    });
    match parsed {
        Some((width, height)) if width >= 640. && height >= 480. => (width, height),
        _ => DEFAULT,
    }
}

/// What a shortcut does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shortcut {
    NewAgent,
    NewTerminal,
    CloseTab,
    /// Shows the tab at this 0-based index.
    SelectTab(usize),
}

/// Every key binding: the keystroke and what it does.
pub fn shortcuts() -> Vec<(String, Shortcut)> {
    let mut table = vec![
        ("cmd-n".to_string(), Shortcut::NewAgent),
        ("cmd-t".to_string(), Shortcut::NewTerminal),
        ("cmd-w".to_string(), Shortcut::CloseTab),
    ];
    for n in 1..=9usize {
        table.push((format!("cmd-{n}"), Shortcut::SelectTab(n - 1)));
    }
    table
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agent(id: &str, workspace: Option<&str>, permissions: usize) -> AgentSummary {
        AgentSummary {
            id: id.into(),
            title: format!("title {id}"),
            status: "idle".into(),
            cwd: "/work/a".into(),
            workspace_id: workspace.map(str::to_string),
            provider: "mock".into(),
            model: None,
            pending_permissions: permissions,
        }
    }

    fn workspace(id: &str, name: &str, cwd: &str) -> WorkspaceSummary {
        WorkspaceSummary {
            id: id.into(),
            name: name.into(),
            cwd: cwd.into(),
        }
    }

    fn host(name: &str) -> HostConfig {
        HostConfig {
            name: name.into(),
            url: format!("ws://127.0.0.1/{name}"),
            password: None,
        }
    }

    #[test]
    fn opening_an_open_agent_shows_its_tab() {
        let mut tabs = Tabs::default();
        let (first, new) = tabs.open_agent("a1");
        assert!(new);
        let (second, _) = tabs.open_agent("a2");
        assert_eq!(tabs.active_id(), Some(second));
        let (again, new) = tabs.open_agent("a1");
        assert!(!new);
        assert_eq!(again, first);
        assert_eq!(tabs.active_id(), Some(first));
        assert_eq!(tabs.tabs().len(), 2);
    }

    #[test]
    fn closing_the_active_tab_shows_the_right_neighbor_then_the_left() {
        let mut tabs = Tabs::default();
        let a = tabs.open_agent("a").0;
        let b = tabs.open_terminal("term");
        let c = tabs.open_agent("c").0;
        tabs.select(b);
        tabs.close(b);
        assert_eq!(tabs.active_id(), Some(c));
        tabs.close(c);
        assert_eq!(tabs.active_id(), Some(a));
        tabs.close(a);
        assert_eq!(tabs.active_id(), None);
        assert!(tabs.close(a).is_none());
    }

    #[test]
    fn closing_a_background_tab_keeps_the_active_one() {
        let mut tabs = Tabs::default();
        let a = tabs.open_agent("a").0;
        let b = tabs.open_agent("b").0;
        tabs.close(a);
        assert_eq!(tabs.active_id(), Some(b));
    }

    #[test]
    fn number_shortcuts_pick_the_nth_tab() {
        let mut tabs = Tabs::default();
        let a = tabs.open_agent("a").0;
        let b = tabs.open_terminal("t");
        assert_eq!(tabs.select_nth(1), Some(b));
        assert_eq!(tabs.select_nth(0), Some(a));
        assert_eq!(tabs.select_nth(2), None);
        assert_eq!(tabs.active_id(), Some(a));
    }

    #[test]
    fn a_host_switch_closes_every_tab() {
        let mut tabs = Tabs::default();
        tabs.open_agent("a");
        tabs.open_terminal("t");
        let mut current = 1;
        let closed = switch_host(&mut tabs, &mut current, 0, 2).expect("switches");
        assert_eq!(closed.len(), 2);
        assert_eq!(current, 0);
        assert!(tabs.tabs().is_empty());
        assert_eq!(tabs.active_id(), None);
    }

    #[test]
    fn a_switch_to_the_same_or_an_unknown_host_keeps_the_tabs() {
        let mut tabs = Tabs::default();
        tabs.open_agent("a");
        let mut current = 1;
        assert!(switch_host(&mut tabs, &mut current, 1, 2).is_none());
        assert!(switch_host(&mut tabs, &mut current, 5, 2).is_none());
        assert_eq!(current, 1);
        assert_eq!(tabs.tabs().len(), 1);
    }

    #[test]
    fn the_app_opens_on_live() {
        let hosts = [host("release"), host("live")];
        assert_eq!(initial_host(&hosts, None), 1);
        assert_eq!(initial_host(&hosts, Some("release")), 0);
        assert_eq!(initial_host(&hosts, Some("nope")), 1);
        assert_eq!(initial_host(&[host("mac")], None), 0);
    }

    #[test]
    fn the_sidebar_groups_by_workspace_and_puts_strays_last() {
        let workspaces = [
            workspace("w1", "alpha", "/a"),
            workspace("w2", "beta", "/b"),
        ];
        let agents = [
            agent("a1", Some("w2"), 0),
            agent("a2", Some("w1"), 0),
            agent("a3", None, 0),
            agent("a4", Some("gone"), 0),
            agent("a5", Some("w2"), 0),
        ];
        let groups = group_agents(&agents, &workspaces);
        let names: Vec<&str> = groups.iter().map(|group| group.name).collect();
        assert_eq!(names, ["alpha", "beta", "Other"]);
        let beta: Vec<&str> = groups[1].agents.iter().map(|a| a.id.as_str()).collect();
        assert_eq!(beta, ["a1", "a5"]);
        assert_eq!(groups[2].agents.len(), 2);
    }

    #[test]
    fn no_agents_gives_no_groups() {
        assert!(group_agents(&[], &[workspace("w1", "alpha", "/a")]).is_empty());
    }

    #[test]
    fn the_badge_sums_permissions_and_clears_at_zero() {
        let agents = [
            agent("a", None, 1),
            agent("b", None, 2),
            agent("c", None, 0),
        ];
        assert_eq!(badge_count(&agents), 3);
        assert_eq!(badge_label(3), Some("3".into()));
        assert_eq!(badge_count(&[agent("a", None, 0)]), 0);
        assert_eq!(badge_label(0), None);
        assert_eq!(badge_count(&[]), 0);
    }

    #[test]
    fn a_blank_title_falls_back_to_the_short_id() {
        let mut a = agent("0123456789", None, 0);
        a.title = "  ".into();
        assert_eq!(agent_title(&a), "Agent 01234567");
        assert_eq!(short_id("abc"), "abc");
    }

    #[test]
    fn a_terminal_opens_in_a_workspace_folder() {
        let workspaces = [workspace("w1", "alpha", "/work")];
        let mut a = agent("a", Some("w1"), 0);
        a.cwd = "/work".into();
        assert_eq!(terminal_folder(&a, &workspaces), "/work");
        a.cwd = "/work/sub".into();
        assert_eq!(terminal_folder(&a, &workspaces), "/work");
        a.workspace_id = None;
        assert_eq!(terminal_folder(&a, &workspaces), "/work/sub");
        assert_eq!(folder_name("/work/sub/"), "sub");
        assert_eq!(folder_name("/"), "/");
    }

    #[test]
    fn connection_text_shows_the_rejection_reason() {
        let mut state = Connection::Connecting;
        assert_eq!(state.text(), "Connecting...");
        state.apply(&ClientEvent::Disconnected {
            reason: "incorrect_password".into(),
        });
        assert_eq!(state.text(), "Disconnected: incorrect_password, retrying");
        state.apply(&ClientEvent::AgentsChanged);
        assert!(matches!(state, Connection::Disconnected(_)));
        state.apply(&ClientEvent::Connected);
        assert_eq!(state, Connection::Connected);
    }

    #[test]
    fn only_an_unfocused_window_notifies() {
        let agents = [agent("a1", None, 1)];
        let permission = ClientEvent::PermissionRequested {
            agent_id: "a1".into(),
        };
        assert_eq!(notification_for(&permission, &agents, true), None);
        let note = notification_for(&permission, &agents, false).expect("notifies");
        assert_eq!(note.title, "Permission requested");
        assert_eq!(note.body, "title a1 waits for your approval");
        let finished = ClientEvent::AgentFinished {
            agent_id: "a1".into(),
        };
        let note = notification_for(&finished, &agents, false).expect("notifies");
        assert_eq!(note.tag, "finished-a1");
        assert_eq!(
            notification_for(&ClientEvent::AgentsChanged, &agents, false),
            None
        );
    }

    #[test]
    fn the_window_size_reads_width_and_height() {
        assert_eq!(window_size(None), (1100., 760.));
        assert_eq!(window_size(Some("1280x800")), (1280., 800.));
        assert_eq!(window_size(Some(" 900 x 700 ")), (900., 700.));
        assert_eq!(window_size(Some("wide")), (1100., 760.));
        assert_eq!(window_size(Some("100x100")), (1100., 760.));
    }

    #[test]
    fn a_notification_tag_names_its_agent() {
        assert_eq!(agent_from_tag("permission-a1"), Some("a1"));
        assert_eq!(agent_from_tag("finished-a1"), Some("a1"));
        assert_eq!(agent_from_tag("other-a1"), None);
    }

    #[test]
    fn shortcuts_cover_new_terminal_close_and_nine_tabs() {
        let table = shortcuts();
        let find = |key: &str| table.iter().find(|(k, _)| k == key).map(|(_, s)| *s);
        assert_eq!(find("cmd-n"), Some(Shortcut::NewAgent));
        assert_eq!(find("cmd-t"), Some(Shortcut::NewTerminal));
        assert_eq!(find("cmd-w"), Some(Shortcut::CloseTab));
        assert_eq!(find("cmd-1"), Some(Shortcut::SelectTab(0)));
        assert_eq!(find("cmd-9"), Some(Shortcut::SelectTab(8)));
        assert_eq!(find("cmd-0"), None);
        assert_eq!(table.len(), 12);
    }
}
