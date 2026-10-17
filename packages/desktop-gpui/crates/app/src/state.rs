//! App state that needs no window: connection text, notification choice, the dock badge count,
//! pane keys, workspace order, panel widths, the stored layout, and the command center items.
//! The views in `shell` read these.

use bachuc_client::{
    AgentSummary, ClientEvent, HostConfig, ProjectNode, WorkspaceNode, WorkspaceScript,
};
use bachuc_command::{CommandItem, Shortcut};
use bachuc_explorer::ShipDefault;
use bachuc_icons::IconName;
use bachuc_theme::ThemeChoice;
use gpui_kit::Hsla;
use std::path::Path;

/// The host to open on. `preferred` is a host name (the `BACHUC_HOST` variable); without it
/// the host named `live` opens, then the first one.
pub fn initial_host(hosts: &[HostConfig], preferred: Option<&str>) -> usize {
    let named = |name: &str| hosts.iter().position(|host| host.name == name);
    preferred
        .and_then(named)
        .or_else(|| named("live"))
        .unwrap_or(0)
}

/// The dock badge number: pending permissions over all agents.
pub fn badge_count(agents: &[AgentSummary]) -> usize {
    agents.iter().map(|agent| agent.pending_permissions).sum()
}

/// The badge text. Zero clears the badge, so it has no text.
pub fn badge_label(count: usize) -> Option<String> {
    (count > 0).then(|| count.to_string())
}

/// The title of an agent tab or command row.
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

/// The last path segment, for a tab label.
pub fn file_name(path: &str) -> String {
    path.trim_end_matches('/')
        .rsplit('/')
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or("/")
        .to_string()
}

/// `path` when it is absolute, else `path` under `cwd`.
pub fn absolute_path(cwd: &str, path: &str) -> String {
    if path.starts_with('/') {
        path.to_string()
    } else {
        format!(
            "{}/{}",
            cwd.trim_end_matches('/'),
            path.trim_start_matches("./")
        )
    }
}

// ----- pane keys -----

pub fn agent_key(agent_id: &str) -> String {
    format!("agent:{agent_id}")
}

/// The agent id of an agent pane key.
pub fn agent_of_key(key: &str) -> Option<&str> {
    key.strip_prefix("agent:")
}

/// One file pane per file: a second open of the same file shows the open tab.
pub fn file_key(cwd: &str, path: &str) -> String {
    format!("file:{}", absolute_path(cwd, path))
}

pub fn diff_key(cwd: &str, path: &str) -> String {
    format!("diff:{}", absolute_path(cwd, path))
}

/// One all-files diff tab per folder: a second Changes click shows the open tab.
pub fn all_diffs_key(cwd: &str) -> String {
    format!("diffs:{}", cwd.trim_end_matches('/'))
}

/// One Pull request tab per folder.
pub fn pull_request_key(cwd: &str) -> String {
    format!("pr:{}", cwd.trim_end_matches('/'))
}

/// One diff tab per file of a commit.
pub fn commit_diff_key(cwd: &str, sha: &str, path: &str) -> String {
    format!("commit:{sha}:{}", absolute_path(cwd, path))
}

/// The tab title of a commit diff: the file name and the short sha.
pub fn commit_diff_title(sha: &str, path: &str) -> String {
    format!(
        "{} ({})",
        file_name(path),
        sha.chars().take(7).collect::<String>()
    )
}

/// The absolute path of a file pane key.
pub fn file_of_key(key: &str) -> Option<&str> {
    key.strip_prefix("file:")
}

/// The tab icon of an agent when it has no provider logo.
pub fn agent_icon(provider: Option<&str>) -> IconName {
    provider.map_or(IconName::Bot, bachuc_icons::provider_fallback_icon)
}

/// The color the provider logos are drawn in. The logos are rebuilt when it changes, not when
/// another theme setting (font, size, syntax) writes the theme.
pub struct LogoFill(Hsla);

impl LogoFill {
    pub fn new(fill: Hsla) -> Self {
        Self(fill)
    }

    /// Records `fill` and tells whether the logos need a rebuild.
    pub fn changed(&mut self, fill: Hsla) -> bool {
        std::mem::replace(&mut self.0, fill) != fill
    }
}

// ----- workspaces -----

/// The workspace with this id and its project.
pub fn find_workspace<'a>(
    projects: &'a [ProjectNode],
    workspace_id: &str,
) -> Option<(&'a ProjectNode, &'a WorkspaceNode)> {
    projects.iter().find_map(|project| {
        project
            .workspaces
            .iter()
            .find(|workspace| workspace.id == workspace_id)
            .map(|workspace| (project, workspace))
    })
}

/// Workspaces in the sidebar order: pinned ones first, then each project's unpinned ones.
pub fn workspace_order(projects: &[ProjectNode]) -> Vec<&str> {
    let all = || projects.iter().flat_map(|project| &project.workspaces);
    all()
        .filter(|workspace| workspace.pinned)
        .chain(all().filter(|workspace| !workspace.pinned))
        .map(|workspace| workspace.id.as_str())
        .collect()
}

/// The workspace before or after `current` in `order`, wrapping at the ends. With no current
/// workspace the first one is next.
pub fn step_workspace(order: &[&str], current: Option<&str>, forward: bool) -> Option<String> {
    if order.is_empty() {
        return None;
    }
    let index = current.and_then(|id| order.iter().position(|item| *item == id));
    step_index(order.len(), index, forward).map(|next| order[next].to_string())
}

/// The index before or after `active` in a list of `len`, wrapping at the ends.
pub fn step_index(len: usize, active: Option<usize>, forward: bool) -> Option<usize> {
    if len == 0 {
        return None;
    }
    Some(match (active, forward) {
        (None, _) => 0,
        (Some(index), true) => (index + 1) % len,
        (Some(index), false) => (index + len - 1) % len,
    })
}

/// The agents of a workspace that the agent list still has, in its order.
pub fn workspace_agents<'a>(workspace: &'a WorkspaceNode, agents: &[AgentSummary]) -> Vec<&'a str> {
    workspace
        .agent_ids
        .iter()
        .filter(|id| agents.iter().any(|agent| agent.id == **id))
        .map(String::as_str)
        .collect()
}

// ----- layout -----

/// `DEFAULT_EXPLORER_SIDEBAR_WIDTH`, `MIN_EXPLORER_SIDEBAR_WIDTH`, and
/// `MIN_WORKSPACE_BODY_WIDTH` of `explorer-sidebar-layout.ts`.
pub const EXPLORER_WIDTH: f32 = 320.;
pub const EXPLORER_MIN_WIDTH: f32 = 240.;
pub const WORKSPACE_BODY_MIN_WIDTH: f32 = 400.;

/// `resolveExplorerSidebarWidth`: the stored width, kept under the space that leaves the
/// workspace body its minimum, and never under the Explorer minimum.
pub fn explorer_width(requested: f32, container: f32) -> f32 {
    let maximum = if container > 0. {
        EXPLORER_MIN_WIDTH.max(container - WORKSPACE_BODY_MIN_WIDTH)
    } else {
        requested
    };
    EXPLORER_MIN_WIDTH.max(maximum.min(requested))
}

/// The panel layout that survives a restart, stored in `layout.json`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutPrefs {
    pub sidebar_open: bool,
    pub explorer_open: bool,
    pub explorer_width: f32,
}

impl Default for LayoutPrefs {
    fn default() -> Self {
        Self {
            sidebar_open: true,
            explorer_open: false,
            explorer_width: EXPLORER_WIDTH,
        }
    }
}

const LAYOUT_FILE: &str = "layout.json";

impl LayoutPrefs {
    /// Reads the stored layout. A missing or bad file, or a bad field, gives the default.
    pub fn parse(text: &str) -> Self {
        let value: serde_json::Value = serde_json::from_str(text).unwrap_or_default();
        let default = Self::default();
        let flag = |key: &str, fallback: bool| value[key].as_bool().unwrap_or(fallback);
        Self {
            sidebar_open: flag("sidebarOpen", default.sidebar_open),
            explorer_open: flag("explorerOpen", default.explorer_open),
            explorer_width: value["explorerWidth"]
                .as_f64()
                .map(|width| width as f32)
                .filter(|width| width.is_finite() && *width > 0.)
                .unwrap_or(default.explorer_width),
        }
    }

    pub fn to_json(self) -> String {
        serde_json::json!({
            "sidebarOpen": self.sidebar_open,
            "explorerOpen": self.explorer_open,
            "explorerWidth": self.explorer_width,
        })
        .to_string()
    }

    pub fn load_from(dir: &Path) -> Self {
        std::fs::read_to_string(dir.join(LAYOUT_FILE))
            .map(|text| Self::parse(&text))
            .unwrap_or_default()
    }

    /// Writes through a temp file and a rename, so a crash leaves the old file.
    pub fn save_to(self, dir: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(dir)?;
        let temp = dir.join(format!("{LAYOUT_FILE}.tmp"));
        std::fs::write(&temp, self.to_json())?;
        std::fs::rename(temp, dir.join(LAYOUT_FILE))
    }
}

// ----- git ship default -----

/// The git ship default of each repo, by cwd (Q-111), as Electron keeps it per repo.
const SHIP_DEFAULTS_FILE: &str = "git-ship-defaults.json";

fn ship_defaults(dir: &Path) -> serde_json::Map<String, serde_json::Value> {
    std::fs::read_to_string(dir.join(SHIP_DEFAULTS_FILE))
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default()
}

/// The saved ship default of a repo. A missing file, entry, or bad value gives `Pr`.
pub fn load_ship_default(dir: &Path, cwd: &str) -> ShipDefault {
    match ship_defaults(dir).get(cwd).and_then(|value| value.as_str()) {
        Some("merge") => ShipDefault::Merge,
        _ => ShipDefault::Pr,
    }
}

/// Saves a repo's ship default and keeps the other repos' entries. Writes through a temp file
/// and a rename, as `LayoutPrefs::save_to` does.
pub fn save_ship_default(dir: &Path, cwd: &str, default: ShipDefault) -> std::io::Result<()> {
    let mut all = ship_defaults(dir);
    let value = match default {
        ShipDefault::Pr => "pr",
        ShipDefault::Merge => "merge",
    };
    all.insert(cwd.to_string(), value.into());
    std::fs::create_dir_all(dir)?;
    let temp = dir.join(format!("{SHIP_DEFAULTS_FILE}.tmp"));
    std::fs::write(&temp, serde_json::Value::Object(all).to_string())?;
    std::fs::rename(temp, dir.join(SHIP_DEFAULTS_FILE))
}

// ----- workspace scripts -----

/// The state text of a script row: `running` (with its health when known), `exit N` after an
/// exit, else `stopped`.
pub fn script_state(script: &WorkspaceScript) -> String {
    if script.lifecycle == "running" {
        return match script.health.as_deref() {
            Some(health) => format!("running, {health}"),
            None => "running".into(),
        };
    }
    match script.exit_code {
        Some(code) => format!("exit {code}"),
        None => "stopped".into(),
    }
}

// ----- open in editor -----

/// An app the header's Open in editor button can open the workspace folder in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorTarget {
    VsCode,
    Cursor,
    Zed,
    Finder,
    Terminal,
}

impl EditorTarget {
    pub fn label(self) -> &'static str {
        match self {
            EditorTarget::VsCode => "VS Code",
            EditorTarget::Cursor => "Cursor",
            EditorTarget::Zed => "Zed",
            EditorTarget::Finder => "Finder",
            EditorTarget::Terminal => "Terminal",
        }
    }

    pub fn icon(self) -> IconName {
        match self {
            EditorTarget::VsCode | EditorTarget::Cursor | EditorTarget::Zed => IconName::Code,
            EditorTarget::Finder => IconName::FolderOpen,
            EditorTarget::Terminal => IconName::SquareTerminal,
        }
    }

    /// The bundle under `/Applications`; Finder and Terminal are system apps that always exist.
    fn bundle(self) -> Option<&'static str> {
        match self {
            EditorTarget::VsCode => Some("/Applications/Visual Studio Code.app"),
            EditorTarget::Cursor => Some("/Applications/Cursor.app"),
            EditorTarget::Zed => Some("/Applications/Zed.app"),
            EditorTarget::Finder | EditorTarget::Terminal => None,
        }
    }
}

/// The targets to list: each editor whose bundle `exists`, then Finder and Terminal.
pub fn editor_targets(exists: impl Fn(&str) -> bool) -> Vec<EditorTarget> {
    [
        EditorTarget::VsCode,
        EditorTarget::Cursor,
        EditorTarget::Zed,
        EditorTarget::Finder,
        EditorTarget::Terminal,
    ]
    .into_iter()
    .filter(|target| target.bundle().is_none_or(&exists))
    .collect()
}

/// The arguments of the macOS `open` command that opens `cwd` in `target`.
pub fn open_args(target: EditorTarget, cwd: &str) -> Vec<String> {
    let app = match target {
        EditorTarget::Finder => return vec![cwd.to_string()],
        EditorTarget::VsCode => "Visual Studio Code",
        EditorTarget::Cursor => "Cursor",
        EditorTarget::Zed => "Zed",
        EditorTarget::Terminal => "Terminal",
    };
    vec!["-a".into(), app.into(), cwd.to_string()]
}

/// The GitHub page of a remote (`git@github.com:o/r.git`, `https://github.com/o/r`,
/// `ssh://git@github.com/o/r.git`), at `branch` when known. Other forges give `None`.
pub fn github_url(remote: &str, branch: Option<&str>) -> Option<String> {
    let remote = remote.trim();
    let path = remote
        .strip_prefix("git@github.com:")
        .or_else(|| remote.strip_prefix("ssh://git@github.com/"))
        .or_else(|| remote.strip_prefix("https://github.com/"))
        .or_else(|| remote.strip_prefix("http://github.com/"))?;
    let path = path.trim_end_matches('/');
    let path = path.strip_suffix(".git").unwrap_or(path);
    let mut parts = path.split('/');
    let (owner, repo) = (parts.next()?, parts.next()?);
    if owner.is_empty() || repo.is_empty() || parts.next().is_some() {
        return None;
    }
    let base = format!("https://github.com/{owner}/{repo}");
    Some(match branch.filter(|branch| !branch.is_empty()) {
        Some(branch) => format!("{base}/tree/{branch}"),
        None => base,
    })
}

/// `theme.cycle`: Light, then Claude dark, then System.
pub fn next_theme(theme: ThemeChoice) -> ThemeChoice {
    match theme {
        ThemeChoice::Light => ThemeChoice::ClaudeDark,
        ThemeChoice::ClaudeDark => ThemeChoice::System,
        ThemeChoice::System => ThemeChoice::Light,
    }
}

/// `message-input.mode-cycle`: the mode after the current one, wrapping. None when the agent
/// has fewer than two modes.
pub fn next_mode(modes: &[(String, String)], current: Option<&str>) -> Option<String> {
    if modes.len() < 2 {
        return None;
    }
    let index = current.and_then(|id| modes.iter().position(|(mode, _)| mode == id));
    step_index(modes.len(), index, true).map(|next| modes[next].0.clone())
}

// ----- command center -----

/// The action that jumps to a tab by digit; the command center does not list it.
pub const JUMP_TO_TAB: &str = "workspace.tab.navigate.index";

fn action_icon(action: &str) -> Option<IconName> {
    Some(match action {
        "command-center.toggle" => IconName::Search,
        "settings.toggle" => IconName::Settings,
        "theme.cycle" => IconName::Palette,
        "host.cycle" => IconName::Server,
        "workspace.archive" => IconName::Archive,
        "workspace.tab.menu.open" => IconName::Plus,
        "workspace.terminal.new" => IconName::SquareTerminal,
        "workspace.tab.target.agent" => IconName::SquarePen,
        "workspace.tab.target.changes" => IconName::GitCompareArrows,
        "workspace.tab.target.files" => IconName::Files,
        "workspace.pane.split.right" | "workspace.pane.split.down" => IconName::Split,
        "sidebar.toggle.left" => IconName::PanelLeft,
        "sidebar.toggle.right" => IconName::PanelRight,
        "message-input.focus" => IconName::MessageSquare,
        _ => return None,
    })
}

/// The command center rows: the app actions with their chords, then the workspaces in sidebar
/// order, then the open agents. Ids are `action:<action>`, `workspace:<id>`, and `agent:<id>`.
pub fn command_items(
    shortcuts: &[Shortcut],
    projects: &[ProjectNode],
    agents: &[AgentSummary],
) -> Vec<CommandItem> {
    let mut items: Vec<CommandItem> = shortcuts
        .iter()
        .filter(|shortcut| shortcut.action != JUMP_TO_TAB)
        .map(|shortcut| CommandItem {
            id: format!("action:{}", shortcut.action),
            title: shortcut.label.clone().into(),
            group: "Actions".into(),
            icon: action_icon(&shortcut.action),
            shortcut: shortcut.chord.clone(),
        })
        .collect();
    for id in workspace_order(projects) {
        let Some((project, workspace)) = find_workspace(projects, id) else {
            continue;
        };
        items.push(CommandItem {
            id: format!("workspace:{id}"),
            title: format!("{} / {}", project.name, workspace.name).into(),
            group: "Workspaces".into(),
            icon: Some(IconName::Folder),
            shortcut: None,
        });
    }
    for agent in agents {
        items.push(CommandItem {
            id: format!("agent:{}", agent.id),
            title: agent_title(agent).into(),
            group: "Agents".into(),
            icon: Some(IconName::Bot),
            shortcut: None,
        });
    }
    items
}

// ----- connection and notifications -----

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Connection {
    Connecting,
    Connected,
    Disconnected(String),
}

impl Connection {
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

/// The text of Electron's host gate (`workspace-route-state-views.tsx`): a title and a
/// description line. Connected shows no gate.
pub fn host_gate(connection: &Connection, host: &str) -> Option<(String, String)> {
    match connection {
        Connection::Connected => None,
        Connection::Connecting => Some(("Connecting".into(), host.to_string())),
        Connection::Disconnected(_) => {
            Some((format!("{host} is offline"), "Host status: Offline".into()))
        }
    }
}

/// The label of a host option in the host switch.
pub fn host_option_label(host: &HostConfig, current: bool) -> String {
    let mut label = format!("Host {}, {}", host.name, host.url);
    if current {
        label.push_str(", selected");
    }
    label
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
/// else gives the default, the size of the reference Electron window.
pub fn window_size(spec: Option<&str>) -> (f32, f32) {
    const DEFAULT: (f32, f32) = (1200., 800.);
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

#[cfg(test)]
mod tests {
    use super::*;

    fn agent(id: &str, permissions: usize) -> AgentSummary {
        AgentSummary {
            id: id.into(),
            title: format!("title {id}"),
            status: "idle".into(),
            cwd: "/work/a".into(),
            workspace_id: None,
            provider: "mock".into(),
            model: None,
            pending_permissions: permissions,
        }
    }

    fn host(name: &str) -> HostConfig {
        HostConfig {
            name: name.into(),
            url: format!("ws://127.0.0.1/{name}"),
            password: None,
        }
    }

    fn workspace(id: &str, pinned: bool, agents: &[&str]) -> WorkspaceNode {
        WorkspaceNode {
            id: id.into(),
            name: format!("ws {id}"),
            cwd: format!("/work/{id}"),
            branch: None,
            pinned,
            agent_ids: agents.iter().map(|id| id.to_string()).collect(),
        }
    }

    fn projects() -> Vec<ProjectNode> {
        vec![
            ProjectNode {
                id: "p1".into(),
                name: "alpha".into(),
                root: "/work".into(),
                workspaces: vec![workspace("w1", false, &["a1"]), workspace("w2", true, &[])],
            },
            ProjectNode {
                id: "p2".into(),
                name: "beta".into(),
                root: "/beta".into(),
                workspaces: vec![workspace("w3", false, &["a2", "gone"])],
            },
        ]
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
    fn the_badge_sums_permissions_and_clears_at_zero() {
        let agents = [agent("a", 1), agent("b", 2), agent("c", 0)];
        assert_eq!(badge_count(&agents), 3);
        assert_eq!(badge_label(3), Some("3".into()));
        assert_eq!(badge_count(&[agent("a", 0)]), 0);
        assert_eq!(badge_label(0), None);
    }

    #[test]
    fn a_blank_title_falls_back_to_the_short_id() {
        let mut a = agent("0123456789", 0);
        a.title = "  ".into();
        assert_eq!(agent_title(&a), "Agent 01234567");
        assert_eq!(short_id("abc"), "abc");
    }

    #[test]
    fn file_keys_resolve_relative_paths_under_the_folder() {
        assert_eq!(absolute_path("/w/", "src/a.rs"), "/w/src/a.rs");
        assert_eq!(absolute_path("/w", "./a.rs"), "/w/a.rs");
        assert_eq!(absolute_path("/w", "/etc/x"), "/etc/x");
        assert_eq!(file_key("/w", "a.rs"), file_key("/x", "/w/a.rs"));
        assert_ne!(file_key("/w", "a.rs"), diff_key("/w", "a.rs"));
        assert_eq!(file_name("/w/src/a.rs"), "a.rs");
        assert_eq!(file_name("/"), "/");
        assert_eq!(agent_of_key(&agent_key("a1")), Some("a1"));
        assert_eq!(agent_of_key("file:/a"), None);
    }

    #[test]
    fn workspaces_follow_the_sidebar_order_with_pinned_first() {
        let projects = projects();
        assert_eq!(workspace_order(&projects), ["w2", "w1", "w3"]);
        let (project, found) = find_workspace(&projects, "w3").expect("found");
        assert_eq!(
            (project.name.as_str(), found.cwd.as_str()),
            ("beta", "/work/w3")
        );
        assert!(find_workspace(&projects, "nope").is_none());
    }

    #[test]
    fn workspace_navigation_wraps() {
        let order = ["w2", "w1", "w3"];
        assert_eq!(
            step_workspace(&order, Some("w3"), true).as_deref(),
            Some("w2")
        );
        assert_eq!(
            step_workspace(&order, Some("w2"), false).as_deref(),
            Some("w3")
        );
        assert_eq!(step_workspace(&order, None, false).as_deref(), Some("w2"));
        assert_eq!(step_workspace(&[], None, true), None);
        assert_eq!(step_index(3, Some(1), true), Some(2));
        assert_eq!(step_index(0, None, true), None);
    }

    #[test]
    fn a_workspace_opens_only_agents_the_list_still_has() {
        let projects = projects();
        let agents = [agent("a2", 0)];
        let (_, w3) = find_workspace(&projects, "w3").expect("found");
        assert_eq!(workspace_agents(w3, &agents), ["a2"]);
    }

    #[test]
    fn the_explorer_leaves_the_workspace_body_its_minimum() {
        assert_eq!(explorer_width(320., 1000.), 320.);
        // A 900 px window with a 320 px sidebar leaves 580 px.
        assert_eq!(explorer_width(320., 580.), 240.);
        assert_eq!(explorer_width(500., 800.), 400.);
        assert_eq!(explorer_width(100., 2000.), 240.);
        assert_eq!(explorer_width(300., 0.), 300.);
    }

    #[test]
    fn the_layout_reads_back_and_bad_fields_fall_back() {
        let prefs = LayoutPrefs {
            sidebar_open: false,
            explorer_open: true,
            explorer_width: 410.,
        };
        assert_eq!(LayoutPrefs::parse(&prefs.to_json()), prefs);
        assert_eq!(LayoutPrefs::parse("not json"), LayoutPrefs::default());
        let partial = LayoutPrefs::parse(r#"{"explorerOpen":true,"explorerWidth":-3}"#);
        assert!(partial.explorer_open && partial.sidebar_open);
        assert_eq!(partial.explorer_width, EXPLORER_WIDTH);
    }

    #[test]
    fn the_layout_file_round_trips() {
        let dir = std::env::temp_dir().join(format!("bachuc-layout-{}", std::process::id()));
        let prefs = LayoutPrefs {
            explorer_width: 333.,
            ..LayoutPrefs::default()
        };
        prefs.save_to(&dir).expect("saves");
        assert_eq!(LayoutPrefs::load_from(&dir), prefs);
        assert_eq!(
            LayoutPrefs::load_from(&dir.join("missing")),
            LayoutPrefs::default()
        );
    }

    #[test]
    fn the_ship_default_saves_per_repo_and_a_missing_value_is_pr() {
        let dir = std::env::temp_dir().join(format!("bachuc-ship-{}", std::process::id()));
        let _ = std::fs::remove_file(dir.join(SHIP_DEFAULTS_FILE));
        assert_eq!(load_ship_default(&dir, "/repo/a"), ShipDefault::Pr);
        save_ship_default(&dir, "/repo/a", ShipDefault::Merge).expect("saves");
        save_ship_default(&dir, "/repo/b", ShipDefault::Pr).expect("saves");
        assert_eq!(load_ship_default(&dir, "/repo/a"), ShipDefault::Merge);
        assert_eq!(load_ship_default(&dir, "/repo/b"), ShipDefault::Pr);
        assert_eq!(load_ship_default(&dir, "/repo/c"), ShipDefault::Pr);
        // An unreadable file or a bad value gives the default.
        std::fs::write(dir.join(SHIP_DEFAULTS_FILE), "not json").expect("writes");
        assert_eq!(load_ship_default(&dir, "/repo/a"), ShipDefault::Pr);
        std::fs::write(dir.join(SHIP_DEFAULTS_FILE), r#"{"/repo/a":7}"#).expect("writes");
        assert_eq!(load_ship_default(&dir, "/repo/a"), ShipDefault::Pr);
        assert_eq!(
            load_ship_default(&dir.join("missing"), "/repo/a"),
            ShipDefault::Pr
        );
    }

    #[test]
    fn a_script_row_shows_its_lifecycle() {
        let mut script = WorkspaceScript {
            name: "dev".into(),
            kind: "service".into(),
            lifecycle: "stopped".into(),
            health: None,
            terminal_id: None,
            exit_code: None,
        };
        assert_eq!(script_state(&script), "stopped");
        script.exit_code = Some(1);
        assert_eq!(script_state(&script), "exit 1");
        script.lifecycle = "running".into();
        assert_eq!(script_state(&script), "running");
        script.health = Some("healthy".into());
        assert_eq!(script_state(&script), "running, healthy");
    }

    #[test]
    fn open_in_editor_lists_installed_editors_and_builds_open_args() {
        let only_zed = editor_targets(|path| path == "/Applications/Zed.app");
        assert_eq!(
            only_zed,
            [
                EditorTarget::Zed,
                EditorTarget::Finder,
                EditorTarget::Terminal
            ]
        );
        assert_eq!(editor_targets(|_| true).len(), 5);
        assert_eq!(open_args(EditorTarget::Finder, "/w"), ["/w"]);
        assert_eq!(
            open_args(EditorTarget::VsCode, "/w"),
            ["-a", "Visual Studio Code", "/w"]
        );
        assert_eq!(
            open_args(EditorTarget::Terminal, "/w"),
            ["-a", "Terminal", "/w"]
        );
    }

    #[test]
    fn a_github_remote_gives_its_branch_page() {
        let tree = Some("https://github.com/o/r/tree/main".to_string());
        assert_eq!(github_url("git@github.com:o/r.git", Some("main")), tree);
        assert_eq!(github_url("https://github.com/o/r/", Some("main")), tree);
        assert_eq!(
            github_url("ssh://git@github.com/o/r.git", Some("main")),
            tree
        );
        assert_eq!(
            github_url("https://github.com/o/r.git", None).as_deref(),
            Some("https://github.com/o/r")
        );
        assert_eq!(github_url("git@gitlab.com:o/r.git", Some("main")), None);
        assert_eq!(github_url("https://github.com/o", None), None);
    }

    #[test]
    fn theme_and_mode_cycles_wrap() {
        assert_eq!(next_theme(ThemeChoice::Light), ThemeChoice::ClaudeDark);
        assert_eq!(next_theme(ThemeChoice::System), ThemeChoice::Light);
        let modes = vec![("a".to_string(), "A".to_string()), ("b".into(), "B".into())];
        assert_eq!(next_mode(&modes, Some("b")).as_deref(), Some("a"));
        assert_eq!(next_mode(&modes, None).as_deref(), Some("a"));
        assert_eq!(next_mode(&modes[..1], Some("a")), None);
    }

    #[test]
    fn the_command_center_lists_actions_workspaces_then_agents() {
        let shortcuts = bachuc_command::default_shortcuts();
        let items = command_items(&shortcuts, &projects(), &[agent("a1", 0)]);
        assert!(
            items
                .iter()
                .all(|item| item.id != format!("action:{JUMP_TO_TAB}"))
        );
        let first = &items[0];
        assert_eq!(first.id, "action:command-center.toggle");
        assert_eq!(first.shortcut.as_deref(), Some("Cmd+K"));
        let groups: Vec<&str> = items.iter().map(|item| item.group.as_ref()).collect();
        let workspaces = groups.iter().position(|g| *g == "Workspaces").expect("has");
        let agents = groups.iter().position(|g| *g == "Agents").expect("has");
        assert!(groups[..workspaces].iter().all(|g| *g == "Actions"));
        assert!(workspaces < agents);
        assert_eq!(items[workspaces].title.as_ref(), "alpha / ws w2");
        assert_eq!(items.last().map(|i| i.id.as_str()), Some("agent:a1"));
    }

    #[test]
    fn the_host_gate_names_the_host_until_connected() {
        let mut state = Connection::Connecting;
        assert_eq!(
            host_gate(&state, "live"),
            Some(("Connecting".into(), "live".into()))
        );
        state.apply(&ClientEvent::Disconnected {
            reason: "refused".into(),
        });
        assert_eq!(
            host_gate(&state, "live").expect("gate").0,
            "live is offline"
        );
        state.apply(&ClientEvent::AgentsChanged);
        assert!(matches!(state, Connection::Disconnected(_)));
        state.apply(&ClientEvent::Connected);
        assert_eq!(host_gate(&state, "live"), None);
        assert_eq!(
            host_option_label(&host("live"), true),
            "Host live, ws://127.0.0.1/live, selected"
        );
    }

    #[test]
    fn only_an_unfocused_window_notifies() {
        let agents = [agent("a1", 1)];
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
        assert_eq!(window_size(None), (1200., 800.));
        assert_eq!(window_size(Some("1280x800")), (1280., 800.));
        assert_eq!(window_size(Some(" 900 x 700 ")), (900., 700.));
        assert_eq!(window_size(Some("wide")), (1200., 800.));
        assert_eq!(window_size(Some("100x100")), (1200., 800.));
    }

    #[test]
    fn a_notification_tag_names_its_agent() {
        assert_eq!(agent_from_tag("permission-a1"), Some("a1"));
        assert_eq!(agent_from_tag("finished-a1"), Some("a1"));
        assert_eq!(agent_from_tag("other-a1"), None);
    }

    #[test]
    fn diff_tabs_have_stable_identities() {
        assert_eq!(all_diffs_key("/w/"), all_diffs_key("/w"));
        assert_ne!(all_diffs_key("/w"), all_diffs_key("/x"));
        let commit = commit_diff_key("/w", "abc1234ff", "src/a.rs");
        assert_eq!(commit, commit_diff_key("/w/", "abc1234ff", "./src/a.rs"));
        assert_eq!(commit, commit_diff_key("/x", "abc1234ff", "/w/src/a.rs"));
        assert_ne!(commit, commit_diff_key("/w", "def5678aa", "src/a.rs"));
        assert_ne!(commit, commit_diff_key("/w", "abc1234ff", "src/b.rs"));
        // The kinds never collide with each other or with the one-file diff and the file tab.
        let keys = [
            all_diffs_key("/w"),
            commit,
            diff_key("/w", "src/a.rs"),
            file_key("/w", "src/a.rs"),
        ];
        for (at, key) in keys.iter().enumerate() {
            assert!(keys[at + 1..].iter().all(|other| other != key));
        }
        assert_eq!(commit_diff_title("abc1234ff", "src/a.rs"), "a.rs (abc1234)");
        assert_eq!(file_of_key(&file_key("/w", "a.rs")), Some("/w/a.rs"));
        assert_eq!(file_of_key(&diff_key("/w", "a.rs")), None);
    }

    #[test]
    fn an_agent_tab_without_a_logo_shows_the_provider_fallback_icon() {
        assert_eq!(agent_icon(None), IconName::Bot);
        assert_eq!(agent_icon(Some("claude")), IconName::Bot);
        assert_eq!(agent_icon(Some("kiro")), IconName::PackagePlus);
    }

    #[test]
    fn the_logos_rebuild_only_when_the_fill_changes() {
        let light = gpui_kit::hsla(0., 0., 0.1, 1.);
        let dark = gpui_kit::hsla(0., 0., 0.9, 1.);
        let mut fill = LogoFill::new(light);
        // A font, size, or syntax change writes the theme but leaves the fill.
        assert!(!fill.changed(light));
        assert!(fill.changed(dark));
        assert!(!fill.changed(dark));
        assert!(fill.changed(light));
    }

    #[test]
    fn the_shell_refreshes_logos_only_from_the_theme_observer() {
        let shell = include_str!("shell.rs");
        // One call site, in the `Theme` observer; the client event handler returns early on
        // `TimelineChanged` before anything else, so a timeline update never rebuilds a logo.
        assert_eq!(shell.matches(".refresh_logos(").count(), 1);
        let observer = shell.find("observe_global::<Theme>").expect("observer");
        let call = shell.find(".refresh_logos(").expect("call");
        assert!(call > observer && call - observer < 200);
        let handler = shell.find("fn on_client_event").expect("handler");
        let early = shell[handler..]
            .find("ClientEvent::TimelineChanged { .. }")
            .expect("early return");
        assert!(!shell[handler..handler + early].contains("logo"));
    }
}
