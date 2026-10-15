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
