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

/// Why an agent asks for attention (`attentionReason`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttentionReason {
    Finished,
    Error,
    Permission,
}

impl AttentionReason {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "finished" => Some(Self::Finished),
            "error" => Some(Self::Error),
            "permission" => Some(Self::Permission),
            _ => None,
        }
    }
}

/// The added and removed line counts of a workspace (`diffStat`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiffStat {
    pub additions: u32,
    pub deletions: u32,
}

/// `CheckoutStatus` with the git flag and the full file fields.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckoutDetails {
    pub is_git: bool,
    pub branch: Option<String>,
    pub files: Vec<ChangedFileDetails>,
}

/// One changed file with its rename and add or delete flags. `status` is `ok`, `too_large`,
/// or `binary`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChangedFileDetails {
    pub path: String,
    pub old_path: Option<String>,
    pub is_new: bool,
    pub is_deleted: bool,
    pub status: String,
    pub additions: u32,
    pub deletions: u32,
}

/// A provider feature of an agent, such as a toggle or a select in the composer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentFeature {
    pub id: String,
    pub label: String,
    pub description: Option<String>,
    pub tooltip: Option<String>,
    pub icon: Option<String>,
    pub kind: AgentFeatureKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentFeatureKind {
    Toggle {
        value: bool,
    },
    Select {
        value: Option<String>,
        options: Vec<AgentFeatureOption>,
    },
}

/// One choice of a select feature.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentFeatureOption {
    pub id: String,
    pub label: String,
    pub description: Option<String>,
    pub is_default: Option<bool>,
    pub metadata: serde_json::Map<String, serde_json::Value>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileDiffStatus {
    Ok,
    TooLarge,
    Binary,
}

/// The diff of one file. `text` is the unified diff of an `Ok` file and empty otherwise.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileDiff {
    pub path: String,
    pub old_path: Option<String>,
    pub is_new: bool,
    pub is_deleted: bool,
    pub status: FileDiffStatus,
    pub additions: u32,
    pub deletions: u32,
    pub text: String,
}

/// The commits of a checkout since its base.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitList {
    pub base_ref: Option<String>,
    pub commits: Vec<CommitSummary>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitSummary {
    pub sha: String,
    pub short_sha: String,
    pub subject: String,
    pub author_name: String,
    pub author_date: String,
    /// False for a local-only commit.
    pub is_on_remote: bool,
    pub is_on_base: Option<bool>,
    pub files: Vec<CommitFile>,
}

/// One file of a commit. `status` is `added`, `modified`, `deleted`, or `renamed`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitFile {
    pub path: String,
    pub additions: u32,
    pub deletions: u32,
    pub status: Option<String>,
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
    /// The wire `kind` is "skill".
    pub skill: bool,
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

// ---- composer, New workspace, and tab menu data (TASK-169) ----

/// The daemon's metrics of one finished turn (`TurnMetricsSchema`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TurnMetrics {
    pub ttft_ms: Option<f64>,
    pub output_tokens: Option<u64>,
    pub output_tokens_per_second: Option<f64>,
}

/// The running turn of an agent (snapshot `activeTurn`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActiveTurn {
    pub turn_id: String,
    pub started_at: Option<String>,
}

/// The token use of an agent's last turn (snapshot `lastUsage`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AgentUsage {
    pub input_tokens: Option<u64>,
    pub cached_input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub total_cost_usd: Option<f64>,
    pub context_window_max_tokens: Option<u64>,
    pub context_window_used_tokens: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
}

/// One item of the agent's newest `todo` timeline item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskEntry {
    pub id: Option<String>,
    pub text: String,
    pub completed: bool,
    pub status: Option<TaskStatus>,
    pub active_form: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubagentKind {
    /// A Paseo agent whose `paseo.parent-agent-id` label names the parent.
    Paseo,
    /// A provider-owned child (`agent.provider_subagents.*`).
    Provider,
}

/// One row of the composer track strip.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubagentRow {
    pub id: String,
    pub kind: SubagentKind,
    pub provider: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub subtitle: Option<String>,
    pub status: String,
    pub requires_attention: bool,
    pub created_at: String,
}

/// A prompt with its images (`{data, mimeType}`) and attachments (`AgentAttachment` objects).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OutgoingMessage {
    pub text: String,
    pub images: Vec<serde_json::Value>,
    pub attachments: Vec<serde_json::Value>,
}

/// The agent settings of a draft that has no agent yet.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DraftConfig {
    pub provider: String,
    pub cwd: String,
    pub model: Option<String>,
    pub mode_id: Option<String>,
    pub thinking_option_id: Option<String>,
}

/// A `workspace.create.request`.
#[derive(Clone, Debug, PartialEq)]
pub struct CreateWorkspace {
    pub source: WorkspaceSource,
    pub title: Option<String>,
    pub first_agent_prompt: Option<String>,
    pub agent: Option<InitialAgent>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkspaceSource {
    /// The workspace runs in this folder.
    Directory {
        path: String,
        project_id: Option<String>,
    },
    /// The daemon creates a worktree of the repo at `cwd`.
    Worktree {
        cwd: String,
        project_id: Option<String>,
        action: Option<WorktreeAction>,
        ref_name: Option<String>,
        base_branch: Option<String>,
        branch_name: Option<String>,
        worktree_slug: Option<String>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorktreeAction {
    BranchOff,
    Checkout,
}

/// The first agent of a new workspace, or a new agent with its full config.
#[derive(Clone, Debug, PartialEq)]
pub struct InitialAgent {
    pub config: DraftConfig,
    pub feature_values: std::collections::BTreeMap<String, serde_json::Value>,
    pub env: std::collections::BTreeMap<String, String>,
    pub initial_prompt: String,
    pub images: Vec<serde_json::Value>,
    pub attachments: Vec<serde_json::Value>,
    pub labels: std::collections::BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CreatedWorkspace {
    pub workspace_id: String,
    pub agent_id: Option<String>,
    pub setup_terminal_id: Option<String>,
}

/// A branch of `branch_suggestions`; the detail fields are `None` without `branchDetails`.
#[derive(Clone, Debug, PartialEq)]
pub struct BranchSuggestion {
    pub name: String,
    pub committer_date: Option<f64>,
    pub has_local: Option<bool>,
    pub has_remote: Option<bool>,
    pub local_ahead: Option<u32>,
    pub local_behind: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BranchValidation {
    pub exists: bool,
    pub resolved_ref: Option<String>,
    pub is_remote: bool,
}

/// A project that `project.add` registered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AddedProject {
    pub id: String,
    pub name: String,
    pub root: String,
    /// `projectKey`, else `["<serverId>","<projectId>"]`, as `Client::project_view_key`.
    pub view_key: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiffMode {
    Uncommitted,
    Base,
}

/// What `checkout.diff.get` compares against.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffCompare {
    pub mode: DiffMode,
    pub base_ref: Option<String>,
    pub ignore_whitespace: bool,
}

/// The provider CLI command that resumes a session, from the app's
/// `provider-command-templates.ts`. `None` for a provider without a template.
pub fn resume_command(provider: &str, session_id: &str) -> Option<String> {
    let template = match provider {
        "codex" => "codex resume {sessionId}",
        "claude" => "claude --resume {sessionId}",
        "hermes" => "hermes --resume {sessionId}",
        "pi" => "pi --session {sessionId}",
        "omp" => "omp --session {sessionId}",
        "opencode" => "opencode --session {sessionId}",
        _ => return None,
    };
    Some(template.replace("{sessionId}", session_id))
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

// ---- git, PR, stash, and script calls (TASK-204) ----

/// A pull request merge method (`mergeMethod`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MergeMethod {
    Merge,
    Squash,
    Rebase,
}

impl MergeMethod {
    pub(crate) fn wire(self) -> &'static str {
        match self {
            Self::Merge => "merge",
            Self::Squash => "squash",
            Self::Rebase => "rebase",
        }
    }

    /// GitHub's upper-case method name, as `viewerDefaultMergeMethod` and
    /// `autoMergeRequest.mergeMethod` carry it.
    fn from_github(value: &str) -> Option<Self> {
        match value {
            "MERGE" => Some(Self::Merge),
            "SQUASH" => Some(Self::Squash),
            "REBASE" => Some(Self::Rebase),
            _ => None,
        }
    }
}

/// A git or PR action of the checkout header, as the app's `actions-store.ts` runs it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GitAction {
    /// Commits every change (`addAll`).
    Commit,
    Pull,
    Push,
    /// Pull, then push.
    PullAndPush,
    /// A PR with the daemon's title and body.
    CreatePr,
    MergePr(MergeMethod),
    SetAutoMerge {
        enabled: bool,
        method: Option<MergeMethod>,
    },
    /// Merges the branch into its base; the base checkout must be clean.
    MergeBranch,
    MergeFromBase,
    Refresh,
}

/// What a git action gives back; only `CreatePr` sets the fields.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GitActionReply {
    pub url: Option<String>,
    pub number: Option<u64>,
}

/// The full git status of a folder. A folder outside git has `is_git: false`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GitStatus {
    pub is_git: bool,
    pub branch: Option<String>,
    pub is_dirty: bool,
    pub base_ref: Option<String>,
    /// Commits ahead of and behind the base; 0 when the daemon sends no counts.
    pub ahead: u32,
    pub behind: u32,
    pub ahead_of_origin: u32,
    pub behind_of_origin: u32,
    pub has_remote: bool,
    pub remote_url: Option<String>,
    pub is_paseo_owned_worktree: bool,
    /// The branch tracks an upstream. The daemon sends a null `aheadOfOrigin` without one,
    /// also for a deleted upstream branch, and the app's git policy reads it that way.
    pub has_upstream: bool,
}

impl GitStatus {
    pub(crate) fn parse(payload: &serde_json::Value) -> Self {
        let count = |value: &serde_json::Value| value.as_u64().unwrap_or(0) as u32;
        let text = |value: &serde_json::Value| value.as_str().map(str::to_string);
        Self {
            is_git: payload["isGit"] == true,
            branch: text(&payload["currentBranch"]),
            is_dirty: payload["isDirty"] == true,
            base_ref: text(&payload["baseRef"]),
            ahead: count(&payload["aheadBehind"]["ahead"]),
            behind: count(&payload["aheadBehind"]["behind"]),
            ahead_of_origin: count(&payload["aheadOfOrigin"]),
            behind_of_origin: count(&payload["behindOfOrigin"]),
            has_remote: payload["hasRemote"] == true,
            remote_url: text(&payload["remoteUrl"]),
            is_paseo_owned_worktree: payload["isPaseoOwnedWorktree"] == true,
            has_upstream: payload["aheadOfOrigin"].is_number(),
        }
    }
}

/// What the git policy may do with a PR, from the `github` facts as the app's
/// `deriveGithubMergeCapability` reads them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PrMergeCapability {
    pub auto_merge_enabled: bool,
    pub auto_merge_method: Option<MergeMethod>,
    pub can_enable_auto_merge: bool,
    pub can_disable_auto_merge: bool,
    /// In the order merge, squash, rebase.
    pub allowed_methods: Vec<MergeMethod>,
    pub merge_state_status: Option<String>,
}

impl PrMergeCapability {
    /// A PR without a `github` block (another forge) gets no capability.
    pub(crate) fn parse(github: &serde_json::Value) -> Self {
        let repository = &github["repository"];
        let allowed_methods = [
            ("mergeCommitAllowed", MergeMethod::Merge),
            ("squashMergeAllowed", MergeMethod::Squash),
            ("rebaseMergeAllowed", MergeMethod::Rebase),
        ]
        .into_iter()
        .filter(|(key, _)| repository[*key] == true)
        .map(|(_, method)| method)
        .collect();
        let merge_state_status = github["mergeStateStatus"].as_str().map(str::to_string);
        let auto_merge = &github["autoMergeRequest"];
        Self {
            auto_merge_enabled: auto_merge.is_object(),
            auto_merge_method: auto_merge["mergeMethod"]
                .as_str()
                .and_then(MergeMethod::from_github),
            can_enable_auto_merge: merge_state_status.as_deref() == Some("BLOCKED")
                && repository["autoMergeAllowed"] == true
                && github["viewerCanEnableAutoMerge"] == true,
            can_disable_auto_merge: github["viewerCanDisableAutoMerge"] == true,
            allowed_methods,
            merge_state_status,
        }
    }
}

/// One CI check of a PR.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrCheck {
    pub name: String,
    pub status: String,
    pub url: Option<String>,
    pub workflow: Option<String>,
    /// The forge's display text, such as `2m 5s`.
    pub duration: Option<String>,
    /// Forge-neutral refinements, such as `required`; unknown ones are kept.
    pub traits: Vec<String>,
}

/// The PR of a checkout's branch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrStatus {
    /// Optional on the wire.
    pub number: Option<u64>,
    pub url: String,
    pub title: String,
    pub state: String,
    pub is_merged: bool,
    pub is_draft: bool,
    /// `MERGEABLE`, `CONFLICTING`, or `UNKNOWN`.
    pub mergeable: Option<String>,
    pub checks: Vec<PrCheck>,
    pub review_decision: Option<String>,
    pub repo_owner: String,
    pub repo_name: String,
    pub merge: PrMergeCapability,
}

impl PrStatus {
    pub(crate) fn parse(status: &serde_json::Value) -> Option<Self> {
        let text = |value: &serde_json::Value| value.as_str().map(str::to_string);
        let checks = status["checks"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|check| {
                Some(PrCheck {
                    name: check["name"].as_str()?.to_string(),
                    status: check["status"].as_str().unwrap_or_default().to_string(),
                    url: text(&check["url"]),
                    workflow: text(&check["workflow"]),
                    duration: text(&check["duration"]),
                    traits: check["traits"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|item| item.as_str().map(str::to_string))
                        .collect(),
                })
            })
            .collect();
        Some(Self {
            number: status["number"].as_u64(),
            url: status["url"].as_str()?.to_string(),
            title: status["title"].as_str().unwrap_or_default().to_string(),
            state: status["state"].as_str().unwrap_or_default().to_string(),
            is_merged: status["isMerged"] == true,
            is_draft: status["isDraft"] == true,
            mergeable: text(&status["mergeable"]),
            checks,
            review_decision: text(&status["reviewDecision"]),
            repo_owner: status["repoOwner"].as_str().unwrap_or_default().to_string(),
            repo_name: status["repoName"].as_str().unwrap_or_default().to_string(),
            merge: PrMergeCapability::parse(&status["github"]),
        })
    }
}

/// A `checkout_pr_status` answer. `status` is `None` when the branch has no PR.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrStatusReply {
    pub status: Option<PrStatus>,
    pub github_features_enabled: bool,
}

/// One review or comment of a PR timeline.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PrTimelineItem {
    Review {
        author: String,
        body: String,
        created_at_ms: i64,
        url: Option<String>,
        /// `approved`, `changes_requested`, or `commented`.
        state: String,
    },
    Comment {
        author: String,
        body: String,
        created_at_ms: i64,
        url: Option<String>,
        /// The file and line of an inline review comment.
        path: Option<String>,
        line: Option<u32>,
    },
}

impl PrTimelineItem {
    /// An item of another kind reads as a comment, as the protocol schema does.
    pub(crate) fn parse(item: &serde_json::Value) -> Self {
        let author = item["author"].as_str().unwrap_or("unknown").to_string();
        let body = item["body"].as_str().unwrap_or_default().to_string();
        let created_at_ms = item["createdAt"].as_f64().unwrap_or_default() as i64;
        let url = item["url"]
            .as_str()
            .filter(|url| !url.is_empty())
            .map(str::to_string);
        if item["kind"] == "review" {
            return Self::Review {
                author,
                body,
                created_at_ms,
                url,
                state: item["reviewState"]
                    .as_str()
                    .unwrap_or("commented")
                    .to_string(),
            };
        }
        Self::Comment {
            author,
            body,
            created_at_ms,
            url,
            path: item["location"]["path"].as_str().map(str::to_string),
            line: item["location"]["line"].as_u64().map(|line| line as u32),
        }
    }
}

/// One `git stash` entry; `index` is the zero-based stash index.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StashEntry {
    pub index: u32,
    pub message: String,
    pub branch: Option<String>,
    /// Paseo made the stash.
    pub is_paseo: bool,
}

impl StashEntry {
    pub(crate) fn parse(entry: &serde_json::Value) -> Option<Self> {
        Some(Self {
            index: entry["index"].as_u64()? as u32,
            message: entry["message"].as_str().unwrap_or_default().to_string(),
            branch: entry["branch"].as_str().map(str::to_string),
            is_paseo: entry["isPaseo"] == true,
        })
    }
}

/// One script or service of a workspace's `paseo.json`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceScript {
    pub name: String,
    /// `script` or `service`.
    pub kind: String,
    /// `running` or `stopped`.
    pub lifecycle: String,
    /// `healthy` or `unhealthy`; `None` while unknown.
    pub health: Option<String>,
    pub terminal_id: Option<String>,
    pub exit_code: Option<i32>,
    pub proxy_url: Option<String>,
    pub local_proxy_url: Option<String>,
    pub public_proxy_url: Option<String>,
}

impl WorkspaceScript {
    pub(crate) fn parse_list(scripts: &serde_json::Value) -> Vec<Self> {
        scripts
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|script| {
                Some(Self {
                    name: script["scriptName"].as_str()?.to_string(),
                    kind: script["type"].as_str().unwrap_or("service").to_string(),
                    lifecycle: script["lifecycle"]
                        .as_str()
                        .unwrap_or("stopped")
                        .to_string(),
                    health: script["health"].as_str().map(str::to_string),
                    terminal_id: script["terminalId"].as_str().map(str::to_string),
                    exit_code: script["exitCode"].as_i64().map(|code| code as i32),
                    proxy_url: script["proxyUrl"].as_str().map(str::to_string),
                    local_proxy_url: script["localProxyUrl"].as_str().map(str::to_string),
                    public_proxy_url: script["publicProxyUrl"].as_str().map(str::to_string),
                })
            })
            .collect()
    }
}

/// Git and script status changes. `Client` emits these next to `DirectoryEvent`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GitStatusEvent {
    /// The git status of `cwd` changed (`checkout_status_update`).
    Changed { cwd: String },
    /// The scripts of a workspace changed (`script_status_update` or a workspace update).
    ScriptsChanged { workspace_id: String },
}

/// One agent row of the agent history, archived agents included.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HistoryEntry {
    pub agent_id: String,
    pub title: String,
    pub provider: String,
    pub workspace_id: Option<String>,
    pub workspace_name: Option<String>,
    pub project_name: String,
    pub branch: Option<String>,
    pub cwd: String,
    pub archived: bool,
    /// The snapshot `updatedAt`, as the app's history list reads it.
    pub last_activity_at: Option<String>,
    pub pending_permissions: usize,
}

/// One page of `fetch_agent_history_request`, newest first.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HistoryPage {
    pub entries: Vec<HistoryEntry>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
    /// An older daemon cut a search short instead of giving a cursor.
    pub search_truncated: bool,
}

/// A timeline position, as `AgentTimelineCursorSchema`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForkCursor {
    pub epoch: String,
    pub seq: u64,
}

/// Where a fork ends; both `None` forks the whole chat.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ForkBoundary {
    pub cursor: Option<ForkCursor>,
    pub message_id: Option<String>,
}

/// The chat history of a fork. Send `attachment` unchanged with the new agent's prompt.
#[derive(Clone, Debug, PartialEq)]
pub struct ForkContext {
    pub attachment: serde_json::Value,
    pub title: Option<String>,
    pub item_count: u32,
}

/// One user prompt of the daemon's prompt index (`agent.timeline.list_prompts.response`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromptEntry {
    pub seq: u64,
    pub preview: String,
}

/// The prompts of one agent timeline in seq order, with the epoch they belong to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PromptIndex {
    pub epoch: String,
    pub prompts: Vec<PromptEntry>,
}

/// How `directory_suggestions_request` matches `query`: `suffix` matches the end of a path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatchMode {
    Fuzzy,
    Suffix,
}

impl MatchMode {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Fuzzy => "fuzzy",
            Self::Suffix => "suffix",
        }
    }
}

/// One `directory_suggestions_request` match; `path` is as the daemon gives it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirectorySuggestion {
    pub path: String,
    pub is_directory: bool,
}

impl DirectorySuggestion {
    /// The `entries`, or the `directories` of a host that sends only those, as the app reads
    /// them (`use-agent-autocomplete.ts` `mapDirectorySuggestionsToEntries`).
    pub(crate) fn parse_all(payload: &serde_json::Value) -> Vec<Self> {
        let entries: Vec<Self> = payload["entries"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|entry| {
                let is_directory = match entry["kind"].as_str()? {
                    "file" => false,
                    "directory" => true,
                    _ => return None,
                };
                Some(Self {
                    path: entry["path"].as_str()?.to_string(),
                    is_directory,
                })
            })
            .collect();
        if !entries.is_empty() {
            return entries;
        }
        payload["directories"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|path| {
                Some(Self {
                    path: path.as_str()?.to_string(),
                    is_directory: true,
                })
            })
            .collect()
    }
}

/// What `fs.entry.create.request` makes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NewEntryKind {
    File,
    Directory,
}

impl NewEntryKind {
    pub(crate) fn wire(self) -> &'static str {
        match self {
            NewEntryKind::File => "file",
            NewEntryKind::Directory => "directory",
        }
    }
}
