//! Directory state the UI reads: agents, workspaces, providers, and agent controls.

/// What the client tells the UI.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClientEvent {
    /// The daemon answered `hello` with `server_info`.
    Connected,
    /// The socket closed or could not open. A rejected `hello` gives its reason, for example
    /// `incorrect_password`. The client keeps retrying with backoff.
    Disconnected { reason: String },
    /// Agents, workspaces, or providers changed.
    AgentsChanged,
    /// A viewed timeline changed. Sent at most every 50 ms per batch.
    TimelineChanged { agent_id: String },
    /// An agent asks for a new permission.
    PermissionRequested { agent_id: String },
    /// An agent left the running status.
    AgentFinished { agent_id: String },
}

/// One agent in the directory. Archived agents are left out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentSummary {
    pub id: String,
    pub title: String,
    pub status: String,
    pub cwd: String,
    pub workspace_id: Option<String>,
    pub provider: String,
    pub model: Option<String>,
    pub pending_permissions: usize,
}

/// One workspace; `cwd` is its folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceSummary {
    pub id: String,
    pub name: String,
    pub cwd: String,
}

/// One provider with its selectable models as `(id, label)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProviderSummary {
    pub id: String,
    pub label: String,
    pub available: bool,
    pub models: Vec<(String, String)>,
}

/// Mode and model choices of one agent, as `(id, label)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentControls {
    pub modes: Vec<(String, String)>,
    pub current_mode: Option<String>,
    pub models: Vec<(String, String)>,
    pub current_model: Option<String>,
    pub running: bool,
}

/// Project tree and checkout changes. `Client` emits these next to `ClientEvent`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DirectoryEvent {
    /// `Client::projects` changed.
    ProjectsChanged,
    /// The changed files of a watched folder changed.
    CheckoutChanged { cwd: String },
}

/// One project of the sidebar tree, in the daemon's project order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectNode {
    pub id: String,
    pub name: String,
    pub root: String,
    pub workspaces: Vec<WorkspaceNode>,
}

/// One workspace of a project. `agent_ids` follow the agent list order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceNode {
    pub id: String,
    pub name: String,
    pub cwd: String,
    pub branch: Option<String>,
    pub pinned: bool,
    pub agent_ids: Vec<String>,
}

/// One entry of a directory listing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileKind {
    Text,
    Image,
    Binary,
}

/// A read file. `text` is set for `Text`, `bytes` for `Image` and `Binary`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileContent {
    pub path: String,
    pub kind: FileKind,
    pub text: Option<String>,
    pub bytes: Option<Vec<u8>>,
    pub mime_type: Option<String>,
    pub size: u64,
}

/// The branch and uncommitted changes of a folder. A folder outside git has no branch and no
/// files.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckoutStatus {
    pub branch: Option<String>,
    pub files: Vec<ChangedFile>,
}

/// One changed file. `status` is `ok`, `too_large`, or `binary`; only `ok` has a diff.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChangedFile {
    pub path: String,
    pub status: String,
    pub added: u32,
    pub removed: u32,
}

/// Thinking choices of an agent's model, as `(id, label)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThinkingControls {
    pub options: Vec<(String, String)>,
    pub current: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlashCommand {
    pub name: String,
    pub description: String,
}

/// What a prompt sent during a running turn does to that turn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TurnBehavior {
    Interrupt,
    Steer,
}

impl TurnBehavior {
    pub(crate) fn wire(self) -> &'static str {
        match self {
            TurnBehavior::Interrupt => "interrupt",
            TurnBehavior::Steer => "steer",
        }
    }
}

/// A prompt the daemon holds until the running turn ends.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueuedPrompt {
    pub id: String,
    pub text: String,
}

/// The display name of a provider id. The provider list RPC has no labels.
pub(crate) fn provider_label(id: &str) -> String {
    match id {
        "claude" => "Claude".into(),
        "codex" => "Codex".into(),
        "copilot" => "Copilot".into(),
        "opencode" => "OpenCode".into(),
        "pi" => "Pi".into(),
        "glm" => "GLM".into(),
        "mock" => "Mock".into(),
        other => {
            let mut chars = other.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect(),
                None => String::new(),
            }
        }
    }
}
