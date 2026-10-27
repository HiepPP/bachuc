//! The client state machine, free of GPUI: it turns worker events into state and requests.
//! `Client` wraps it in a GPUI entity; tests drive it directly.

use crate::composer_data::{self, PARENT_AGENT_LABEL, ProviderSubagent};
use crate::connection::{Connection, ConnectionConfig, WorkerEvent, session_frame};
use crate::files::{self, TransferFrame};
use crate::model::{
    ActiveTurn, AddedProject, AgentUsage, BranchSuggestion, BranchValidation, CreateWorkspace,
    CreatedWorkspace, DiffCompare, DraftConfig, InitialAgent, OutgoingMessage, SubagentKind,
    SubagentRow, TaskEntry, TurnMetrics,
};
use crate::model::{
    AgentControls, AgentFeature, AgentFeatureKind, AgentFeatureOption, AgentSummary,
    AttentionReason, BackgroundProcess, ChangedFile, CheckoutDetails, CheckoutStatus, ClientEvent,
    CommitList, DiffStat, DirectoryEvent, FileContent, FileDiff, FileEntry, ProjectNode,
    ProviderSummary, QueuedPrompt, SlashCommand, ThinkingControls, TurnBehavior, WorkspaceNode,
    WorkspaceSummary, provider_label,
};
use crate::model::{DirectorySuggestion, MatchMode, NewEntryKind, SkillHookUsage};
use crate::model::{ForkBoundary, ForkContext, HistoryEntry, HistoryPage};
use crate::model::{
    GitAction, GitActionReply, GitStatus, GitStatusEvent, PrStatus, PrStatusReply, PrTimelineItem,
    StashEntry, WorkspaceScript,
};
use crate::model::{PromptEntry, PromptIndex};
use crate::model::{
    PullRequestState, WorkspaceLabel, WorkspacePullRequest, parse_iso_ms, select_check_summary,
};
use crate::model::{RewindMode, RewindModes};
use crate::plugins::{self, PluginEvent, PluginListItem, PluginSettingsRead, PluginSettingsWrite};
use crate::sent_images::{self, SentImage};
use crate::terminal::{self, TerminalFrame, TerminalHandle};
use crate::timeline::{FetchPlan, LiveRow, TimelineState, TimelineSync};
use crate::{parse_provider_subagent_key, provider_subagent_key};
use anyhow::anyhow;
use bachuc_protocol::ServerInfo;
use bachuc_protocol::generated::UpsertAgentFeaturesItem;
use futures::channel::{mpsc, oneshot};
use serde::Deserialize as _;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::PathBuf;

type Reply<T> = oneshot::Sender<anyhow::Result<T>>;

/// A receiver that already holds `Err(message)`, for a call that sends no request.
fn failed<T>(message: &'static str) -> oneshot::Receiver<anyhow::Result<T>> {
    let (reply, receiver) = oneshot::channel();
    let _ = reply.send(Err(anyhow!(message)));
    receiver
}

/// A `directory_suggestions_request` without `matchMode`; `limit` is clamped to 1 to 100.
fn directory_suggestions_message(
    cwd: &str,
    query: &str,
    include_files: bool,
    include_directories: bool,
    limit: u32,
) -> Value {
    json!({"type": "directory_suggestions_request", "cwd": cwd, "query": query,
           "includeFiles": include_files, "includeDirectories": include_directories,
           "limit": limit.clamp(1, 100)})
}

/// Takes a plugin RPC `output`, or the error, and answers the caller with its decoded form.
type RpcFinish = Box<dyn FnOnce(anyhow::Result<Value>) + Send>;

/// The diff the changed files come from, as the app's working diff shows it by default.
const UNCOMMITTED: &str = "uncommitted";

enum Pending {
    Projects,
    FileList(Reply<Vec<FileEntry>>),
    /// `transfer` holds the `FileBegin` metadata and the chunks so far.
    ReadFile {
        path: String,
        reply: Reply<FileContent>,
        transfer: Option<Box<(Value, Vec<u8>)>>,
    },
    CheckoutStatus(CheckoutReply),
    CheckoutFiles {
        branch: Option<String>,
        reply: CheckoutReply,
    },
    CheckoutWatch(String),
    FileDiff {
        path: String,
        reply: Reply<String>,
    },
    CheckoutDiff(Reply<Vec<FileDiff>>),
    Commits(Reply<CommitList>),
    CommitFileDiff(Reply<Option<FileDiff>>),
    SetFeature(Reply<()>),
    Commands(Reply<Vec<SlashCommand>>),
    /// A queue request; its answer replaces the stored queue. `reply` waits for the answer.
    Queue {
        agent_id: String,
        reply: Option<Reply<()>>,
        /// The images of an enqueue, saved under the item id the answer gives.
        images: Vec<SentImage>,
    },
    ReadConfig {
        root: String,
        reply: Reply<Value>,
    },
    WriteConfig {
        root: String,
        reply: Reply<()>,
    },
    Agents {
        first: bool,
    },
    Workspaces {
        first: bool,
    },
    Providers,
    Models(String),
    /// A provider snapshot refresh; its answer asks for these providers' models again.
    RefreshModels(Vec<String>),
    Timeline {
        agent_id: String,
        plan: FetchPlan,
        generation: u64,
    },
    /// A child timeline page; `key` is its `provider_subagent_key`.
    ProviderTimeline {
        key: String,
        plan: FetchPlan,
        generation: u64,
    },
    TimelineSubscription,
    CreateAgent(Reply<String>),
    CreateTerminal(Reply<TerminalHandle>),
    SubscribeTerminal {
        terminal_id: String,
        reply: Option<(
            Reply<TerminalHandle>,
            mpsc::UnboundedReceiver<TerminalFrame>,
        )>,
    },
    Action(&'static str),
    /// A `project.remove.response`: `accepted: true` drops the project, anything else reports it.
    ProjectRemove(String),
    PluginList(Reply<Vec<PluginListItem>>),
    PluginCatalog(Reply<Vec<String>>),
    PluginRpc(RpcFinish),
    /// An `AgentActionResponsePayloadSchema` answer: `accepted: false` fails with `error`.
    Accepted(Reply<()>),
    ProviderSubagents(String),
    Modes(Reply<Vec<(String, String)>>),
    DraftFeatures(Reply<Vec<AgentFeature>>),
    /// `key` is set when the request subscribed to its creation updates.
    CreateWorkspace {
        key: Option<String>,
        reply: Reply<CreatedWorkspace>,
    },
    BranchSuggestions(Reply<Vec<BranchSuggestion>>),
    ValidateBranch(Reply<BranchValidation>),
    AddProject(Reply<AddedProject>),
    Refresh(Reply<()>),
    RenameTerminal(Reply<()>),
    GitAction(Reply<GitActionReply>),
    /// The pull of a pull and push; the push follows when it succeeds.
    PullThenPush {
        cwd: String,
        reply: Reply<GitActionReply>,
    },
    GitStatus(Reply<GitStatus>),
    PrStatus(Reply<PrStatusReply>),
    PrTimeline(Reply<Vec<PrTimelineItem>>),
    StashList(Reply<Vec<StashEntry>>),
    /// A checkout answer with only `success` and `error`.
    CheckoutDone(Reply<()>),
    StartScript(Reply<String>),
    History(Reply<HistoryPage>),
    FetchAgent(Reply<()>),
    ForkContext(Reply<ForkContext>),
    Prompts(Reply<PromptIndex>),
    /// The `workspace.label.list` answer; it replaces or patches the label catalog.
    LabelList,
    /// An `agent.rewind` answer; a rewind of the conversation reloads the timeline tail.
    Rewind {
        agent_id: String,
        mode: RewindMode,
        reply: Reply<()>,
    },
    DirectorySuggestions(Reply<Vec<DirectorySuggestion>>),
    /// An `fs.entry.*` answer with `success` and a string `error`.
    FsEntry(Reply<()>),
}

impl Pending {
    /// Fails the reply this request carries, if any.
    fn fail(self, error: anyhow::Error) {
        fn send<T>(reply: Reply<T>, error: anyhow::Error) {
            let _ = reply.send(Err(error));
        }
        match self {
            Pending::CreateAgent(reply) => send(reply, error),
            Pending::CreateTerminal(reply) => send(reply, error),
            Pending::SubscribeTerminal {
                reply: Some((reply, _)),
                ..
            } => send(reply, error),
            Pending::FileList(reply) => send(reply, error),
            Pending::ReadFile { reply, .. } => send(reply, error),
            Pending::CheckoutStatus(reply) | Pending::CheckoutFiles { reply, .. } => {
                reply.send(Err(error))
            }
            Pending::FileDiff { reply, .. } => send(reply, error),
            Pending::CheckoutDiff(reply) => send(reply, error),
            Pending::Commits(reply) => send(reply, error),
            Pending::CommitFileDiff(reply) => send(reply, error),
            Pending::SetFeature(reply) => send(reply, error),
            Pending::Commands(reply) => send(reply, error),
            Pending::ReadConfig { reply, .. } => send(reply, error),
            Pending::WriteConfig { reply, .. } => send(reply, error),
            Pending::PluginList(reply) => send(reply, error),
            Pending::PluginCatalog(reply) => send(reply, error),
            Pending::PluginRpc(finish) => finish(Err(error)),
            Pending::Accepted(reply) => send(reply, error),
            Pending::Rewind { reply, .. } => send(reply, error),
            Pending::Queue {
                reply: Some(reply), ..
            } => send(reply, error),
            Pending::Modes(reply) => send(reply, error),
            Pending::DraftFeatures(reply) => send(reply, error),
            Pending::CreateWorkspace { reply, .. } => send(reply, error),
            Pending::BranchSuggestions(reply) => send(reply, error),
            Pending::ValidateBranch(reply) => send(reply, error),
            Pending::AddProject(reply) => send(reply, error),
            Pending::Refresh(reply) => send(reply, error),
            Pending::RenameTerminal(reply) => send(reply, error),
            Pending::GitAction(reply) | Pending::PullThenPush { reply, .. } => send(reply, error),
            Pending::GitStatus(reply) => send(reply, error),
            Pending::PrStatus(reply) => send(reply, error),
            Pending::PrTimeline(reply) => send(reply, error),
            Pending::StashList(reply) => send(reply, error),
            Pending::CheckoutDone(reply) => send(reply, error),
            Pending::StartScript(reply) => send(reply, error),
            Pending::History(reply) => send(reply, error),
            Pending::FetchAgent(reply) => send(reply, error),
            Pending::ForkContext(reply) => send(reply, error),
            Pending::Prompts(reply) => send(reply, error),
            Pending::DirectorySuggestions(reply) => send(reply, error),
            Pending::FsEntry(reply) => send(reply, error),
            _ => {}
        }
    }
}

/// Where a checkout status answer goes: `checkout_status` or `checkout_details`.
enum CheckoutReply {
    Status(Reply<CheckoutStatus>),
    Details(Reply<CheckoutDetails>),
}

impl CheckoutReply {
    fn send(self, result: anyhow::Result<CheckoutDetails>) {
        match self {
            CheckoutReply::Status(reply) => {
                let _ = reply.send(result.map(|details| CheckoutStatus {
                    branch: details.branch,
                    files: details.files.into_iter().map(ChangedFile::from).collect(),
                }));
            }
            CheckoutReply::Details(reply) => {
                let _ = reply.send(result);
            }
        }
    }
}

#[derive(Default)]
struct AgentExtra {
    modes: Vec<(String, String)>,
    current_mode: Option<String>,
    permissions: Vec<Value>,
    thinking: Option<String>,
    queue: Vec<QueuedPrompt>,
    updated_at: Option<String>,
    requires_attention: bool,
    attention_reason: Option<AttentionReason>,
    /// The daemon `attentionTimestamp`; a blank value is `None`.
    attention_timestamp: Option<String>,
    features: Vec<AgentFeature>,
    labels: BTreeMap<String, String>,
    /// The snapshot `title`, which `AgentSummary` replaces with an id prefix when null.
    title: Option<String>,
    created_at: Option<String>,
    active_turn: Option<ActiveTurn>,
    last_usage: Option<AgentUsage>,
    /// `runtimeInfo.sessionId`, else `persistence.sessionId`.
    session_id: Option<String>,
    queue_held: bool,
    /// `capabilities.supportsRewind*`.
    rewind: RewindModes,
    /// The snapshot `backgroundProcesses`, in the daemon's order.
    background_processes: Vec<BackgroundProcess>,
    /// The snapshot `skillHookUsage`. `None` when the snapshot has none.
    skill_hook_usage: Option<SkillHookUsage>,
}

/// The newest `todo` item of a viewed timeline, kept as pages and live rows arrive.
#[derive(Default)]
struct TasksCache {
    epoch: Option<String>,
    seq: Option<u64>,
    tasks: Vec<TaskEntry>,
}

impl TasksCache {
    /// Keeps the `todo` entries of `entries` that are newer than the kept one.
    fn note<'a>(&mut self, epoch: &str, entries: impl IntoIterator<Item = &'a Value>) {
        if self.epoch.as_deref() != Some(epoch) {
            return;
        }
        for entry in entries {
            if entry["item"]["type"] != "todo" {
                continue;
            }
            let Some(seq) = entry["seqStart"].as_u64() else {
                continue;
            };
            if self.seq.is_none_or(|kept| seq >= kept) {
                self.seq = Some(seq);
                self.tasks = composer_data::task_entries(&entry["item"]);
            }
        }
    }
}

/// A project as `project.list` or a workspace describes it.
#[derive(Clone)]
struct ProjectInfo {
    id: String,
    name: String,
    root: String,
    /// `projectKey`; a workspace descriptor has none.
    key: Option<String>,
}

/// The tree fields of a workspace that `WorkspaceSummary` does not hold.
struct WorkspaceMeta {
    project: ProjectInfo,
    branch: Option<String>,
    pinned_at: Option<String>,
    diff_stat: Option<DiffStat>,
    is_dirty: Option<bool>,
    pull_request: Option<WorkspacePullRequest>,
    /// `workspace_label_key` of each `labels` name, which `rebuild_projects` looks up in the
    /// label catalog.
    label_keys: Vec<String>,
    status_entered_at: Option<i64>,
}

/// The host label catalog (`workspace.label.list`), keyed by `workspace_label_key`, with the
/// cursor that orders its `workspace.label.update` pushes.
#[derive(Default)]
struct LabelCatalog {
    labels: HashMap<String, WorkspaceLabel>,
    cursor: Option<(String, u64)>,
}

/// The key the app compares label names by: whitespace collapsed, trimmed, lower case.
fn workspace_label_key(name: &str) -> String {
    name.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Thinking options of one model, as `(id, label)`, and its default.
struct ModelThinking {
    options: Vec<(String, String)>,
    default: Option<String>,
}

struct TerminalStream {
    sender: mpsc::UnboundedSender<TerminalFrame>,
    slot: Option<u8>,
}

/// Session events this client subscribes to; with `explicit_event_subscriptions` the daemon
/// sends them only on request.
const SESSION_EVENTS: [&str; 8] = [
    "agent_permission_request",
    "agent_permission_resolved",
    "project.update",
    "status.plugin_catalog_changed",
    "status.plugin_settings_changed",
    "agent.provider_subagents.update",
    "checkout_status_update",
    "script_status_update",
];

/// Client state for one host.
pub struct ClientCore {
    conn: Connection,
    generation: Option<u64>,
    ready: bool,
    server_info: Option<Value>,
    next_request: u64,
    pending: HashMap<String, Pending>,
    agents: Vec<AgentSummary>,
    extras: HashMap<String, AgentExtra>,
    /// Archived agents seen in `agent_update` or the history; never in `agents`.
    archived: HashMap<String, AgentSummary>,
    workspaces: Vec<WorkspaceSummary>,
    providers: Vec<ProviderSummary>,
    viewed: Vec<String>,
    /// Viewed provider subagent keys, which never reach an agent request.
    viewed_provider_subagents: Vec<String>,
    timelines: HashMap<String, TimelineSync>,
    timeline_subscription: Option<String>,
    latest_timeline_request: Option<String>,
    terminals: HashMap<String, TerminalStream>,
    slots: HashMap<u8, String>,
    bootstrapped: bool,
    events: Vec<ClientEvent>,
    directory_changed: bool,
    dirty_timelines: BTreeSet<String>,
    typed_server_info: Option<ServerInfo>,
    project_list: Vec<ProjectInfo>,
    /// `emptyProjects` of the workspace list, for a daemon without `project.list`.
    empty_projects: Vec<ProjectInfo>,
    workspace_meta: HashMap<String, WorkspaceMeta>,
    label_catalog: LabelCatalog,
    projects: Vec<ProjectNode>,
    projects_dirty: bool,
    directory_events: Vec<DirectoryEvent>,
    /// Keyed by `(provider, model id)`.
    model_thinking: HashMap<(String, String), ModelThinking>,
    default_models: HashMap<String, String>,
    /// Watched folders and their diff subscription, `None` until the daemon answers.
    checkout_watches: HashMap<String, Option<String>>,
    /// The revision of the last project config read or written, per repo root.
    config_revisions: HashMap<String, Value>,
    plugin_events: Vec<PluginEvent>,
    /// Per viewed agent.
    tasks: HashMap<String, TasksCache>,
    /// Provider-owned subagents by parent agent id.
    provider_subagents: HashMap<String, Vec<ProviderSubagent>>,
    git_status_events: Vec<GitStatusEvent>,
    /// By workspace id, from the workspace `scripts` and `script_status_update`.
    workspace_scripts: HashMap<String, Vec<WorkspaceScript>>,
    /// The images of each prompt this client sent, by message id. The daemon's `user_message`
    /// holds only text, so the bubble shows them from here.
    sent_images: HashMap<String, Vec<SentImage>>,
    /// Where sent images are saved, so they outlive the session. `None` keeps them in memory.
    sent_images_dir: Option<PathBuf>,
}

impl ClientCore {
    /// Starts the socket worker. `sink` receives worker events; feed them back to `handle`.
    pub fn new(
        config: ConnectionConfig,
        sink: impl FnMut(WorkerEvent) -> bool + Send + 'static,
    ) -> Self {
        Self {
            conn: Connection::spawn(config, sink),
            generation: None,
            ready: false,
            server_info: None,
            next_request: 0,
            pending: HashMap::new(),
            agents: Vec::new(),
            extras: HashMap::new(),
            archived: HashMap::new(),
            workspaces: Vec::new(),
            providers: Vec::new(),
            viewed: Vec::new(),
            viewed_provider_subagents: Vec::new(),
            timelines: HashMap::new(),
            timeline_subscription: None,
            latest_timeline_request: None,
            terminals: HashMap::new(),
            slots: HashMap::new(),
            bootstrapped: false,
            events: Vec::new(),
            directory_changed: false,
            dirty_timelines: BTreeSet::new(),
            typed_server_info: None,
            project_list: Vec::new(),
            empty_projects: Vec::new(),
            workspace_meta: HashMap::new(),
            label_catalog: LabelCatalog::default(),
            projects: Vec::new(),
            projects_dirty: false,
            directory_events: Vec::new(),
            model_thinking: HashMap::new(),
            default_models: HashMap::new(),
            checkout_watches: HashMap::new(),
            config_revisions: HashMap::new(),
            plugin_events: Vec::new(),
            tasks: HashMap::new(),
            provider_subagents: HashMap::new(),
            git_status_events: Vec::new(),
            workspace_scripts: HashMap::new(),
            sent_images: HashMap::new(),
            sent_images_dir: None,
        }
    }

    /// Saves sent images under `dir` from now on.
    pub fn set_sent_images_dir(&mut self, dir: PathBuf) {
        self.sent_images_dir = Some(dir);
    }

    /// Keeps the images of a sent prompt, and saves them when a folder is set.
    fn remember_sent_images(&mut self, message_id: &str, images: Vec<SentImage>) {
        if images.is_empty() {
            return;
        }
        if let Some(dir) = self.sent_images_dir.clone() {
            let (id, saved) = (message_id.to_string(), images.clone());
            std::thread::spawn(move || {
                if let Err(error) = sent_images::save(&dir, &id, &saved) {
                    log::warn!("cannot save the images of {id}: {error}");
                }
            });
        }
        self.sent_images.insert(message_id.to_string(), images);
    }

    /// The images sent with the prompt `message_id`, from this session or from disk.
    pub fn sent_images(&self, message_id: &str) -> Option<Vec<SentImage>> {
        if let Some(images) = self.sent_images.get(message_id) {
            return Some(images.clone());
        }
        sent_images::load(self.sent_images_dir.as_deref()?, message_id)
    }

    /// Git and script status events since the last call.
    pub fn take_git_status_events(&mut self) -> Vec<GitStatusEvent> {
        std::mem::take(&mut self.git_status_events)
    }

    /// Plugin events since the last call.
    pub fn take_plugin_events(&mut self) -> Vec<PluginEvent> {
        std::mem::take(&mut self.plugin_events)
    }

    /// The `server_info` of the current connection as the generated type. `None` before the
    /// first connection, or when a newer daemon sends a value this build cannot decode.
    pub fn typed_server_info(&self) -> Option<&ServerInfo> {
        self.typed_server_info.as_ref()
    }

    /// Whether the host holds prompts sent during a turn (`features.agentMessageQueue`).
    pub fn supports_queue(&self) -> bool {
        self.typed_server_info
            .as_ref()
            .and_then(|info| info.features.as_ref())
            .and_then(|features| features.agent_message_queue)
            == Some(true)
    }

    /// Projects with their workspaces, in the daemon's project order.
    pub fn projects(&self) -> &[ProjectNode] {
        &self.projects
    }

    /// Project tree and checkout events since the last call.
    pub fn take_directory_events(&mut self) -> Vec<DirectoryEvent> {
        std::mem::take(&mut self.directory_events)
    }

    pub fn queued_prompts(&self, agent_id: &str) -> Vec<QueuedPrompt> {
        self.extras
            .get(agent_id)
            .map(|extra| extra.queue.clone())
            .unwrap_or_default()
    }

    /// The agent's background processes, in the daemon's order.
    pub fn background_processes(&self, agent_id: &str) -> Vec<BackgroundProcess> {
        self.extras
            .get(agent_id)
            .map(|extra| extra.background_processes.clone())
            .unwrap_or_default()
    }

    /// The skills and hooks the agent used. `None` when its snapshot carries none.
    pub fn skill_hook_usage(&self, agent_id: &str) -> Option<SkillHookUsage> {
        self.extras
            .get(agent_id)
            .and_then(|extra| extra.skill_hook_usage.clone())
    }

    /// Whether the host can stop a background process (`features.agentBackgroundProcesses`).
    pub fn supports_background_stop(&self) -> bool {
        self.typed_server_info
            .as_ref()
            .and_then(|info| info.features.as_ref())
            .and_then(|features| features.agent_background_processes)
            == Some(true)
    }

    /// The thinking options of the agent's model. `None` when the model has none.
    pub fn agent_thinking(&self, agent_id: &str) -> Option<ThinkingControls> {
        let agent = self.agents.iter().find(|agent| agent.id == agent_id)?;
        let model = agent
            .model
            .clone()
            .or_else(|| self.default_models.get(&agent.provider).cloned())?;
        let thinking = self.model_thinking.get(&(agent.provider.clone(), model))?;
        if thinking.options.is_empty() {
            return None;
        }
        let current = self
            .extras
            .get(agent_id)
            .and_then(|extra| extra.thinking.clone())
            .or_else(|| thinking.default.clone());
        Some(ThinkingControls {
            options: thinking.options.clone(),
            current,
        })
    }

    pub fn is_connected(&self) -> bool {
        self.ready
    }

    /// The `server_info` status payload of the current connection.
    pub fn server_info(&self) -> Option<&Value> {
        self.server_info.as_ref()
    }

    pub fn agents(&self) -> &[AgentSummary] {
        &self.agents
    }

    pub fn workspaces(&self) -> &[WorkspaceSummary] {
        &self.workspaces
    }

    pub fn providers(&self) -> &[ProviderSummary] {
        &self.providers
    }

    pub fn timeline(&self, agent_id: &str) -> Option<&TimelineState> {
        self.timelines.get(agent_id).map(|sync| &sync.state)
    }

    /// The epoch of the agent's current timeline window, for building fork cursors.
    pub fn timeline_epoch(&self, agent_id: &str) -> Option<&str> {
        self.timelines.get(agent_id)?.epoch()
    }

    /// Events since the last call, with one `AgentsChanged` for any directory change.
    pub fn take_events(&mut self) -> Vec<ClientEvent> {
        if self.directory_changed || std::mem::take(&mut self.projects_dirty) {
            self.rebuild_projects();
        }
        if std::mem::take(&mut self.directory_changed) {
            self.events.push(ClientEvent::AgentsChanged);
        }
        std::mem::take(&mut self.events)
    }

    pub fn has_dirty_timelines(&self) -> bool {
        !self.dirty_timelines.is_empty()
    }

    /// Viewed agents whose timeline changed since the last call.
    pub fn take_dirty_timelines(&mut self) -> Vec<String> {
        std::mem::take(&mut self.dirty_timelines)
            .into_iter()
            .collect()
    }

    pub fn agent_controls(&self, agent_id: &str) -> Option<AgentControls> {
        let agent = self.agents.iter().find(|agent| agent.id == agent_id)?;
        let extra = self.extras.get(agent_id);
        let models = self
            .providers
            .iter()
            .find(|provider| provider.id == agent.provider)
            .map(|provider| provider.models.clone())
            .unwrap_or_default();
        Some(AgentControls {
            modes: extra.map(|extra| extra.modes.clone()).unwrap_or_default(),
            current_mode: extra.and_then(|extra| extra.current_mode.clone()),
            models,
            current_model: agent.model.clone(),
            running: agent.status == "running",
        })
    }

    /// Whether the agent asks for attention, and why. `None` for an unknown agent.
    pub fn agent_attention(&self, agent_id: &str) -> Option<(bool, Option<AttentionReason>)> {
        if !self.agents.iter().any(|agent| agent.id == agent_id) {
            return None;
        }
        let extra = self.extras.get(agent_id)?;
        Some((extra.requires_attention, extra.attention_reason))
    }

    /// The daemon `attentionTimestamp` of the agent. `None` when null, blank, absent, or the
    /// agent is unknown.
    pub fn agent_attention_timestamp(&self, agent_id: &str) -> Option<String> {
        if !self.agents.iter().any(|agent| agent.id == agent_id) {
            return None;
        }
        self.extras.get(agent_id)?.attention_timestamp.clone()
    }

    /// The agent's provider features. A feature type this client does not know is left out.
    pub fn agent_features(&self, agent_id: &str) -> Vec<AgentFeature> {
        self.extras
            .get(agent_id)
            .map(|extra| extra.features.clone())
            .unwrap_or_default()
    }

    /// The key the app groups a project by: its `projectKey`, else `["<serverId>","<projectId>"]`
    /// as `createProjectViewKey` builds it. `None` for an unknown project.
    pub fn project_view_key(&self, project_id: &str) -> Option<String> {
        let mut matches = self
            .project_list
            .iter()
            .chain(&self.empty_projects)
            .chain(self.workspace_meta.values().map(|meta| &meta.project))
            .filter(|project| project.id == project_id)
            .peekable();
        matches.peek()?;
        if let Some(key) = matches.find_map(|project| project.key.clone()) {
            return Some(key);
        }
        let server_id = self.server_info.as_ref()?["serverId"].as_str()?;
        Some(json!([server_id, project_id]).to_string())
    }

    pub fn workspace_diff_stat(&self, workspace_id: &str) -> Option<DiffStat> {
        self.workspace_meta.get(workspace_id)?.diff_stat
    }

    /// The workspace `gitRuntime.isDirty`, which also counts untracked files. `None` when the
    /// flag is absent or null, or the workspace is unknown.
    pub fn workspace_is_dirty(&self, workspace_id: &str) -> Option<bool> {
        self.workspace_meta.get(workspace_id)?.is_dirty
    }

    pub fn workspace_pinned_at(&self, workspace_id: &str) -> Option<String> {
        self.workspace_meta.get(workspace_id)?.pinned_at.clone()
    }

    // ---- worker events ----

    pub fn handle(&mut self, event: WorkerEvent) {
        match event {
            WorkerEvent::Opened { generation } => self.opened(generation),
            WorkerEvent::Ready {
                generation,
                server_info,
            } => {
                if self.generation == Some(generation) {
                    self.ready = true;
                    self.typed_server_info = bachuc_protocol::decode_server_info(&server_info)
                        .inspect_err(|error| log::warn!("server_info does not decode: {error}"))
                        .ok();
                    self.server_info = Some(server_info);
                    self.events.push(ClientEvent::Connected);
                    // The feature flag is known only now, so the list goes here, not in `opened`.
                    for agent_id in self.viewed.clone() {
                        self.request_provider_subagents(&agent_id);
                    }
                    // A reconnect starts every viewed child timeline again.
                    for key in self.viewed_provider_subagents.clone() {
                        self.request_provider_subagent_tail(&key);
                    }
                    self.request_label_list();
                }
            }
            WorkerEvent::Closed { generation, reason } => self.closed(generation, reason),
            WorkerEvent::Message(message) => self.message(message),
            WorkerEvent::Binary(bytes) => self.binary(&bytes),
        }
    }

    /// A new socket: send `hello`'s followers. The worker holds them until `server_info`.
    fn opened(&mut self, generation: u64) {
        self.generation = Some(generation);
        self.ready = false;
        self.slots.clear();
        self.timeline_subscription = None;
        self.latest_timeline_request = None;
        for stream in self.terminals.values_mut() {
            stream.slot = None;
        }
        // These cannot fail: the generation was just set.
        let _ = self.request(
            json!({
                "type": "fetch_agents_request",
                "sort": [{"key": "updated_at", "direction": "desc"}],
                "page": {"limit": 200},
                "subscribe": {},
            }),
            Pending::Agents { first: true },
        );
        let _ = self.request(
            json!({
                "type": "fetch_workspaces_request",
                "page": {"limit": 200},
                "subscribe": {},
            }),
            Pending::Workspaces { first: true },
        );
        let _ = self.request(json!({"type": "project.list.request"}), Pending::Projects);
        let _ = self.request(
            json!({"type": "list_available_providers_request"}),
            Pending::Providers,
        );
        let _ = self.request(
            json!({"type": "session.events.set_subscription.request", "events": SESSION_EVENTS}),
            Pending::Action("session.events.set_subscription"),
        );
        if !self.viewed.is_empty() {
            self.update_timeline_subscription();
        }
        for agent_id in self.viewed.clone() {
            if let Some(sync) = self.timelines.get_mut(&agent_id) {
                let plan = sync.resume();
                self.send_plan(&agent_id, plan);
            }
        }
        let terminal_ids: Vec<String> = self.terminals.keys().cloned().collect();
        for terminal_id in terminal_ids {
            self.subscribe_terminal(terminal_id, None);
        }
        let watched: Vec<String> = self.checkout_watches.keys().cloned().collect();
        for cwd in watched {
            self.subscribe_checkout(cwd);
        }
    }

    fn closed(&mut self, generation: u64, reason: String) {
        if self.generation.is_some_and(|current| current != generation) {
            return;
        }
        self.generation = None;
        self.ready = false;
        self.slots.clear();
        self.timeline_subscription = None;
        self.latest_timeline_request = None;
        for (_, pending) in self.pending.drain() {
            if let Pending::SubscribeTerminal {
                terminal_id,
                reply: Some(_),
            } = &pending
            {
                self.terminals.remove(terminal_id);
            }
            pending.fail(anyhow!("disconnected: {reason}"));
        }
        for subscription in self.checkout_watches.values_mut() {
            *subscription = None;
        }
        for sync in self.timelines.values_mut() {
            sync.disconnected();
        }
        self.events.push(ClientEvent::Disconnected { reason });
    }

    // ---- sending ----

    fn send(&mut self, message: Value) -> bool {
        let Some(generation) = self.generation else {
            return false;
        };
        match session_frame(message) {
            Ok(text) => {
                self.conn.send_text(generation, text);
                true
            }
            Err(error) => {
                log::error!("{error}");
                false
            }
        }
    }

    /// Sends a request and remembers what its response is for. Returns false when it could
    /// not be sent; the caller then fails its reply.
    fn request(&mut self, mut message: Value, pending: Pending) -> Result<(), Pending> {
        self.next_request += 1;
        let request_id = format!("bgc-{}", self.next_request);
        message["requestId"] = json!(request_id);
        if !self.send(message) {
            return Err(pending);
        }
        self.pending.insert(request_id, pending);
        Ok(())
    }

    fn send_plan(&mut self, agent_id: &str, plan: FetchPlan) {
        if parse_provider_subagent_key(agent_id).is_some() {
            self.send_provider_plan(agent_id, plan);
            return;
        }
        let Some(sync) = self.timelines.get(agent_id) else {
            return;
        };
        let generation = sync.generation();
        let message = plan.request(agent_id);
        let _ = self.request(
            message,
            Pending::Timeline {
                agent_id: agent_id.to_string(),
                plan,
                generation,
            },
        );
    }

    /// Sends a child timeline request for a provider subagent key.
    fn send_provider_plan(&mut self, key: &str, plan: FetchPlan) {
        if !self.supports_provider_subagent_timeline() {
            return;
        }
        let Some((parent_agent_id, subagent_id)) = parse_provider_subagent_key(key) else {
            return;
        };
        let Some(sync) = self.timelines.get(key) else {
            return;
        };
        let generation = sync.generation();
        let message = provider_subagent_timeline_request(&parent_agent_id, &subagent_id, &plan);
        let _ = self.request(
            message,
            Pending::ProviderTimeline {
                key: key.to_string(),
                plan,
                generation,
            },
        );
    }

    fn request_provider_subagent_tail(&mut self, key: &str) {
        let Some(sync) = self.timelines.get_mut(key) else {
            return;
        };
        let plan = sync.plan_tail();
        self.send_provider_plan(key, plan);
    }

    fn update_timeline_subscription(&mut self) {
        if self.viewed.is_empty() {
            self.latest_timeline_request = None;
            if let Some(old) = self.timeline_subscription.take() {
                self.release(&old);
            }
            return;
        }
        let agent_ids = self.viewed.clone();
        let request_id = format!("bgc-{}", self.next_request + 1);
        if self
            .request(
                json!({"type": "agent.timeline.set_subscription.request", "agentIds": agent_ids}),
                Pending::TimelineSubscription,
            )
            .is_ok()
        {
            self.latest_timeline_request = Some(request_id);
        }
    }

    fn release(&mut self, subscription_id: &str) {
        let _ = self.request(
            json!({"type": "subscription.release.request", "subscriptionId": subscription_id}),
            Pending::Action("subscription.release"),
        );
    }

    // ---- public requests ----

    pub fn view_agent(&mut self, agent_id: &str) {
        if parse_provider_subagent_key(agent_id).is_some() {
            self.view_provider_subagent(agent_id);
            return;
        }
        if self.viewed.iter().any(|id| id == agent_id) {
            return;
        }
        self.viewed.push(agent_id.to_string());
        let mut sync = TimelineSync::new();
        if let Some(extra) = self.extras.get(agent_id) {
            sync.state.pending_permissions = extra.permissions.clone();
        }
        let plan = sync.plan_tail();
        self.timelines.insert(agent_id.to_string(), sync);
        self.tasks
            .insert(agent_id.to_string(), TasksCache::default());
        if self.generation.is_some() {
            // Subscribe first so rows after the tail page are buffered, not missed.
            self.update_timeline_subscription();
            self.send_plan(agent_id, plan);
        }
        if self.ready {
            self.request_provider_subagents(agent_id);
        }
    }

    fn supports_provider_subagents(&self) -> bool {
        self.typed_server_info
            .as_ref()
            .and_then(|info| info.features.as_ref())
            .and_then(|features| features.provider_subagents)
            == Some(true)
    }

    /// Whether the host streams provider subagent timelines
    /// (`features.projectedSubagentTimeline`).
    pub fn supports_provider_subagent_timeline(&self) -> bool {
        self.typed_server_info
            .as_ref()
            .and_then(|info| info.features.as_ref())
            .and_then(|features| features.projected_subagent_timeline)
            == Some(true)
    }

    /// Views a child timeline under its provider subagent key. A second call for the same key
    /// sends nothing; a reconnect refetches the tail.
    fn view_provider_subagent(&mut self, key: &str) {
        if self.viewed_provider_subagents.iter().any(|id| id == key) {
            return;
        }
        self.viewed_provider_subagents.push(key.to_string());
        self.timelines.insert(key.to_string(), TimelineSync::new());
        if self.ready {
            self.request_provider_subagent_tail(key);
        }
    }

    /// Lists the provider subagents of a viewed agent when the host has them.
    fn request_provider_subagents(&mut self, agent_id: &str) {
        if !self.supports_provider_subagents() {
            return;
        }
        let _ = self.request(
            json!({"type": "agent.provider_subagents.list.request", "parentAgentId": agent_id}),
            Pending::ProviderSubagents(agent_id.to_string()),
        );
    }

    /// The provider of a listed subagent, for a child page whose reply has no `provider`.
    fn provider_subagent_provider(&self, key: &str) -> Option<String> {
        let (parent, child) = parse_provider_subagent_key(key)?;
        self.provider_subagents
            .get(&parent)?
            .iter()
            .find(|subagent| subagent.row.id == child)
            .map(|subagent| subagent.row.provider.clone())
            .filter(|provider| !provider.is_empty())
    }

    pub fn unview_agent(&mut self, agent_id: &str) {
        if parse_provider_subagent_key(agent_id).is_some() {
            self.viewed_provider_subagents.retain(|id| id != agent_id);
            self.timelines.remove(agent_id);
            self.dirty_timelines.remove(agent_id);
            return;
        }
        let Some(index) = self.viewed.iter().position(|id| id == agent_id) else {
            return;
        };
        self.viewed.remove(index);
        self.timelines.remove(agent_id);
        self.tasks.remove(agent_id);
        self.dirty_timelines.remove(agent_id);
        if self.generation.is_some() {
            self.update_timeline_subscription();
        }
    }

    pub fn load_older(&mut self, agent_id: &str) {
        let Some(plan) = self
            .timelines
            .get_mut(agent_id)
            .and_then(TimelineSync::load_older)
        else {
            return;
        };
        self.send_plan(agent_id, plan);
    }

    pub fn send_prompt(&mut self, agent_id: &str, text: String, images: Vec<Value>) {
        self.send_message(agent_id, text, images, None);
    }

    /// Sends a prompt with what it does to a running turn.
    pub fn send_prompt_with(
        &mut self,
        agent_id: &str,
        text: String,
        images: Vec<Value>,
        behavior: TurnBehavior,
    ) {
        self.send_message(agent_id, text, images, Some(behavior));
    }

    fn send_message(
        &mut self,
        agent_id: &str,
        text: String,
        images: Vec<Value>,
        behavior: Option<TurnBehavior>,
    ) {
        let mut message = json!({
            "type": "send_agent_message_request",
            "agentId": agent_id,
            "text": text,
            "messageId": uuid::Uuid::new_v4().to_string(),
        });
        if !images.is_empty() {
            message["images"] = Value::Array(images);
        }
        if let Some(behavior) = behavior {
            message["activeTurnBehavior"] = json!(behavior.wire());
        }
        let _ = self.request(message, Pending::Action("send_agent_message"));
    }

    /// Queues a prompt behind the running turn. Needs `supports_queue`.
    pub fn enqueue_prompt(&mut self, agent_id: &str, text: String, images: Vec<Value>) {
        let mut message =
            json!({"type": "agent.queue.enqueue.request", "agentId": agent_id, "text": text});
        if !images.is_empty() {
            message["images"] = Value::Array(images);
        }
        self.queue_request(message, agent_id, None);
    }

    pub fn cancel_queued(&mut self, agent_id: &str, queued_id: &str) {
        self.queue_request(
            json!({"type": "agent.queue.cancel.request", "agentId": agent_id, "itemId": queued_id}),
            agent_id,
            None,
        );
    }

    fn queue_request(&mut self, message: Value, agent_id: &str, reply: Option<Reply<()>>) {
        let pending = Pending::Queue {
            agent_id: agent_id.to_string(),
            reply,
            images: Vec::new(),
        };
        if let Err(pending) = self.request(message, pending) {
            pending.fail(anyhow!("not connected"));
        }
    }

    // ---- composer, New workspace, and tab menu requests (TASK-169) ----

    /// Adds the images and attachments of `message` to a send or enqueue request.
    fn with_outgoing(mut request: Value, message: OutgoingMessage) -> Value {
        if !message.images.is_empty() {
            request["images"] = Value::Array(message.images);
        }
        if !message.attachments.is_empty() {
            request["attachments"] = Value::Array(message.attachments);
        }
        request
    }

    /// Sends a prompt with images and attachments; resolves on the daemon's answer.
    pub fn send_outgoing(
        &mut self,
        agent_id: &str,
        message: OutgoingMessage,
        behavior: TurnBehavior,
    ) -> oneshot::Receiver<anyhow::Result<()>> {
        let message_id = uuid::Uuid::new_v4().to_string();
        self.remember_sent_images(&message_id, sent_images::decode(&message.images));
        let request = json!({
            "type": "send_agent_message_request",
            "agentId": agent_id,
            "text": message.text,
            "messageId": message_id,
            "activeTurnBehavior": behavior.wire(),
        });
        self.request_reply(Self::with_outgoing(request, message), Pending::Accepted)
    }

    /// Queues a prompt with images and attachments; resolves when the daemon holds it.
    pub fn enqueue_outgoing(
        &mut self,
        agent_id: &str,
        message: OutgoingMessage,
    ) -> oneshot::Receiver<anyhow::Result<()>> {
        let request = json!({"type": "agent.queue.enqueue.request", "agentId": agent_id, "text": message.text});
        let (reply, receiver) = oneshot::channel();
        // The daemon sends the item with its id as the message id, so the bubble finds them.
        let pending = Pending::Queue {
            agent_id: agent_id.to_string(),
            reply: Some(reply),
            images: sent_images::decode(&message.images),
        };
        if let Err(pending) = self.request(Self::with_outgoing(request, message), pending) {
            pending.fail(anyhow!("not connected"));
        }
        receiver
    }

    /// Whether Stop holds the agent's queue until a resume or a promote.
    pub fn queue_held(&self, agent_id: &str) -> bool {
        self.extras
            .get(agent_id)
            .is_some_and(|extra| extra.queue_held)
    }

    pub fn edit_queued(&mut self, agent_id: &str, queued_id: &str, text: &str) {
        self.queue_request(
            json!({"type": "agent.queue.edit.request", "agentId": agent_id, "itemId": queued_id,
                   "text": text}),
            agent_id,
            None,
        );
    }

    /// Moves a queued prompt before `before_id`; `None` moves it to the end.
    pub fn reorder_queued(&mut self, agent_id: &str, queued_id: &str, before_id: Option<&str>) {
        self.queue_request(
            json!({"type": "agent.queue.reorder.request", "agentId": agent_id, "itemId": queued_id,
                   "beforeItemId": before_id}),
            agent_id,
            None,
        );
    }

    /// Sends a queued prompt now.
    pub fn promote_queued(&mut self, agent_id: &str, queued_id: &str) {
        self.queue_request(
            json!({"type": "agent.queue.promote.request", "agentId": agent_id, "itemId": queued_id}),
            agent_id,
            None,
        );
    }

    /// Releases a held queue.
    pub fn resume_queue(&mut self, agent_id: &str) {
        self.queue_request(
            json!({"type": "agent.queue.resume.request", "agentId": agent_id}),
            agent_id,
            None,
        );
    }

    pub fn agent_active_turn(&self, agent_id: &str) -> Option<ActiveTurn> {
        self.extras.get(agent_id)?.active_turn.clone()
    }

    pub fn agent_last_usage(&self, agent_id: &str) -> Option<AgentUsage> {
        self.extras.get(agent_id)?.last_usage
    }

    /// The metrics of the viewed agent's last turn. It scans back from the end and stops at
    /// the last user message, as the app's `findLastTurnMetrics` does.
    pub fn agent_last_turn_metrics(&self, agent_id: &str) -> Option<TurnMetrics> {
        let entries = &self.timelines.get(agent_id)?.state.entries;
        for entry in entries.iter().rev() {
            match entry["item"]["type"].as_str() {
                Some("user_message") => return None,
                Some("assistant_message") => {
                    if let Some(metrics) = composer_data::turn_metrics(&entry["turnMetrics"]) {
                        return Some(metrics);
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// The newest `todo` list of a viewed agent; empty for an agent that is not viewed.
    pub fn agent_tasks(&self, agent_id: &str) -> Vec<TaskEntry> {
        self.tasks
            .get(agent_id)
            .map(|cache| cache.tasks.clone())
            .unwrap_or_default()
    }

    /// The trimmed `paseo.parent-agent-id` label; `None` when it is absent or blank.
    pub fn agent_parent_id(&self, agent_id: &str) -> Option<String> {
        let parent = self.extras.get(agent_id)?.labels.get(PARENT_AGENT_LABEL)?;
        let parent = parent.trim();
        (!parent.is_empty()).then(|| parent.to_string())
    }

    /// The track strip rows of a parent agent: Paseo children, then provider children, merged
    /// by `createdAt` as the app's `useSubagentsForParent` does.
    pub fn subagents(&self, parent_agent_id: &str) -> Vec<SubagentRow> {
        let mut rows: Vec<SubagentRow> = self
            .agents
            .iter()
            .filter(|agent| self.agent_parent_id(&agent.id).as_deref() == Some(parent_agent_id))
            .map(|agent| {
                let extra = self.extras.get(&agent.id);
                SubagentRow {
                    id: agent.id.clone(),
                    kind: SubagentKind::Paseo,
                    provider: agent.provider.clone(),
                    title: extra.and_then(|extra| extra.title.clone()),
                    description: None,
                    subtitle: None,
                    status: agent.status.clone(),
                    requires_attention: extra.is_some_and(|extra| extra.requires_attention),
                    created_at: extra
                        .and_then(|extra| extra.created_at.clone())
                        .unwrap_or_default(),
                }
            })
            .collect();
        rows.sort_by(|left, right| left.created_at.cmp(&right.created_at));
        if !self.supports_provider_subagents() {
            return rows;
        }
        let nesting = self
            .typed_server_info
            .as_ref()
            .and_then(|info| info.features.as_ref())
            .and_then(|features| features.provider_subagent_nesting)
            == Some(true);
        let mut provider: Vec<SubagentRow> = self
            .provider_subagents
            .get(parent_agent_id)
            .into_iter()
            .flatten()
            .filter(|subagent| !(nesting && subagent.nested))
            .map(|subagent| subagent.row.clone())
            .collect();
        if provider.is_empty() {
            return rows;
        }
        rows.append(&mut provider);
        rows.sort_by(|left, right| left.created_at.cmp(&right.created_at));
        rows
    }

    /// The modes of a provider as `(id, label)`.
    pub fn provider_modes(
        &mut self,
        provider: &str,
        cwd: Option<&str>,
    ) -> oneshot::Receiver<anyhow::Result<Vec<(String, String)>>> {
        let mut message = json!({"type": "list_provider_modes_request", "provider": provider});
        if let Some(cwd) = cwd {
            message["cwd"] = json!(cwd);
        }
        self.request_reply(message, Pending::Modes)
    }

    /// The provider features of a draft; a feature type this client does not know is left out.
    pub fn draft_features(
        &mut self,
        config: DraftConfig,
    ) -> oneshot::Receiver<anyhow::Result<Vec<AgentFeature>>> {
        self.request_reply(
            json!({"type": "list_provider_features_request",
                   "draftConfig": composer_data::draft_config(&config)}),
            Pending::DraftFeatures,
        )
    }

    /// The provider's default model.
    pub fn default_model(&self, provider: &str) -> Option<String> {
        self.default_models.get(provider).cloned()
    }

    /// The thinking options of a model; `current` is the model default. `None` when the model
    /// has none.
    pub fn model_thinking(&self, provider: &str, model: &str) -> Option<ThinkingControls> {
        let thinking = self
            .model_thinking
            .get(&(provider.to_string(), model.to_string()))?;
        if thinking.options.is_empty() {
            return None;
        }
        Some(ThinkingControls {
            options: thinking.options.clone(),
            current: thinking.default.clone(),
        })
    }

    /// Creates a workspace, with its first agent when `request.agent` is set.
    pub fn create_workspace(
        &mut self,
        request: CreateWorkspace,
    ) -> oneshot::Receiver<anyhow::Result<CreatedWorkspace>> {
        let features = self
            .typed_server_info
            .as_ref()
            .and_then(|info| info.features.as_ref());
        let receipts =
            features.and_then(|features| features.workspace_request_receipts) == Some(true);
        let lifecycle = features.and_then(|features| features.creation_lifecycle) == Some(true);
        let key = receipts.then(|| uuid::Uuid::new_v4().to_string());
        // Updates name the request by its key; without one the reply waits for the response.
        let subscribe = lifecycle && key.is_some();
        let message = composer_data::create_workspace(&request, key.as_deref(), subscribe);
        self.request_reply(message, |reply| Pending::CreateWorkspace {
            key: key.filter(|_| subscribe),
            reply,
        })
    }

    /// Creates an agent with its full config and resolves to its id.
    pub fn create_agent_with(
        &mut self,
        agent: InitialAgent,
    ) -> oneshot::Receiver<anyhow::Result<String>> {
        let mut fields = composer_data::initial_agent(&agent);
        // As `create_agent`: no prompt, no turn.
        if agent.initial_prompt.is_empty() {
            fields.remove("initialPrompt");
            fields.remove("clientMessageId");
        }
        let mut message = Value::Object(fields);
        message["type"] = json!("create_agent_request");
        self.request_reply(message, Pending::CreateAgent)
    }

    pub fn branch_suggestions(
        &mut self,
        cwd: &str,
        query: Option<&str>,
        limit: Option<u32>,
    ) -> oneshot::Receiver<anyhow::Result<Vec<BranchSuggestion>>> {
        let mut message = json!({"type": "branch_suggestions_request", "cwd": cwd});
        if let Some(query) = query {
            message["query"] = json!(query);
        }
        if let Some(limit) = limit {
            message["limit"] = json!(limit);
        }
        self.request_reply(message, Pending::BranchSuggestions)
    }

    pub fn validate_branch(
        &mut self,
        cwd: &str,
        branch_name: &str,
    ) -> oneshot::Receiver<anyhow::Result<BranchValidation>> {
        self.request_reply(
            json!({"type": "validate_branch_request", "cwd": cwd, "branchName": branch_name}),
            Pending::ValidateBranch,
        )
    }

    /// Registers the folder as a project.
    pub fn add_project(&mut self, cwd: &str) -> oneshot::Receiver<anyhow::Result<AddedProject>> {
        self.request_reply(
            json!({"type": "project.add.request", "cwd": cwd}),
            Pending::AddProject,
        )
    }

    /// Reloads the agent's timeline from its provider.
    pub fn refresh_agent(&mut self, agent_id: &str) -> oneshot::Receiver<anyhow::Result<()>> {
        self.request_reply(
            json!({"type": "refresh_agent_request", "agentId": agent_id}),
            Pending::Refresh,
        )
    }

    /// The daemon unarchives the agent and restores its workspace on a refresh.
    pub fn unarchive_agent(&mut self, agent_id: &str) -> oneshot::Receiver<anyhow::Result<()>> {
        self.refresh_agent(agent_id)
    }

    /// Fetches one agent by id. An archived agent fills the archived map: the agent subscription
    /// has no `includeArchived`, so the daemon sends `remove` when an agent is archived.
    pub fn fetch_agent(&mut self, agent_id: &str) -> oneshot::Receiver<anyhow::Result<()>> {
        self.request_reply(
            json!({"type": "fetch_agent_request", "agentId": agent_id}),
            Pending::FetchAgent,
        )
    }

    pub fn archived_agent(&self, agent_id: &str) -> Option<&AgentSummary> {
        self.archived.get(agent_id)
    }

    pub fn is_archived(&self, agent_id: &str) -> bool {
        self.archived.contains_key(agent_id)
    }

    fn server_feature(&self, name: &str) -> bool {
        self.server_info
            .as_ref()
            .is_some_and(|info| info["features"][name] == true)
    }

    /// Whether the host searches the history (`features.agentHistorySearch`).
    pub fn history_search_supported(&self) -> bool {
        self.server_feature("agentHistorySearch")
    }

    /// One history page, newest first. The search is left out when empty or unsupported.
    pub fn fetch_agent_history(
        &mut self,
        search: Option<&str>,
        cursor: Option<&str>,
    ) -> oneshot::Receiver<anyhow::Result<HistoryPage>> {
        let mut page = json!({"limit": 200});
        if let Some(cursor) = cursor {
            page["cursor"] = json!(cursor);
        }
        let mut message = json!({"type": "fetch_agent_history_request",
                                 "sort": [{"key": "updated_at", "direction": "desc"}],
                                 "page": page});
        if let Some(search) = search.filter(|search| !search.is_empty())
            && self.history_search_supported()
        {
            message["search"] = json!(search);
        }
        self.request_reply(message, Pending::History)
    }

    /// Whether the host lists the prompts of a timeline (`features.agentTimelinePromptIndex`).
    pub fn prompt_index_supported(&self) -> bool {
        self.server_feature("agentTimelinePromptIndex")
    }

    /// Every user prompt of the agent's timeline, including the ones outside the loaded window.
    pub fn list_prompts(
        &mut self,
        agent_id: &str,
    ) -> oneshot::Receiver<anyhow::Result<PromptIndex>> {
        if !self.prompt_index_supported() {
            return failed("Update the host to use this.");
        }
        self.request_reply(
            json!({"type": "agent.timeline.list_prompts.request", "agentId": agent_id}),
            Pending::Prompts,
        )
    }

    /// The rewind modes of the agent: none for a host without `features.rewind`.
    pub fn rewind_modes(&self, agent_id: &str) -> RewindModes {
        if !self.server_feature("rewind") {
            return RewindModes::default();
        }
        self.extras
            .get(agent_id)
            .map(|extra| extra.rewind)
            .unwrap_or_default()
    }

    /// Rewinds the agent to the user message `message_id`.
    pub fn rewind(
        &mut self,
        agent_id: &str,
        message_id: &str,
        mode: RewindMode,
    ) -> oneshot::Receiver<anyhow::Result<()>> {
        if !self.server_feature("rewind") {
            return failed("Update the host to use this.");
        }
        let message = json!({"type": "agent.rewind.request", "agentId": agent_id,
                             "messageId": message_id, "mode": mode.wire()});
        let agent_id = agent_id.to_string();
        self.request_reply(message, |reply| Pending::Rewind {
            agent_id,
            mode,
            reply,
        })
    }

    /// Whether the host builds fork contexts (`features.agentForkContext`).
    pub fn fork_supported(&self) -> bool {
        self.server_feature("agentForkContext")
    }

    /// Whether the host takes a fork boundary cursor (`features.agentForkContextCursor`).
    pub fn fork_cursor_supported(&self) -> bool {
        self.server_feature("agentForkContextCursor")
    }

    /// The chat history up to `boundary`, as an attachment for a new agent.
    pub fn fork_context(
        &mut self,
        agent_id: &str,
        boundary: ForkBoundary,
    ) -> oneshot::Receiver<anyhow::Result<ForkContext>> {
        if !self.fork_supported() {
            return failed("Update the host to use this.");
        }
        let mut message = json!({"type": "agent.fork_context.request", "agentId": agent_id});
        if let Some(cursor) = boundary.cursor
            && self.fork_cursor_supported()
        {
            message["boundaryCursor"] = json!({"epoch": cursor.epoch, "seq": cursor.seq});
        }
        if let Some(message_id) = boundary.message_id {
            message["boundaryMessageId"] = json!(message_id);
        }
        self.request_reply(message, Pending::ForkContext)
    }

    /// Files and folders under `cwd` that match `query`, at most `limit` (1 to 100).
    pub fn directory_suggestions(
        &mut self,
        cwd: &str,
        query: &str,
        include_files: bool,
        include_directories: bool,
        limit: u32,
    ) -> oneshot::Receiver<anyhow::Result<Vec<DirectorySuggestion>>> {
        self.request_reply(
            directory_suggestions_message(cwd, query, include_files, include_directories, limit),
            Pending::DirectorySuggestions,
        )
    }

    /// `directory_suggestions` with an explicit `matchMode`.
    pub fn directory_suggestions_matching(
        &mut self,
        cwd: &str,
        query: &str,
        include_files: bool,
        include_directories: bool,
        limit: u32,
        mode: MatchMode,
    ) -> oneshot::Receiver<anyhow::Result<Vec<DirectorySuggestion>>> {
        let mut message =
            directory_suggestions_message(cwd, query, include_files, include_directories, limit);
        message["matchMode"] = json!(mode.as_str());
        self.request_reply(message, Pending::DirectorySuggestions)
    }

    /// Whether the host discards checkout changes (`features.checkoutDiscardChanges`).
    pub fn discard_changes_supported(&self) -> bool {
        self.server_feature("checkoutDiscardChanges")
    }

    /// Whether the host creates, renames, and deletes entries (`features.fsEntryOps`).
    pub fn fs_entry_ops_supported(&self) -> bool {
        self.server_feature("fsEntryOps")
    }

    /// Whether the host duplicates entries (`features.fsEntryDuplicate`).
    pub fn fs_entry_duplicate_supported(&self) -> bool {
        self.server_feature("fsEntryDuplicate")
    }

    /// Sends an `fs.entry.*` request, or fails without one when the host lacks the feature.
    fn fs_entry(
        &mut self,
        supported: bool,
        message: Value,
    ) -> oneshot::Receiver<anyhow::Result<()>> {
        if !supported {
            return failed("Update the host to use this.");
        }
        self.request_reply(message, Pending::FsEntry)
    }

    pub fn create_entry(
        &mut self,
        cwd: &str,
        parent_path: &str,
        name: &str,
        kind: NewEntryKind,
    ) -> oneshot::Receiver<anyhow::Result<()>> {
        let supported = self.fs_entry_ops_supported();
        self.fs_entry(
            supported,
            json!({"type": "fs.entry.create.request", "cwd": cwd, "parentPath": parent_path,
                   "name": name, "kind": kind.wire()}),
        )
    }

    pub fn rename_entry(
        &mut self,
        cwd: &str,
        path: &str,
        name: &str,
    ) -> oneshot::Receiver<anyhow::Result<()>> {
        let supported = self.fs_entry_ops_supported();
        self.fs_entry(
            supported,
            json!({"type": "fs.entry.rename.request", "cwd": cwd, "path": path, "name": name}),
        )
    }

    pub fn duplicate_entry(
        &mut self,
        cwd: &str,
        path: &str,
    ) -> oneshot::Receiver<anyhow::Result<()>> {
        let supported = self.fs_entry_duplicate_supported();
        self.fs_entry(
            supported,
            json!({"type": "fs.entry.duplicate.request", "cwd": cwd, "path": path}),
        )
    }

    pub fn delete_entry(&mut self, cwd: &str, path: &str) -> oneshot::Receiver<anyhow::Result<()>> {
        let supported = self.fs_entry_ops_supported();
        self.fs_entry(
            supported,
            json!({"type": "fs.entry.delete.request", "cwd": cwd, "path": path}),
        )
    }

    /// Discards the changes of `paths`; the protocol needs at least one path.
    pub fn discard_changes(
        &mut self,
        cwd: &str,
        paths: Vec<String>,
    ) -> oneshot::Receiver<anyhow::Result<()>> {
        if paths.is_empty() {
            return failed("No files to discard");
        }
        self.request_reply(
            json!({"type": "checkout.discard_changes.request", "cwd": cwd, "paths": paths}),
            Pending::CheckoutDone,
        )
    }

    /// The provider-native session id: `runtimeInfo.sessionId`, else `persistence.sessionId`.
    pub fn agent_provider_session_id(&self, agent_id: &str) -> Option<String> {
        self.extras.get(agent_id)?.session_id.clone()
    }

    /// The snapshot `updatedAt`; `None` for an unknown agent.
    pub fn agent_updated_at(&self, agent_id: &str) -> Option<String> {
        if !self.agents.iter().any(|agent| agent.id == agent_id) {
            return None;
        }
        self.extras.get(agent_id)?.updated_at.clone()
    }

    /// The stored `thinkingOptionId`; unlike `agent_thinking` it needs no loaded model options.
    pub fn agent_thinking_option_id(&self, agent_id: &str) -> Option<String> {
        if !self.agents.iter().any(|agent| agent.id == agent_id) {
            return None;
        }
        self.extras.get(agent_id)?.thinking.clone()
    }

    pub fn rename_terminal(
        &mut self,
        terminal_id: &str,
        title: &str,
    ) -> oneshot::Receiver<anyhow::Result<()>> {
        self.request_reply(
            json!({"type": "terminal.rename.request", "terminalId": terminal_id, "title": title}),
            Pending::RenameTerminal,
        )
    }

    /// Every changed file of `cwd` with its diff, against `compare`.
    pub fn checkout_diff_with(
        &mut self,
        cwd: &str,
        compare: DiffCompare,
    ) -> oneshot::Receiver<anyhow::Result<Vec<FileDiff>>> {
        self.request_reply(
            json!({"type": "checkout.diff.get.request", "cwd": cwd,
                   "compare": composer_data::diff_compare(&compare)}),
            Pending::CheckoutDiff,
        )
    }

    pub fn set_thinking(&mut self, agent_id: &str, option_id: &str) {
        let _ = self.request(
            json!({
                "type": "set_agent_thinking_request",
                "agentId": agent_id,
                "thinkingOptionId": option_id,
            }),
            Pending::Action("set_agent_thinking"),
        );
    }

    pub fn archive_agent(&mut self, agent_id: &str) {
        let _ = self.request(
            json!({"type": "archive_agent_request", "agentId": agent_id}),
            Pending::Action("archive_agent"),
        );
    }

    pub fn rename_agent(&mut self, agent_id: &str, title: &str) {
        let _ = self.request(
            json!({"type": "update_agent_request", "agentId": agent_id, "name": title}),
            Pending::Action("update_agent"),
        );
    }

    pub fn rename_workspace(&mut self, workspace_id: &str, title: &str) {
        let _ = self.request(
            json!({"type": "workspace.title.set.request", "workspaceId": workspace_id, "title": title}),
            Pending::Action("workspace.title.set"),
        );
    }

    pub fn pin_workspace(&mut self, workspace_id: &str, pinned: bool) {
        let _ = self.request(
            json!({"type": "workspace.pin.set.request", "workspaceId": workspace_id, "pinned": pinned}),
            Pending::Action("workspace.pin.set"),
        );
    }

    pub fn archive_workspace(&mut self, workspace_id: &str) {
        let _ = self.request(
            json!({"type": "archive_workspace_request", "workspaceId": workspace_id}),
            Pending::Action("archive_workspace"),
        );
    }

    /// Asks the daemon to remove a project and its workspaces from the sidebar; the files on disk
    /// stay. The project leaves the tree when the answer accepts.
    pub fn remove_project(&mut self, project_id: &str) {
        let pending = Pending::ProjectRemove(project_id.to_string());
        if self
            .request(
                json!({"type": "project.remove.request", "projectId": project_id}),
                pending,
            )
            .is_err()
        {
            self.events.push(ClientEvent::ProjectRemoveFailed {
                project_id: project_id.to_string(),
            });
        }
    }

    pub fn mark_workspace_unread(&mut self, workspace_id: &str) {
        let _ = self.request(
            json!({"type": "workspace.mark_unread.request", "workspaceId": workspace_id}),
            Pending::Action("workspace.mark_unread"),
        );
    }

    pub fn clear_agent_attention(&mut self, agent_id: &str) {
        let _ = self.request(
            json!({"type": "clear_agent_attention", "agentId": agent_id}),
            Pending::Action("clear_agent_attention"),
        );
    }

    /// Answers a permission request with a full `AgentPermissionResponse`: question answers,
    /// plan approval, or `updatedInput`.
    pub fn respond_permission_with(&mut self, agent_id: &str, request_id: &str, response: Value) {
        self.send(json!({
            "type": "agent_permission_response",
            "agentId": agent_id,
            "requestId": request_id,
            "response": response,
        }));
    }

    /// Sends a request whose answer resolves the returned receiver.
    fn request_reply<T>(
        &mut self,
        message: Value,
        pending: impl FnOnce(Reply<T>) -> Pending,
    ) -> oneshot::Receiver<anyhow::Result<T>> {
        let (reply, receiver) = oneshot::channel();
        if let Err(pending) = self.request(message, pending(reply)) {
            pending.fail(anyhow!("not connected"));
        }
        receiver
    }

    pub fn list_commands(
        &mut self,
        agent_id: &str,
    ) -> oneshot::Receiver<anyhow::Result<Vec<SlashCommand>>> {
        self.request_reply(
            json!({"type": "list_commands_request", "agentId": agent_id}),
            Pending::Commands,
        )
    }

    /// The commands of a draft that has no agent yet, with the project skills of its folder.
    pub fn list_draft_commands(
        &mut self,
        agent_id: &str,
        config: &DraftConfig,
    ) -> oneshot::Receiver<anyhow::Result<Vec<SlashCommand>>> {
        self.request_reply(
            json!({
                "type": "list_commands_request",
                "agentId": agent_id,
                "draftConfig": composer_data::draft_config(config),
            }),
            Pending::Commands,
        )
    }

    pub fn list_directory(
        &mut self,
        cwd: &str,
        path: &str,
    ) -> oneshot::Receiver<anyhow::Result<Vec<FileEntry>>> {
        self.request_reply(
            json!({"type": "file_explorer_request", "cwd": cwd, "path": path, "mode": "list"}),
            Pending::FileList,
        )
    }

    /// Reads a file. The daemon streams it as binary frames; an older one answers in JSON.
    pub fn read_file(
        &mut self,
        cwd: &str,
        path: &str,
    ) -> oneshot::Receiver<anyhow::Result<FileContent>> {
        self.request_reply(
            json!({
                "type": "file_explorer_request",
                "cwd": cwd,
                "path": path,
                "mode": "file",
                "acceptBinary": true,
            }),
            |reply| Pending::ReadFile {
                path: path.to_string(),
                reply,
                transfer: None,
            },
        )
    }

    /// The branch and the uncommitted changed files of a folder.
    pub fn checkout_status(
        &mut self,
        cwd: &str,
    ) -> oneshot::Receiver<anyhow::Result<CheckoutStatus>> {
        self.request_reply(
            json!({"type": "checkout_status_request", "cwd": cwd}),
            |reply| Pending::CheckoutStatus(CheckoutReply::Status(reply)),
        )
    }

    /// `checkout_status` with the git flag and the full file fields.
    pub fn checkout_details(
        &mut self,
        cwd: &str,
    ) -> oneshot::Receiver<anyhow::Result<CheckoutDetails>> {
        self.request_reply(
            json!({"type": "checkout_status_request", "cwd": cwd}),
            |reply| Pending::CheckoutStatus(CheckoutReply::Details(reply)),
        )
    }

    /// Every uncommitted file of `cwd` with its diff, from one request. A folder outside git
    /// gives no files.
    pub fn checkout_diff(&mut self, cwd: &str) -> oneshot::Receiver<anyhow::Result<Vec<FileDiff>>> {
        self.request_reply(
            json!({"type": "checkout.diff.get.request", "cwd": cwd, "compare": {"mode": UNCOMMITTED}}),
            Pending::CheckoutDiff,
        )
    }

    pub fn commits(&mut self, cwd: &str) -> oneshot::Receiver<anyhow::Result<CommitList>> {
        self.request_reply(
            json!({"type": "checkout.commits.list.request", "cwd": cwd}),
            Pending::Commits,
        )
    }

    /// The diff of one file in one commit; `None` when the commit has no text diff for it.
    pub fn commit_file_diff(
        &mut self,
        cwd: &str,
        sha: &str,
        path: &str,
    ) -> oneshot::Receiver<anyhow::Result<Option<FileDiff>>> {
        self.request_reply(
            json!({"type": "checkout.commits.file_diff.request", "cwd": cwd, "sha": sha, "path": path}),
            Pending::CommitFileDiff,
        )
    }

    /// Sets a provider feature; `value` is a bool for a toggle and an option id for a select.
    pub fn set_agent_feature(
        &mut self,
        agent_id: &str,
        feature_id: &str,
        value: Value,
    ) -> oneshot::Receiver<anyhow::Result<()>> {
        self.request_reply(
            json!({
                "type": "set_agent_feature_request",
                "agentId": agent_id,
                "featureId": feature_id,
                "value": value,
            }),
            Pending::SetFeature,
        )
    }

    /// The unified diff of one uncommitted file; empty when the file has no change.
    pub fn file_diff(
        &mut self,
        cwd: &str,
        path: &str,
    ) -> oneshot::Receiver<anyhow::Result<String>> {
        self.request_reply(
            json!({"type": "checkout.diff.get.request", "cwd": cwd, "compare": {"mode": UNCOMMITTED}}),
            |reply| Pending::FileDiff {
                path: path.to_string(),
                reply,
            },
        )
    }

    /// Emits `CheckoutChanged` for `cwd` whenever its changed files change.
    pub fn watch_checkout(&mut self, cwd: &str) {
        if self.checkout_watches.contains_key(cwd) {
            return;
        }
        self.checkout_watches.insert(cwd.to_string(), None);
        self.subscribe_checkout(cwd.to_string());
    }

    pub fn unwatch_checkout(&mut self, cwd: &str) {
        if let Some(Some(subscription_id)) = self.checkout_watches.remove(cwd) {
            self.send(json!({
                "type": "unsubscribe_checkout_diff_request",
                "subscriptionId": subscription_id,
            }));
        }
    }

    fn subscribe_checkout(&mut self, cwd: String) {
        let message = json!({
            "type": "subscribe_checkout_diff_request",
            "cwd": cwd,
            "compare": {"mode": UNCOMMITTED},
        });
        let _ = self.request(message, Pending::CheckoutWatch(cwd));
    }

    /// The project's `paseo.json` (null when it has none). The revision is kept for the next
    /// write.
    pub fn read_project_config(&mut self, root: &str) -> oneshot::Receiver<anyhow::Result<Value>> {
        self.request_reply(
            json!({"type": "read_project_config_request", "repoRoot": root}),
            |reply| Pending::ReadConfig {
                root: root.to_string(),
                reply,
            },
        )
    }

    /// Writes the project's `paseo.json`. It fails as stale when the file changed after the
    /// last read or write through this client.
    pub fn write_project_config(
        &mut self,
        root: &str,
        config: Value,
    ) -> oneshot::Receiver<anyhow::Result<()>> {
        let revision = self
            .config_revisions
            .get(root)
            .cloned()
            .unwrap_or(Value::Null);
        self.request_reply(
            json!({
                "type": "write_project_config_request",
                "repoRoot": root,
                "config": config,
                "expectedRevision": revision,
            }),
            |reply| Pending::WriteConfig {
                root: root.to_string(),
                reply,
            },
        )
    }

    // ---- plugins (TASK-168) ----

    /// The installed plugins in the daemon's order.
    pub fn list_plugins(&mut self) -> oneshot::Receiver<anyhow::Result<Vec<PluginListItem>>> {
        self.request_reply(json!({"type": "plugin.list.request"}), Pending::PluginList)
    }

    /// The ids of the loaded plugins (`plugin.catalog.get`).
    pub fn plugin_catalog_ids(&mut self) -> oneshot::Receiver<anyhow::Result<Vec<String>>> {
        self.request_reply(
            json!({"type": "plugin.catalog.get.request"}),
            Pending::PluginCatalog,
        )
    }

    /// Calls a plugin RPC and decodes its `output` with `decode`.
    fn plugin_call<T: Send + 'static>(
        &mut self,
        plugin_id: &str,
        method: &str,
        input: Value,
        decode: impl FnOnce(Value) -> anyhow::Result<T> + Send + 'static,
    ) -> oneshot::Receiver<anyhow::Result<T>> {
        let (reply, receiver) = oneshot::channel();
        let finish: RpcFinish = Box::new(move |result| {
            let _ = reply.send(result.and_then(decode));
        });
        let message = json!({
            "type": "plugin.rpc.invoke.request",
            "pluginId": plugin_id,
            "method": method,
            "input": input,
        });
        if let Err(pending) = self.request(message, Pending::PluginRpc(finish)) {
            pending.fail(anyhow!("not connected"));
        }
        receiver
    }

    /// Calls a plugin RPC; `output` must decode as `O`.
    pub fn plugin_rpc<O: serde::de::DeserializeOwned + Send + 'static>(
        &mut self,
        plugin_id: &str,
        method: &str,
        input: Value,
    ) -> oneshot::Receiver<anyhow::Result<O>> {
        let method_name = method.to_string();
        self.plugin_call(plugin_id, method, input, move |output| {
            serde_json::from_value(output)
                .map_err(|error| anyhow!("{method_name} output does not decode: {error}"))
        })
    }

    pub fn read_plugin_settings(
        &mut self,
        plugin_id: &str,
        settings_id: &str,
    ) -> oneshot::Receiver<anyhow::Result<PluginSettingsRead>> {
        let method = format!("settings.{settings_id}.read");
        self.plugin_call(plugin_id, &method, json!({}), plugins::settings_read)
    }

    pub fn write_plugin_settings(
        &mut self,
        plugin_id: &str,
        settings_id: &str,
        revision: &str,
        values: Value,
    ) -> oneshot::Receiver<anyhow::Result<PluginSettingsWrite>> {
        let method = format!("settings.{settings_id}.write");
        let input = json!({"revision": revision, "values": values});
        self.plugin_call(plugin_id, &method, input, plugins::settings_write)
    }

    pub fn reset_plugin_settings(
        &mut self,
        plugin_id: &str,
        settings_id: &str,
        revision: &str,
    ) -> oneshot::Receiver<anyhow::Result<PluginSettingsWrite>> {
        let method = format!("settings.{settings_id}.reset");
        let input = json!({"revision": revision});
        self.plugin_call(plugin_id, &method, input, plugins::settings_write)
    }

    /// The agent's labels; empty when the snapshot has none, `None` for an unknown agent.
    pub fn agent_labels(&self, agent_id: &str) -> Option<BTreeMap<String, String>> {
        if !self.agents.iter().any(|agent| agent.id == agent_id) {
            return None;
        }
        Some(
            self.extras
                .get(agent_id)
                .map(|extra| extra.labels.clone())
                .unwrap_or_default(),
        )
    }

    /// Sets these labels; the daemon keeps the agent's other labels.
    pub fn set_agent_labels(
        &mut self,
        agent_id: &str,
        labels: BTreeMap<String, String>,
    ) -> oneshot::Receiver<anyhow::Result<()>> {
        self.request_reply(
            json!({"type": "update_agent_request", "agentId": agent_id, "labels": labels}),
            Pending::Accepted,
        )
    }

    /// Closes the agent's provider runtime; the agent stays and resumes on the next prompt.
    pub fn close_agent_runtime(&mut self, agent_id: &str) -> oneshot::Receiver<anyhow::Result<()>> {
        self.request_reply(
            json!({"type": "agent.runtime.close.request", "agentId": agent_id}),
            Pending::Accepted,
        )
    }

    pub fn cancel_agent(&mut self, agent_id: &str) {
        let _ = self.request(
            json!({"type": "cancel_agent_request", "agentId": agent_id}),
            Pending::Action("cancel_agent"),
        );
    }

    /// Stops one background process of the agent. A refused stop logs a warning.
    pub fn stop_background_process(&mut self, agent_id: &str, process_id: &str) {
        let _ = self.request(
            json!({"type": "agent.background_process.stop.request", "agentId": agent_id,
                   "processId": process_id}),
            Pending::Action("agent.background_process.stop"),
        );
    }

    pub fn set_mode(&mut self, agent_id: &str, mode_id: &str) {
        let _ = self.request(
            json!({"type": "set_agent_mode_request", "agentId": agent_id, "modeId": mode_id}),
            Pending::Action("set_agent_mode"),
        );
    }

    /// Asks the daemon to discover models again, then reloads them. The daemon caches each
    /// provider's models until a refresh or a restart. `None` refreshes every available provider.
    pub fn refresh_models(&mut self, provider: Option<&str>) {
        let providers: Vec<String> = match provider {
            Some(provider) => vec![provider.to_string()],
            None => self
                .providers
                .iter()
                .filter(|p| p.available)
                .map(|p| p.id.clone())
                .collect(),
        };
        let _ = self.request(
            json!({"type": "refresh_providers_snapshot_request", "providers": providers}),
            Pending::RefreshModels(providers),
        );
    }

    pub fn set_model(&mut self, agent_id: &str, model_id: &str) {
        let _ = self.request(
            json!({"type": "set_agent_model_request", "agentId": agent_id, "modelId": model_id}),
            Pending::Action("set_agent_model"),
        );
    }

    /// Answers a permission request. A request with actions gets the first action of the
    /// chosen behavior, as the app's default buttons do.
    pub fn respond_permission(&mut self, agent_id: &str, request_id: &str, allow: bool) {
        let behavior = if allow { "allow" } else { "deny" };
        let mut response = json!({"behavior": behavior});
        let action = self
            .extras
            .get(agent_id)
            .and_then(|extra| extra.permissions.iter().find(|p| p["id"] == request_id))
            .and_then(|request| {
                request["actions"]
                    .as_array()?
                    .iter()
                    .find(|action| action["behavior"] == behavior)?["id"]
                    .as_str()
                    .map(str::to_string)
            });
        if let Some(action) = action {
            response["selectedActionId"] = json!(action);
        }
        // The daemon answers with `agent_permission_resolved`, not a response.
        self.send(json!({
            "type": "agent_permission_response",
            "agentId": agent_id,
            "requestId": request_id,
            "response": response,
        }));
    }

    pub fn create_agent(
        &mut self,
        provider: &str,
        model: Option<&str>,
        cwd: &str,
        prompt: &str,
    ) -> oneshot::Receiver<anyhow::Result<String>> {
        let (reply, receiver) = oneshot::channel();
        let mut config = json!({"provider": provider, "cwd": cwd});
        if let Some(model) = model {
            config["model"] = json!(model);
        }
        let mut message = json!({"type": "create_agent_request", "config": config, "labels": {}});
        if !prompt.is_empty() {
            message["initialPrompt"] = json!(prompt);
            message["clientMessageId"] = json!(uuid::Uuid::new_v4().to_string());
        }
        if let Err(Pending::CreateAgent(reply)) = self.request(message, Pending::CreateAgent(reply))
        {
            let _ = reply.send(Err(anyhow!("not connected")));
        }
        receiver
    }

    pub fn create_terminal(
        &mut self,
        cwd: &str,
    ) -> oneshot::Receiver<anyhow::Result<TerminalHandle>> {
        let (reply, receiver) = oneshot::channel();
        let mut message = json!({"type": "create_terminal_request", "cwd": cwd});
        // The daemon needs a workspace; it resolves one from cwd only for known folders.
        if let Some(workspace) = self
            .workspaces
            .iter()
            .find(|workspace| workspace.cwd == cwd)
        {
            message["workspaceId"] = json!(workspace.id);
        }
        if let Err(Pending::CreateTerminal(reply)) =
            self.request(message, Pending::CreateTerminal(reply))
        {
            let _ = reply.send(Err(anyhow!("not connected")));
        }
        receiver
    }

    pub fn attach_terminal(
        &mut self,
        terminal_id: &str,
    ) -> oneshot::Receiver<anyhow::Result<TerminalHandle>> {
        let (reply, receiver) = oneshot::channel();
        if self.generation.is_none() {
            let _ = reply.send(Err(anyhow!("not connected")));
        } else {
            self.subscribe_terminal(terminal_id.to_string(), Some(reply));
        }
        receiver
    }

    /// Subscribes to a terminal. Without `reply` it renews the subscription of an open handle.
    ///
    /// The request carries no `restore` mode on purpose: with `restore` the daemon sends the
    /// state as ANSI text (opcode 0x05), and only without it does it send the JSON cell grid
    /// (opcode 0x04) that `TerminalFrame::Snapshot` promises. Both carry the full scrollback.
    fn subscribe_terminal(&mut self, terminal_id: String, reply: Option<Reply<TerminalHandle>>) {
        let reply = reply.map(|reply| {
            let (sender, receiver) = mpsc::unbounded();
            self.terminals
                .insert(terminal_id.clone(), TerminalStream { sender, slot: None });
            (reply, receiver)
        });
        let pending = Pending::SubscribeTerminal {
            terminal_id: terminal_id.clone(),
            reply,
        };
        let message = json!({"type": "subscribe_terminal_request", "terminalId": terminal_id});
        if let Err(Pending::SubscribeTerminal {
            reply: Some((reply, _)),
            ..
        }) = self.request(message, pending)
        {
            self.terminals.remove(&terminal_id);
            let _ = reply.send(Err(anyhow!("not connected")));
        }
    }

    pub fn terminal_input(&mut self, terminal_id: &str, bytes: &[u8]) {
        self.send_terminal(terminal_id, terminal::INPUT, bytes);
    }

    pub fn terminal_resize(&mut self, terminal_id: &str, rows: u16, cols: u16, claim: bool) {
        let intent = if claim { "claim" } else { "update" };
        let payload = json!({"rows": rows, "cols": cols, "intent": intent}).to_string();
        self.send_terminal(terminal_id, terminal::RESIZE, payload.as_bytes());
    }

    fn send_terminal(&mut self, terminal_id: &str, opcode: u8, payload: &[u8]) {
        let (Some(generation), Some(slot)) = (
            self.generation,
            self.terminals
                .get(terminal_id)
                .and_then(|stream| stream.slot),
        ) else {
            return;
        };
        self.conn
            .send_binary(generation, terminal::encode(opcode, slot, payload));
    }

    pub fn kill_terminal(&mut self, terminal_id: &str) {
        if let Some(stream) = self.terminals.remove(terminal_id)
            && let Some(slot) = stream.slot
        {
            self.slots.remove(&slot);
        }
        let _ = self.request(
            json!({"type": "kill_terminal_request", "terminalId": terminal_id}),
            Pending::Action("kill_terminal"),
        );
    }

    // ---- git, PR, stash, and script calls (TASK-204) ----

    /// Runs a git or PR action as the app's `actions-store.ts` does: a commit adds every
    /// change, and a pull and push pushes only after the pull succeeds.
    pub fn git_action(
        &mut self,
        cwd: &str,
        action: GitAction,
    ) -> oneshot::Receiver<anyhow::Result<GitActionReply>> {
        let message = match action {
            GitAction::Commit => {
                json!({"type": "checkout_commit_request", "cwd": cwd, "addAll": true})
            }
            GitAction::Pull | GitAction::PullAndPush => {
                json!({"type": "checkout_pull_request", "cwd": cwd})
            }
            GitAction::Push => json!({"type": "checkout_push_request", "cwd": cwd}),
            GitAction::CreatePr => json!({"type": "checkout_pr_create_request", "cwd": cwd}),
            GitAction::MergePr(method) => json!({"type": "checkout_pr_merge_request", "cwd": cwd,
                                                 "mergeMethod": method.wire()}),
            GitAction::SetAutoMerge { enabled, method } => {
                let Some(kind) = self.auto_merge_request() else {
                    let (reply, receiver) = oneshot::channel();
                    let _ = reply.send(Err(anyhow!("Update the host to use auto-merge actions.")));
                    return receiver;
                };
                let mut message = json!({"type": kind, "cwd": cwd, "enabled": enabled});
                if let Some(method) = method {
                    message["mergeMethod"] = json!(method.wire());
                }
                message
            }
            // The daemon merges into the checkout's base ref when the request names none.
            GitAction::MergeBranch => json!({"type": "checkout_merge_request", "cwd": cwd,
                                             "strategy": "merge", "requireCleanTarget": true}),
            GitAction::MergeFromBase => json!({"type": "checkout_merge_from_base_request",
                                               "cwd": cwd, "requireCleanTarget": true}),
            GitAction::Refresh => json!({"type": "checkout.refresh.request", "cwd": cwd}),
        };
        if action == GitAction::PullAndPush {
            let cwd = cwd.to_string();
            return self.request_reply(message, |reply| Pending::PullThenPush { cwd, reply });
        }
        self.request_reply(message, Pending::GitAction)
    }

    /// The auto-merge request type the host serves; `None` when it serves neither.
    fn auto_merge_request(&self) -> Option<&'static str> {
        let features = &self.server_info.as_ref()?["features"];
        if features["checkoutForgeSetAutoMerge"] == true {
            return Some("checkout.forge.set_auto_merge.request");
        }
        // COMPAT(githubAutoMergeRpc): added in v0.2.0-beta.1, remove after 2027-01-17
        // once the supported daemon floor is >= v0.2.0.
        (features["checkoutGithubSetAutoMerge"] == true)
            .then_some("checkout.github.set_auto_merge.request")
    }

    /// The full git status of `cwd`. A folder outside git gives `is_git: false`.
    pub fn git_status(&mut self, cwd: &str) -> oneshot::Receiver<anyhow::Result<GitStatus>> {
        self.request_reply(
            json!({"type": "checkout_status_request", "cwd": cwd}),
            Pending::GitStatus,
        )
    }

    pub fn pr_status(&mut self, cwd: &str) -> oneshot::Receiver<anyhow::Result<PrStatusReply>> {
        self.request_reply(
            json!({"type": "checkout_pr_status_request", "cwd": cwd}),
            Pending::PrStatus,
        )
    }

    pub fn pr_timeline(
        &mut self,
        cwd: &str,
        number: u64,
        repo_owner: &str,
        repo_name: &str,
    ) -> oneshot::Receiver<anyhow::Result<Vec<PrTimelineItem>>> {
        self.request_reply(
            json!({"type": "pull_request_timeline_request", "cwd": cwd, "prNumber": number,
                   "repoOwner": repo_owner, "repoName": repo_name}),
            Pending::PrTimeline,
        )
    }

    pub fn switch_branch(
        &mut self,
        cwd: &str,
        branch: &str,
    ) -> oneshot::Receiver<anyhow::Result<()>> {
        self.request_reply(
            json!({"type": "checkout_switch_branch_request", "cwd": cwd, "branch": branch}),
            Pending::CheckoutDone,
        )
    }

    /// Stashes every change, tagged with `branch` when set.
    pub fn stash_save(
        &mut self,
        cwd: &str,
        branch: Option<&str>,
    ) -> oneshot::Receiver<anyhow::Result<()>> {
        let mut message = json!({"type": "stash_save_request", "cwd": cwd});
        if let Some(branch) = branch {
            message["branch"] = json!(branch);
        }
        self.request_reply(message, Pending::CheckoutDone)
    }

    pub fn stash_pop(&mut self, cwd: &str, index: u32) -> oneshot::Receiver<anyhow::Result<()>> {
        self.request_reply(
            json!({"type": "stash_pop_request", "cwd": cwd, "stashIndex": index}),
            Pending::CheckoutDone,
        )
    }

    pub fn stash_list(
        &mut self,
        cwd: &str,
        paseo_only: bool,
    ) -> oneshot::Receiver<anyhow::Result<Vec<StashEntry>>> {
        self.request_reply(
            json!({"type": "stash_list_request", "cwd": cwd, "paseoOnly": paseo_only}),
            Pending::StashList,
        )
    }

    /// The workspace's scripts, as the last descriptor or `script_status_update` gave them.
    pub fn workspace_scripts(&self, workspace_id: &str) -> Vec<WorkspaceScript> {
        self.workspace_scripts
            .get(workspace_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Starts a workspace script and resolves to its terminal id.
    pub fn start_workspace_script(
        &mut self,
        workspace_id: &str,
        script_name: &str,
    ) -> oneshot::Receiver<anyhow::Result<String>> {
        self.request_reply(
            json!({"type": "start_workspace_script_request", "workspaceId": workspace_id,
                   "scriptName": script_name}),
            Pending::StartScript,
        )
    }

    /// Stores a workspace's scripts and reports a change.
    fn set_workspace_scripts(&mut self, workspace_id: &str, scripts: Vec<WorkspaceScript>) {
        if self.workspace_scripts.get(workspace_id) == Some(&scripts) {
            return;
        }
        self.workspace_scripts
            .insert(workspace_id.to_string(), scripts);
        self.git_status_events.push(GitStatusEvent::ScriptsChanged {
            workspace_id: workspace_id.to_string(),
        });
    }

    // ---- incoming ----

    fn binary(&mut self, bytes: &[u8]) {
        if bytes
            .first()
            .copied()
            .is_some_and(files::is_transfer_opcode)
        {
            return self.transfer(bytes);
        }
        let Some((slot, frame)) = terminal::parse_frame(bytes) else {
            return;
        };
        let Some(terminal_id) = self.slots.get(&slot).cloned() else {
            return;
        };
        let delivered = self
            .terminals
            .get(&terminal_id)
            .is_some_and(|stream| stream.sender.unbounded_send(frame).is_ok());
        if !delivered {
            // The view dropped its handle.
            self.terminals.remove(&terminal_id);
            self.slots.remove(&slot);
            self.send(json!({"type": "unsubscribe_terminal_request", "terminalId": terminal_id}));
        }
    }

    /// A file transfer frame of a `read_file` request. `FileEnd` completes the read; the
    /// daemon sends no JSON response then.
    fn transfer(&mut self, bytes: &[u8]) {
        let Some(frame) = files::parse_transfer(bytes) else {
            return;
        };
        match frame {
            TransferFrame::Begin {
                request_id,
                metadata,
            } => {
                if let Some(Pending::ReadFile { transfer, .. }) = self.pending.get_mut(&request_id)
                {
                    *transfer = Some(Box::new((metadata, Vec::new())));
                }
            }
            TransferFrame::Chunk { request_id, bytes } => {
                if let Some(Pending::ReadFile {
                    transfer: Some(transfer),
                    ..
                }) = self.pending.get_mut(&request_id)
                {
                    transfer.1.extend_from_slice(&bytes);
                }
            }
            TransferFrame::End { request_id } => {
                if let Some(Pending::ReadFile {
                    transfer: Some(_), ..
                }) = self.pending.get(&request_id)
                    && let Some(Pending::ReadFile {
                        path,
                        reply,
                        transfer: Some(transfer),
                    }) = self.pending.remove(&request_id)
                {
                    let (metadata, buffer) = *transfer;
                    let _ = reply.send(Ok(files::streamed_file(&path, &metadata, buffer)));
                }
            }
        }
    }

    /// Resolves a subscribed `create_workspace` reply once its agent exists, so its tab opens
    /// before the first turn starts. The later response finds no pending request.
    fn workspace_creation_update(&mut self, payload: &Value) {
        let Some(created) = composer_data::created_workspace_update(payload) else {
            return;
        };
        let key = payload["idempotencyKey"].as_str();
        let request_id = self.pending.iter().find_map(|(id, pending)| {
            matches!(pending, Pending::CreateWorkspace { key: Some(k), .. } if Some(k.as_str()) == key)
                .then(|| id.clone())
        });
        if let Some(Pending::CreateWorkspace { reply, .. }) =
            request_id.and_then(|id| self.pending.remove(&id))
        {
            let _ = reply.send(Ok(created));
        }
    }

    fn message(&mut self, message: Value) {
        let kind = message["type"].as_str().unwrap_or_default().to_string();
        if let Some(pending) = message["payload"]["requestId"]
            .as_str()
            .and_then(|id| self.pending.remove(id))
        {
            self.response(pending, &kind, &message["payload"]);
            return;
        }
        let payload = &message["payload"];
        match kind.as_str() {
            "agent_update" => match payload["kind"].as_str() {
                Some("upsert") => self.upsert_agent(&payload["agent"]),
                Some("remove") => {
                    self.remove_agent(payload["agentId"].as_str().unwrap_or_default())
                }
                _ => {}
            },
            "agent_deleted" => self.remove_agent(payload["agentId"].as_str().unwrap_or_default()),
            "terminal_stream_exit" => {
                // Dropping the sender ends the view's frame stream.
                let terminal_id = payload["terminalId"].as_str().unwrap_or_default();
                if let Some(stream) = self.terminals.remove(terminal_id)
                    && let Some(slot) = stream.slot
                {
                    self.slots.remove(&slot);
                }
            }
            "workspace_update" => match payload["kind"].as_str() {
                Some("upsert") => self.upsert_workspace(&payload["workspace"]),
                Some("remove") => {
                    let id = payload["id"].as_str().unwrap_or_default();
                    self.workspaces.retain(|workspace| workspace.id != id);
                    self.workspace_meta.remove(id);
                    self.workspace_scripts.remove(id);
                    self.directory_changed = true;
                }
                _ => {}
            },
            "workspace.label.update" => self.label_update(payload),
            "workspace.create.update" => self.workspace_creation_update(payload),
            "project.update" => match payload["kind"].as_str() {
                Some("upsert") => {
                    if let Some(project) = project_info(&payload["project"]) {
                        match self.project_list.iter_mut().find(|p| p.id == project.id) {
                            Some(existing) => *existing = project,
                            None => self.project_list.push(project),
                        }
                        self.projects_dirty = true;
                    }
                }
                Some("remove") => {
                    let id = payload["projectId"].as_str().unwrap_or_default();
                    self.project_list.retain(|project| project.id != id);
                    self.empty_projects.retain(|project| project.id != id);
                    self.projects_dirty = true;
                }
                _ => {}
            },
            "checkout_diff_update" => {
                let subscription = payload["subscriptionId"].as_str().unwrap_or_default();
                let cwd = self
                    .checkout_watches
                    .iter()
                    .find(|(_, id)| id.as_deref() == Some(subscription))
                    .map(|(cwd, _)| cwd.clone());
                if let Some(cwd) = cwd {
                    self.directory_events
                        .push(DirectoryEvent::CheckoutChanged { cwd });
                }
            }
            "checkout_status_update" => {
                if let Some(cwd) = payload["cwd"].as_str() {
                    self.git_status_events.push(GitStatusEvent::Changed {
                        cwd: cwd.to_string(),
                    });
                }
            }
            "script_status_update" => {
                if let Some(workspace_id) = payload["workspaceId"].as_str() {
                    let scripts = WorkspaceScript::parse_list(&payload["scripts"]);
                    self.set_workspace_scripts(workspace_id, scripts);
                }
            }
            "agent_stream" => self.stream(payload),
            "agent.timeline.replacement" => {
                let agent_id = payload["agentId"].as_str().unwrap_or_default().to_string();
                if let Some(sync) = self.timelines.get_mut(&agent_id) {
                    let plan = sync.plan_tail();
                    self.send_plan(&agent_id, plan);
                }
            }
            "agent_permission_request" => {
                let agent_id = payload["agentId"].as_str().unwrap_or_default().to_string();
                self.add_permission(&agent_id, payload["request"].clone());
            }
            "agent_permission_resolved" => {
                let agent_id = payload["agentId"].as_str().unwrap_or_default().to_string();
                self.remove_permission(
                    &agent_id,
                    payload["requestId"].as_str().unwrap_or_default(),
                );
            }
            "status" => self.plugin_status(payload),
            "agent.provider_subagents.update" => match payload["kind"].as_str() {
                Some("upsert") => {
                    let Some(subagent) = composer_data::provider_subagent(&payload["subagent"])
                    else {
                        return;
                    };
                    let rows = self
                        .provider_subagents
                        .entry(subagent.parent_agent_id.clone())
                        .or_default();
                    match rows.iter_mut().find(|row| row.row.id == subagent.row.id) {
                        Some(existing) => *existing = subagent,
                        None => rows.push(subagent),
                    }
                    self.directory_changed = true;
                }
                Some("timeline") => self.provider_subagent_timeline_update(payload),
                Some("remove") => {
                    let parent = payload["parentAgentId"].as_str().unwrap_or_default();
                    let id = payload["subagentId"].as_str().unwrap_or_default();
                    if let Some(rows) = self.provider_subagents.get_mut(parent) {
                        rows.retain(|row| row.row.id != id);
                        self.directory_changed = true;
                    }
                    // A removed child drops its timeline, as the app's store does.
                    let key = provider_subagent_key(parent, id);
                    if self
                        .viewed_provider_subagents
                        .iter()
                        .any(|viewed| viewed == &key)
                    {
                        self.viewed_provider_subagents
                            .retain(|viewed| viewed != &key);
                        self.timelines.remove(&key);
                        self.dirty_timelines.insert(key);
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }

    /// Applies a live child timeline item for a viewed key. An item of another epoch, or one
    /// whose seq is not above the last, is dropped.
    fn provider_subagent_timeline_update(&mut self, payload: &Value) {
        let (Some(parent), Some(child)) = (
            payload["parentAgentId"].as_str(),
            payload["subagentId"].as_str(),
        ) else {
            return;
        };
        let key = provider_subagent_key(parent, child);
        if !self
            .viewed_provider_subagents
            .iter()
            .any(|viewed| *viewed == key)
        {
            return;
        }
        let (Some(seq), Some(epoch)) = (payload["seq"].as_u64(), payload["epoch"].as_str()) else {
            return;
        };
        let Some(sync) = self.timelines.get_mut(&key) else {
            return;
        };
        if sync.epoch().is_some_and(|current| current != epoch) {
            return;
        }
        let entry = json!({
            "provider": payload["provider"],
            "item": payload["item"],
            "timestamp": payload["timestamp"],
            "seqStart": seq,
            "seqEnd": seq,
            "sourceSeqRanges": [{"startSeq": seq, "endSeq": seq}],
            "collapsed": [],
        });
        let version = sync.state.version;
        let plan = sync.on_live(LiveRow {
            seq,
            epoch: epoch.to_string(),
            entry,
        });
        if sync.state.version != version {
            self.dirty_timelines.insert(key.clone());
        }
        if let Some(plan) = plan {
            self.send_provider_plan(&key, plan);
        }
    }

    /// A `status` event without a request: only the plugin change statuses are read.
    fn plugin_status(&mut self, payload: &Value) {
        let plugin_id = payload["pluginId"].as_str().map(str::to_string);
        let event = match (payload["status"].as_str(), plugin_id) {
            (Some("plugin_catalog_changed"), Some(plugin_id)) => {
                PluginEvent::CatalogChanged { plugin_id }
            }
            (Some("plugin_settings_changed"), Some(plugin_id)) => {
                let Some(settings_id) = payload["settingsId"].as_str() else {
                    return;
                };
                PluginEvent::SettingsChanged {
                    plugin_id,
                    settings_id: settings_id.to_string(),
                }
            }
            _ => return,
        };
        self.plugin_events.push(event);
    }

    fn response(&mut self, pending: Pending, kind: &str, payload: &Value) {
        let error = if kind == "rpc_error" {
            Some(
                payload["error"]
                    .as_str()
                    .unwrap_or("request failed")
                    .to_string(),
            )
        } else {
            payload["error"].as_str().map(str::to_string)
        };
        match pending {
            Pending::Agents { first } => {
                if let Some(error) = error {
                    return log::warn!("fetch_agents failed: {error}");
                }
                self.agents_page(payload, first);
            }
            Pending::Workspaces { first } => {
                if let Some(error) = error {
                    return log::warn!("fetch_workspaces failed: {error}");
                }
                self.workspaces_page(payload, first);
            }
            Pending::Providers => {
                if let Some(error) = error {
                    return log::warn!("list_available_providers failed: {error}");
                }
                self.providers_list(payload);
            }
            Pending::RefreshModels(providers) => {
                if let Some(error) = error {
                    return log::warn!("refresh_providers_snapshot failed: {error}");
                }
                for provider in providers {
                    let _ = self.request(
                        json!({"type": "list_provider_models_request", "provider": provider}),
                        Pending::Models(provider),
                    );
                }
            }
            Pending::Models(provider) => {
                if let Some(error) = error {
                    return log::warn!("list_provider_models {provider} failed: {error}");
                }
                for model in payload["models"].as_array().into_iter().flatten() {
                    let Some(id) = model["id"].as_str() else {
                        continue;
                    };
                    if model["isDefault"] == true {
                        self.default_models.insert(provider.clone(), id.to_string());
                    }
                    let options = model["thinkingOptions"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|option| {
                            let id = option["id"].as_str()?.to_string();
                            let label = option["label"].as_str().unwrap_or(&id).to_string();
                            Some((id, label))
                        })
                        .collect();
                    let default = model["defaultThinkingOptionId"]
                        .as_str()
                        .or_else(|| {
                            model["thinkingOptions"]
                                .as_array()?
                                .iter()
                                .find(|option| option["isDefault"] == true)?["id"]
                                .as_str()
                        })
                        .map(str::to_string);
                    let thinking = ModelThinking { options, default };
                    self.model_thinking
                        .insert((provider.clone(), id.to_string()), thinking);
                }
                let models: Vec<(String, String)> = payload["models"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter(|model| model["isSelectable"] != false)
                    .filter_map(|model| {
                        let id = model["id"].as_str()?.to_string();
                        let label = model["label"].as_str().unwrap_or(&id).to_string();
                        Some((id, label))
                    })
                    .collect();
                if let Some(summary) = self.providers.iter_mut().find(|p| p.id == provider) {
                    summary.models = models;
                    self.directory_changed = true;
                }
            }
            Pending::Timeline {
                agent_id,
                plan,
                generation,
            } => {
                let page = match &error {
                    Some(error) if kind == "rpc_error" => json!({"error": error}),
                    _ => payload.clone(),
                };
                let Some(sync) = self.timelines.get_mut(&agent_id) else {
                    return;
                };
                if generation == sync.generation()
                    && let (Some(epoch), Some(cache)) =
                        (page["epoch"].as_str(), self.tasks.get_mut(&agent_id))
                {
                    let replaces = matches!(plan, FetchPlan::Tail) || page["reset"] == true;
                    if cache.epoch.as_deref() != Some(epoch) && (replaces || cache.epoch.is_none())
                    {
                        *cache = TasksCache {
                            epoch: Some(epoch.to_string()),
                            ..TasksCache::default()
                        };
                    }
                    cache.note(epoch, page["entries"].as_array().into_iter().flatten());
                }
                let version = sync.state.version;
                let next = sync.on_page(&plan, generation, &page);
                if sync.state.version != version {
                    self.dirty_timelines.insert(agent_id.clone());
                }
                if let Some(next) = next {
                    self.send_plan(&agent_id, next);
                }
            }
            Pending::ProviderTimeline {
                key,
                plan,
                generation,
            } => {
                let page = match &error {
                    Some(error) if kind == "rpc_error" => json!({"error": error}),
                    _ => payload.clone(),
                };
                let provider = page["provider"]
                    .as_str()
                    .map(str::to_string)
                    .or_else(|| self.provider_subagent_provider(&key))
                    .unwrap_or_else(|| "agent".to_string());
                let page = provider_subagent_page(&page, &provider);
                let Some(sync) = self.timelines.get_mut(&key) else {
                    return;
                };
                let version = sync.state.version;
                let next = sync.on_page(&plan, generation, &page);
                if sync.state.version != version {
                    self.dirty_timelines.insert(key.clone());
                }
                if let Some(next) = next {
                    self.send_provider_plan(&key, next);
                }
            }
            Pending::TimelineSubscription => {
                let Some(new_id) = payload["subscriptionId"].as_str().map(str::to_string) else {
                    if let Some(error) = error {
                        log::warn!("timeline subscription failed: {error}");
                    }
                    return;
                };
                let request_id = payload["requestId"].as_str();
                if request_id.is_some() && request_id == self.latest_timeline_request.as_deref() {
                    // Release the old set only after the new one holds, so no row is missed;
                    // rows delivered twice meanwhile are dropped by sequence.
                    if let Some(old) = self.timeline_subscription.replace(new_id) {
                        self.release(&old);
                    }
                } else {
                    self.release(&new_id);
                }
            }
            Pending::CreateAgent(reply) => {
                let result = match (payload["status"].as_str(), payload["agentId"].as_str()) {
                    (Some("agent_created"), Some(id)) => Ok(id.to_string()),
                    _ => Err(anyhow!(
                        error.unwrap_or_else(|| "agent creation failed".into())
                    )),
                };
                let _ = reply.send(result);
            }
            Pending::CreateTerminal(reply) => match payload["terminal"]["id"].as_str() {
                Some(id) if error.is_none() => self.subscribe_terminal(id.to_string(), Some(reply)),
                _ => {
                    let error = error.unwrap_or_else(|| "terminal creation failed".into());
                    let _ = reply.send(Err(anyhow!(error)));
                }
            },
            Pending::SubscribeTerminal { terminal_id, reply } => {
                let slot = payload["slot"].as_u64().filter(|_| error.is_none());
                let Some(slot) = slot.and_then(|slot| u8::try_from(slot).ok()) else {
                    let error = error.unwrap_or_else(|| "terminal subscription failed".into());
                    if let Some((reply, _)) = reply {
                        self.terminals.remove(&terminal_id);
                        let _ = reply.send(Err(anyhow!(error)));
                    } else {
                        // The terminal is gone after a reconnect; end the view's stream.
                        log::warn!("terminal {terminal_id} resubscribe failed: {error}");
                        self.terminals.remove(&terminal_id);
                    }
                    return;
                };
                let Some(stream) = self.terminals.get_mut(&terminal_id) else {
                    return;
                };
                stream.slot = Some(slot);
                self.slots.insert(slot, terminal_id.clone());
                if let Some((reply, frames)) = reply {
                    let handle = TerminalHandle {
                        id: terminal_id.clone(),
                        frames,
                    };
                    if reply.send(Ok(handle)).is_err() {
                        // Nobody waits for the handle any more.
                        self.terminals.remove(&terminal_id);
                        self.slots.remove(&slot);
                        self.send(
                            json!({"type": "unsubscribe_terminal_request", "terminalId": terminal_id}),
                        );
                    }
                }
            }
            Pending::Action(name) => {
                if let Some(error) = error {
                    log::warn!("{name} failed: {error}");
                }
            }
            Pending::ProjectRemove(project_id) => {
                let accepted = error.is_none() && payload["accepted"] == true;
                if !accepted {
                    log::warn!(
                        "project.remove failed: {}",
                        error.as_deref().unwrap_or("not accepted")
                    );
                    self.events
                        .push(ClientEvent::ProjectRemoveFailed { project_id });
                    return;
                }
                // The daemon sends workspace removals but no project update, so the project
                // would stay as an empty row.
                self.project_list.retain(|project| project.id != project_id);
                self.empty_projects
                    .retain(|project| project.id != project_id);
                self.projects_dirty = true;
                self.directory_changed = true;
            }
            pending => self.parity_response(pending, kind, payload, error),
        }
    }

    /// Answers of the requests added for the parity views.
    fn parity_response(
        &mut self,
        pending: Pending,
        kind: &str,
        payload: &Value,
        error: Option<String>,
    ) {
        // A checkout error is `{code, message}`, which `error` above does not read.
        let rpc_error = (kind == "rpc_error").then(|| error.clone()).flatten();
        match pending {
            Pending::LabelList => {
                if let Some(error) = error {
                    return log::warn!("workspace.label.list failed: {error}");
                }
                self.label_list(payload);
            }
            Pending::Projects => {
                if let Some(error) = error {
                    return log::warn!("project.list failed: {error}");
                }
                self.project_list = payload["projects"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(project_info)
                    .collect();
                self.projects_dirty = true;
            }
            Pending::FileList(reply) => {
                let result = match (error, payload["directory"].is_object()) {
                    (Some(error), _) => Err(anyhow!(error)),
                    (None, true) => Ok(files::directory_entries(&payload["directory"])),
                    (None, false) => Err(anyhow!("directory listing unavailable")),
                };
                let _ = reply.send(result);
            }
            Pending::ReadFile { reply, .. } => {
                let result = match (error, payload["file"].is_object()) {
                    (Some(error), _) => Err(anyhow!(error)),
                    (None, true) => files::json_file(&payload["file"]),
                    (None, false) => Err(anyhow!("file unavailable")),
                };
                let _ = reply.send(result);
            }
            Pending::CheckoutStatus(reply) => {
                if let Some(error) = rpc_error {
                    reply.send(Err(anyhow!(error)));
                    return;
                }
                if payload["isGit"] != true {
                    let result = match files::checkout_error(payload) {
                        Some((code, message)) if code != "NOT_GIT_REPO" => Err(anyhow!(message)),
                        _ => Ok(CheckoutDetails {
                            is_git: false,
                            branch: None,
                            files: Vec::new(),
                        }),
                    };
                    reply.send(result);
                    return;
                }
                if let Some((_, message)) = files::checkout_error(payload) {
                    reply.send(Err(anyhow!(message)));
                    return;
                }
                // A detached HEAD has no branch to show, as in the app.
                let branch = payload["currentBranch"]
                    .as_str()
                    .filter(|branch| *branch != "HEAD")
                    .map(str::to_string);
                let cwd = payload["cwd"].as_str().unwrap_or_default();
                let message = json!({
                    "type": "checkout.diff.get.request",
                    "cwd": cwd,
                    "compare": {"mode": UNCOMMITTED},
                });
                if let Err(pending) =
                    self.request(message, Pending::CheckoutFiles { branch, reply })
                {
                    pending.fail(anyhow!("not connected"));
                }
            }
            Pending::CheckoutFiles { branch, reply } => {
                let result = match (rpc_error, files::checkout_error(payload)) {
                    (Some(error), _) => Err(anyhow!(error)),
                    (None, Some((code, message))) if code != "NOT_GIT_REPO" => {
                        Err(anyhow!(message))
                    }
                    (None, error) => Ok(CheckoutDetails {
                        is_git: true,
                        branch,
                        files: if error.is_some() {
                            Vec::new()
                        } else {
                            files::changed_files(&payload["files"])
                        },
                    }),
                };
                reply.send(result);
            }
            Pending::CheckoutWatch(cwd) => {
                let Some(subscription_id) = payload["subscriptionId"].as_str() else {
                    let error = rpc_error.or(files::checkout_error(payload).map(|(_, m)| m));
                    return log::warn!("checkout watch of {cwd} failed: {error:?}");
                };
                match self.checkout_watches.get_mut(&cwd) {
                    Some(slot) => {
                        *slot = Some(subscription_id.to_string());
                        self.directory_events
                            .push(DirectoryEvent::CheckoutChanged { cwd });
                    }
                    // Unwatched while the subscription was on its way.
                    None => {
                        self.send(json!({
                            "type": "unsubscribe_checkout_diff_request",
                            "subscriptionId": subscription_id,
                        }));
                    }
                }
            }
            Pending::FileDiff { path, reply } => {
                let result = match (rpc_error, files::checkout_error(payload)) {
                    (Some(error), _) => Err(anyhow!(error)),
                    (None, Some((_, message))) => Err(anyhow!(message)),
                    (None, None) => {
                        let file = payload["files"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .find(|file| file["path"] == path.as_str());
                        match file {
                            None => Ok(String::new()),
                            Some(file) => match file["status"].as_str().unwrap_or("ok") {
                                "ok" => Ok(files::unified_diff(file)),
                                status => Err(anyhow!("{path} has no diff: {status}")),
                            },
                        }
                    }
                };
                let _ = reply.send(result);
            }
            Pending::CheckoutDiff(reply) => {
                let result = match (rpc_error, files::checkout_error(payload)) {
                    (Some(error), _) => Err(anyhow!(error)),
                    (None, Some((code, _))) if code == "NOT_GIT_REPO" => Ok(Vec::new()),
                    (None, Some((_, message))) => Err(anyhow!(message)),
                    (None, None) => Ok(payload["files"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(files::file_diff)
                        .collect()),
                };
                let _ = reply.send(result);
            }
            Pending::Commits(reply) => {
                let result = match (rpc_error, files::checkout_error(payload)) {
                    (Some(error), _) => Err(anyhow!(error)),
                    (None, Some((_, message))) => Err(anyhow!(message)),
                    (None, None) => Ok(files::commit_list(payload)),
                };
                let _ = reply.send(result);
            }
            Pending::CommitFileDiff(reply) => {
                let result = match (rpc_error, files::checkout_error(payload)) {
                    (Some(error), _) => Err(anyhow!(error)),
                    (None, Some((_, message))) => Err(anyhow!(message)),
                    (None, None) => Ok(files::file_diff(&payload["file"])),
                };
                let _ = reply.send(result);
            }
            Pending::SetFeature(reply) => {
                let result = match error {
                    Some(error) => Err(anyhow!(error)),
                    None if payload["accepted"] == false => Err(anyhow!("feature change refused")),
                    None => Ok(()),
                };
                let _ = reply.send(result);
            }
            Pending::Commands(reply) => {
                let result = match error {
                    Some(error) => Err(anyhow!(error)),
                    None => Ok(payload["commands"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|command| {
                            Some(SlashCommand {
                                name: command["name"].as_str()?.to_string(),
                                description: command["description"]
                                    .as_str()
                                    .unwrap_or_default()
                                    .to_string(),
                                skill: command["kind"].as_str() == Some("skill"),
                            })
                        })
                        .collect()),
                };
                let _ = reply.send(result);
            }
            Pending::Queue {
                agent_id,
                reply,
                images,
            } => {
                if let Some(error) = error {
                    log::warn!("queue request for {agent_id} failed: {error}");
                    if let Some(reply) = reply {
                        let _ = reply.send(Err(anyhow!(error)));
                    }
                    return;
                }
                if let Some(item_id) = payload["itemId"].as_str() {
                    self.remember_sent_images(item_id, images);
                }
                if payload["queue"].is_object()
                    && let Some(extra) = self.extras.get_mut(&agent_id)
                {
                    extra.queue = queued_prompts(&payload["queue"]);
                    extra.queue_held = payload["queue"]["held"] == true;
                    self.directory_changed = true;
                }
                if let Some(reply) = reply {
                    let _ = reply.send(Ok(()));
                }
            }
            Pending::ReadConfig { root, reply } => {
                let result = config_result(&rpc_error, payload).map(|()| {
                    self.config_revisions
                        .insert(root, payload["revision"].clone());
                    payload["config"].clone()
                });
                let _ = reply.send(result);
            }
            Pending::WriteConfig { root, reply } => {
                let result = config_result(&rpc_error, payload).map(|()| {
                    self.config_revisions
                        .insert(root, payload["revision"].clone());
                });
                let _ = reply.send(result);
            }
            pending => self.plugin_response(pending, kind, payload, error),
        }
    }

    /// Answers of the plugin requests and the agent actions plugins use (TASK-168).
    fn plugin_response(
        &mut self,
        pending: Pending,
        kind: &str,
        payload: &Value,
        error: Option<String>,
    ) {
        match pending {
            Pending::PluginList(reply) => {
                let result = match error {
                    Some(error) => Err(anyhow!(error)),
                    None => Ok(plugins::plugin_list(&payload["plugins"])),
                };
                let _ = reply.send(result);
            }
            Pending::PluginCatalog(reply) => {
                let result = match error {
                    Some(error) => Err(anyhow!(error)),
                    None => Ok(plugins::catalog_ids(&payload["plugins"])),
                };
                let _ = reply.send(result);
            }
            Pending::PluginRpc(finish) => finish(match error {
                Some(error) => Err(anyhow!(error)),
                None => Ok(payload.get("output").cloned().unwrap_or(Value::Null)),
            }),
            Pending::Accepted(reply) => {
                let result = match error {
                    Some(error) => Err(anyhow!(error)),
                    None if payload["accepted"] != true => Err(anyhow!("request refused")),
                    None => Ok(()),
                };
                let _ = reply.send(result);
            }
            Pending::Rewind {
                agent_id,
                mode,
                reply,
            } => {
                let result = match error {
                    Some(error) => Err(anyhow!(error)),
                    None if payload["ok"] != true => Err(anyhow!("Rewind failed")),
                    None => Ok(()),
                };
                // Only a rewind that worked changes the timeline.
                if result.is_ok()
                    && mode != RewindMode::Files
                    && let Some(sync) = self.timelines.get_mut(&agent_id)
                {
                    let plan = sync.plan_tail();
                    self.send_plan(&agent_id, plan);
                }
                let _ = reply.send(result);
            }
            pending => self.composer_response(pending, kind, payload, error),
        }
    }

    /// Answers of the composer, New workspace, and tab menu requests (TASK-169).
    fn composer_response(
        &mut self,
        pending: Pending,
        kind: &str,
        payload: &Value,
        error: Option<String>,
    ) {
        fn reply<T>(
            reply: Reply<T>,
            error: Option<String>,
            ok: impl FnOnce() -> anyhow::Result<T>,
        ) {
            let _ = reply.send(match error {
                Some(error) => Err(anyhow!(error)),
                None => ok(),
            });
        }
        match pending {
            Pending::ProviderSubagents(parent) => {
                if let Some(error) = error {
                    return log::warn!("provider subagents of {parent} failed: {error}");
                }
                let subagents = payload["subagents"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(composer_data::provider_subagent)
                    .collect();
                self.provider_subagents.insert(parent, subagents);
                self.directory_changed = true;
            }
            Pending::Modes(sender) => reply(sender, error, || {
                Ok(payload["modes"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|mode| {
                        let id = mode["id"].as_str()?.to_string();
                        let label = mode["label"].as_str().unwrap_or(&id).to_string();
                        Some((id, label))
                    })
                    .collect())
            }),
            Pending::DraftFeatures(sender) => {
                reply(sender, error, || Ok(agent_features(&payload["features"])))
            }
            Pending::CreateWorkspace { reply: sender, .. } => reply(sender, error, || {
                let workspace_id = payload["workspace"]["id"]
                    .as_str()
                    .ok_or_else(|| anyhow!("workspace creation failed"))?;
                Ok(CreatedWorkspace {
                    workspace_id: workspace_id.to_string(),
                    agent_id: payload["agent"]["id"].as_str().map(str::to_string),
                    setup_terminal_id: payload["setupTerminalId"].as_str().map(str::to_string),
                })
            }),
            Pending::BranchSuggestions(sender) => reply(sender, error, || {
                Ok(composer_data::branch_suggestions(payload))
            }),
            Pending::ValidateBranch(sender) => reply(sender, error, || {
                Ok(BranchValidation {
                    exists: payload["exists"] == true,
                    resolved_ref: payload["resolvedRef"].as_str().map(str::to_string),
                    is_remote: payload["isRemote"] == true,
                })
            }),
            Pending::AddProject(sender) => {
                let project = project_info(&payload["project"]);
                if let (None, Some(project)) = (&error, &project) {
                    match self.project_list.iter_mut().find(|p| p.id == project.id) {
                        Some(existing) => *existing = project.clone(),
                        None => self.project_list.push(project.clone()),
                    }
                    self.projects_dirty = true;
                }
                reply(sender, error, || {
                    let project = project.ok_or_else(|| anyhow!("project could not be added"))?;
                    let view_key = self
                        .project_view_key(&project.id)
                        .ok_or_else(|| anyhow!("project has no view key"))?;
                    Ok(AddedProject {
                        id: project.id,
                        name: project.name,
                        root: project.root,
                        view_key,
                    })
                })
            }
            Pending::Refresh(sender) => reply(sender, error, || match payload["status"].as_str() {
                Some("agent_refreshed") => Ok(()),
                _ => Err(anyhow!("unexpected refresh answer {kind}")),
            }),
            Pending::RenameTerminal(sender) => reply(sender, error, || {
                if payload["success"] == true {
                    Ok(())
                } else {
                    Err(anyhow!("terminal rename failed"))
                }
            }),
            Pending::History(sender) => {
                if error.is_none() {
                    self.archive_history_rows(payload);
                }
                reply(sender, error, || Ok(history_page(payload)))
            }
            Pending::FetchAgent(sender) => {
                let agent = &payload["agent"];
                // A live agent comes through the subscription; only an archived one is stored.
                if error.is_none() && !agent["archivedAt"].is_null() {
                    self.upsert_agent(agent);
                }
                reply(sender, error, || {
                    if agent.is_null() {
                        Err(anyhow!("agent not found"))
                    } else {
                        Ok(())
                    }
                })
            }
            Pending::ForkContext(sender) => reply(sender, error, || {
                let attachment = &payload["attachment"];
                if attachment.is_null() {
                    return Err(anyhow!("Nothing to fork"));
                }
                Ok(ForkContext {
                    attachment: attachment.clone(),
                    title: attachment["title"].as_str().map(str::to_string),
                    item_count: payload["itemCount"].as_u64().unwrap_or_default() as u32,
                })
            }),
            Pending::Prompts(sender) => reply(sender, error, || {
                let prompts = payload["prompts"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|prompt| {
                        Some(PromptEntry {
                            seq: prompt["seq"].as_u64()?,
                            preview: prompt["preview"].as_str()?.to_string(),
                        })
                    })
                    .collect();
                Ok(PromptIndex {
                    epoch: payload["epoch"].as_str().unwrap_or_default().to_string(),
                    prompts,
                })
            }),
            Pending::DirectorySuggestions(sender) => reply(sender, error, || {
                Ok(DirectorySuggestion::parse_all(payload))
            }),
            Pending::FsEntry(sender) => reply(sender, error, || {
                if payload["success"] == true {
                    Ok(())
                } else {
                    Err(anyhow!("the file operation failed"))
                }
            }),
            pending => self.git_response(pending, kind, payload, error),
        }
    }

    /// Answers of the git, PR, stash, and script requests (TASK-204).
    fn git_response(
        &mut self,
        pending: Pending,
        kind: &str,
        payload: &Value,
        error: Option<String>,
    ) {
        // An `rpc_error` carries a string; a checkout answer carries `{code, message}`.
        let checkout_error = match kind {
            "rpc_error" => error.clone(),
            _ => files::checkout_error(payload).map(|(_, message)| message),
        };
        fn send<T>(reply: Reply<T>, error: Option<String>, ok: impl FnOnce() -> T) {
            let _ = reply.send(match error {
                Some(error) => Err(anyhow!(error)),
                None => Ok(ok()),
            });
        }
        match pending {
            Pending::GitAction(reply) => send(reply, checkout_error, || GitActionReply {
                url: payload["url"].as_str().map(str::to_string),
                number: payload["number"].as_u64(),
            }),
            Pending::PullThenPush { cwd, reply } => {
                if let Some(error) = checkout_error {
                    let _ = reply.send(Err(anyhow!(error)));
                    return;
                }
                let message = json!({"type": "checkout_push_request", "cwd": cwd});
                if let Err(pending) = self.request(message, Pending::GitAction(reply)) {
                    pending.fail(anyhow!("not connected"));
                }
            }
            Pending::GitStatus(reply) => {
                let not_git = kind != "rpc_error"
                    && files::checkout_error(payload)
                        .is_some_and(|(code, _)| code == "NOT_GIT_REPO");
                if not_git {
                    let _ = reply.send(Ok(GitStatus::default()));
                    return;
                }
                send(reply, checkout_error, || GitStatus::parse(payload));
            }
            Pending::PrStatus(reply) => send(reply, checkout_error, || PrStatusReply {
                status: PrStatus::parse(&payload["status"]),
                github_features_enabled: payload["githubFeaturesEnabled"] == true,
            }),
            Pending::PrTimeline(reply) => {
                // A timeline error is `{kind, message}`; the message can be empty.
                let failure = match &payload["error"] {
                    _ if kind == "rpc_error" => error,
                    failure if failure.is_object() => Some(
                        failure["message"]
                            .as_str()
                            .filter(|message| !message.is_empty())
                            .or_else(|| failure["kind"].as_str())
                            .unwrap_or("unknown")
                            .to_string(),
                    ),
                    _ => None,
                };
                send(reply, failure, || {
                    payload["items"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .map(PrTimelineItem::parse)
                        .collect()
                });
            }
            Pending::StashList(reply) => send(reply, checkout_error, || {
                payload["entries"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(StashEntry::parse)
                    .collect()
            }),
            Pending::CheckoutDone(reply) => send(reply, checkout_error, || ()),
            Pending::StartScript(reply) => {
                let result = match (error, payload["terminalId"].as_str()) {
                    (Some(error), _) => Err(anyhow!(error)),
                    (None, Some(terminal_id)) => Ok(terminal_id.to_string()),
                    (None, None) => Err(anyhow!("the script started without a terminal")),
                };
                let _ = reply.send(result);
            }
            other => other.fail(anyhow!("unexpected response {kind}")),
        }
    }

    // ---- directory ----

    fn agents_page(&mut self, payload: &Value, first: bool) {
        let entries = payload["entries"]
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or_default();
        if first {
            let fresh: BTreeSet<&str> = entries
                .iter()
                .filter_map(|entry| entry["agent"]["id"].as_str())
                .collect();
            let gone: Vec<String> = self
                .agents
                .iter()
                .filter(|agent| !fresh.contains(agent.id.as_str()))
                .map(|agent| agent.id.clone())
                .collect();
            for id in gone {
                self.remove_agent(&id);
            }
        }
        for entry in entries {
            self.upsert_agent(&entry["agent"]);
        }
        self.directory_changed = true;
        if payload["pageInfo"]["hasMore"] == true
            && let Some(cursor) = payload["pageInfo"]["nextCursor"].as_str()
        {
            let _ = self.request(
                json!({
                    "type": "fetch_agents_request",
                    "sort": [{"key": "updated_at", "direction": "desc"}],
                    "page": {"limit": 200, "cursor": cursor},
                }),
                Pending::Agents { first: false },
            );
            return;
        }
        self.bootstrapped = true;
    }

    fn upsert_agent(&mut self, agent: &Value) {
        let Some(id) = agent["id"].as_str() else {
            return;
        };
        if !agent["archivedAt"].is_null() {
            self.remove_agent(id);
            self.archived
                .insert(id.to_string(), agent_summary(id, agent));
            self.directory_changed = true;
            return;
        }
        self.archived.remove(id);
        // The daemon can send an older snapshot after a newer one, for example `idle` after
        // `running` at agent start. ISO timestamps compare as strings.
        let updated_at = agent["updatedAt"].as_str().map(str::to_string);
        let stored = self
            .extras
            .get(id)
            .and_then(|extra| extra.updated_at.as_ref());
        if let (Some(new), Some(old)) = (&updated_at, stored)
            && new < old
        {
            return;
        }
        let runtime = &agent["runtimeInfo"];
        let permissions: Vec<Value> = agent["pendingPermissions"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let summary = agent_summary(id, agent);
        let modes = agent["availableModes"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|mode| {
                let id = mode["id"].as_str()?.to_string();
                let label = mode["label"].as_str().unwrap_or(&id).to_string();
                Some((id, label))
            })
            .collect();
        let current_mode = agent["currentModeId"]
            .as_str()
            .or_else(|| runtime["modeId"].as_str())
            .map(str::to_string);
        match self.agents.iter_mut().find(|existing| existing.id == id) {
            Some(existing) => {
                if existing.status == "running" && summary.status != "running" {
                    self.events.push(ClientEvent::AgentFinished {
                        agent_id: id.to_string(),
                    });
                }
                *existing = summary;
            }
            None => self.agents.push(summary),
        }
        let extra = self.extras.entry(id.to_string()).or_default();
        extra.modes = modes;
        extra.current_mode = current_mode;
        extra.thinking = agent["thinkingOptionId"]
            .as_str()
            .or_else(|| runtime["thinkingOptionId"].as_str())
            .map(str::to_string);
        extra.queue = queued_prompts(&agent["queue"]);
        extra.updated_at = updated_at;
        extra.requires_attention = agent["requiresAttention"] == true;
        extra.attention_reason = agent["attentionReason"]
            .as_str()
            .and_then(AttentionReason::parse);
        extra.attention_timestamp = agent["attentionTimestamp"]
            .as_str()
            .filter(|value| !value.trim().is_empty())
            .map(str::to_string);
        extra.features = agent_features(&agent["features"]);
        extra.labels = agent["labels"]
            .as_object()
            .into_iter()
            .flatten()
            .filter_map(|(key, value)| Some((key.clone(), value.as_str()?.to_string())))
            .collect();
        extra.title = agent["title"].as_str().map(str::to_string);
        extra.created_at = agent["createdAt"].as_str().map(str::to_string);
        extra.active_turn = composer_data::active_turn(&agent["activeTurn"]);
        extra.last_usage = composer_data::agent_usage(&agent["lastUsage"]);
        extra.session_id = runtime["sessionId"]
            .as_str()
            .or_else(|| agent["persistence"]["sessionId"].as_str())
            .map(str::to_string);
        extra.queue_held = agent["queue"]["held"] == true;
        extra.background_processes = BackgroundProcess::parse_all(&agent["backgroundProcesses"]);
        extra.skill_hook_usage = SkillHookUsage::parse(&agent["skillHookUsage"]);
        let capabilities = &agent["capabilities"];
        extra.rewind = RewindModes {
            conversation: capabilities["supportsRewindConversation"] == true,
            files: capabilities["supportsRewindFiles"] == true,
            both: capabilities["supportsRewindBoth"] == true,
        };
        self.set_permissions(id, permissions);
        self.directory_changed = true;
    }

    /// Archived history rows fill the archived map; a live agent stays where it is.
    fn archive_history_rows(&mut self, payload: &Value) {
        for entry in payload["entries"].as_array().into_iter().flatten() {
            let agent = &entry["agent"];
            let Some(id) = agent["id"].as_str() else {
                continue;
            };
            if agent["archivedAt"].is_null() || self.agents.iter().any(|a| a.id == id) {
                continue;
            }
            self.archived
                .insert(id.to_string(), agent_summary(id, agent));
            self.directory_changed = true;
        }
    }

    fn remove_agent(&mut self, id: &str) {
        let before = self.agents.len();
        self.agents.retain(|agent| agent.id != id);
        self.extras.remove(id);
        if self.agents.len() != before {
            self.directory_changed = true;
        }
    }

    fn workspaces_page(&mut self, payload: &Value, first: bool) {
        if first {
            self.workspaces.clear();
            self.workspace_meta.clear();
        }
        for workspace in payload["entries"].as_array().into_iter().flatten() {
            self.upsert_workspace(workspace);
        }
        // Projects with no active workspace come only on the first page.
        if first {
            self.empty_projects = payload["emptyProjects"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(project_info)
                .collect();
        }
        self.directory_changed = true;
        if payload["pageInfo"]["hasMore"] == true
            && let Some(cursor) = payload["pageInfo"]["nextCursor"].as_str()
        {
            let _ = self.request(
                json!({
                    "type": "fetch_workspaces_request",
                    "page": {"limit": 200, "cursor": cursor},
                }),
                Pending::Workspaces { first: false },
            );
        }
    }

    fn upsert_workspace(&mut self, workspace: &Value) {
        let Some(id) = workspace["id"].as_str() else {
            return;
        };
        let summary = WorkspaceSummary {
            id: id.to_string(),
            name: workspace["name"].as_str().unwrap_or(id).to_string(),
            cwd: workspace["workspaceDirectory"]
                .as_str()
                .or_else(|| workspace["projectRootPath"].as_str())
                .unwrap_or_default()
                .to_string(),
        };
        if let Some(project) = project_info(workspace) {
            let meta = WorkspaceMeta {
                project,
                branch: workspace["gitRuntime"]["currentBranch"]
                    .as_str()
                    .map(str::to_string),
                is_dirty: workspace["gitRuntime"]["isDirty"].as_bool(),
                pinned_at: workspace["pinnedAt"].as_str().map(str::to_string),
                diff_stat: workspace["diffStat"].is_object().then(|| DiffStat {
                    additions: workspace["diffStat"]["additions"]
                        .as_f64()
                        .unwrap_or_default() as u32,
                    deletions: workspace["diffStat"]["deletions"]
                        .as_f64()
                        .unwrap_or_default() as u32,
                }),
                pull_request: workspace_pull_request(workspace),
                label_keys: workspace["labels"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|name| name.as_str().map(workspace_label_key))
                    .collect(),
                status_entered_at: workspace["statusEnteredAt"].as_str().and_then(parse_iso_ms),
            };
            self.workspace_meta.insert(id.to_string(), meta);
        }
        if workspace["scripts"].is_array() {
            let scripts = WorkspaceScript::parse_list(&workspace["scripts"]);
            self.set_workspace_scripts(id, scripts);
        }
        match self
            .workspaces
            .iter_mut()
            .find(|existing| existing.id == id)
        {
            Some(existing) => *existing = summary,
            None => self.workspaces.push(summary),
        }
        self.directory_changed = true;
    }

    /// Asks for the host label catalog and its updates. A host without `workspaceLabels`
    /// sends no labels, so none are shown.
    fn request_label_list(&mut self) {
        if !self.server_feature("workspaceLabels") {
            return;
        }
        let _ = self.request(
            json!({"type": "workspace.label.list.request", "subscribe": {}}),
            Pending::LabelList,
        );
    }

    /// A `workspace.label.list` answer: a snapshot replaces the catalog, changes patch it.
    fn label_list(&mut self, payload: &Value) {
        let catalog = &mut self.label_catalog;
        if payload["sync"]["mode"] == "snapshot" {
            catalog.labels.clear();
        }
        for removal in payload["sync"]["removals"].as_array().into_iter().flatten() {
            if let Some(name) = removal["name"].as_str() {
                catalog.labels.remove(&workspace_label_key(name));
            }
        }
        for label in payload["labels"].as_array().into_iter().flatten() {
            if let Some(label) = label_definition(label) {
                catalog
                    .labels
                    .insert(workspace_label_key(&label.name), label);
            }
        }
        catalog.cursor = payload["sync"]["generation"]
            .as_str()
            .zip(payload["sync"]["headSeq"].as_u64())
            .map(|(generation, seq)| (generation.to_string(), seq));
        self.projects_dirty = true;
    }

    /// A `workspace.label.update` push. One that does not follow the cursor asks for a
    /// fresh snapshot.
    fn label_update(&mut self, update: &Value) {
        let catalog = &mut self.label_catalog;
        let in_order = catalog.cursor.as_ref().is_some_and(|(generation, seq)| {
            update["generation"].as_str() == Some(generation.as_str())
                && update["seq"].as_u64() == Some(seq + 1)
        });
        if !in_order {
            self.request_label_list();
            return;
        }
        match update["kind"].as_str() {
            Some("remove") => {
                if let Some(name) = update["name"].as_str() {
                    catalog.labels.remove(&workspace_label_key(name));
                }
            }
            Some("upsert") => {
                if let Some(previous) = update["previousName"].as_str() {
                    catalog.labels.remove(&workspace_label_key(previous));
                }
                if let Some(label) = label_definition(&update["label"]) {
                    catalog
                        .labels
                        .insert(workspace_label_key(&label.name), label);
                }
            }
            _ => {}
        }
        catalog.cursor = update["generation"]
            .as_str()
            .zip(update["seq"].as_u64())
            .map(|(generation, seq)| (generation.to_string(), seq));
        self.projects_dirty = true;
    }

    fn providers_list(&mut self, payload: &Value) {
        let previous = std::mem::take(&mut self.providers);
        for provider in payload["providers"].as_array().into_iter().flatten() {
            let Some(id) = provider["provider"].as_str() else {
                continue;
            };
            let available = provider["available"] == true;
            let models = previous
                .iter()
                .find(|p| p.id == id)
                .map(|p| p.models.clone())
                .unwrap_or_default();
            self.providers.push(ProviderSummary {
                id: id.to_string(),
                label: provider_label(id),
                available,
                models,
            });
            if available {
                let _ = self.request(
                    json!({"type": "list_provider_models_request", "provider": id}),
                    Pending::Models(id.to_string()),
                );
            }
        }
        self.directory_changed = true;
    }

    // ---- live stream and permissions ----

    fn stream(&mut self, payload: &Value) {
        let Some(agent_id) = payload["agentId"].as_str().map(str::to_string) else {
            return;
        };
        let event = &payload["event"];
        match event["type"].as_str() {
            Some("timeline") => {
                let Some(row) = LiveRow::from_stream(payload) else {
                    return;
                };
                let Some(sync) = self.timelines.get_mut(&agent_id) else {
                    return;
                };
                if let Some(cache) = self.tasks.get_mut(&agent_id) {
                    cache.note(&row.epoch, [&row.entry]);
                }
                let version = sync.state.version;
                let plan = sync.on_live(row);
                if sync.state.version != version {
                    self.dirty_timelines.insert(agent_id.clone());
                }
                if let Some(plan) = plan {
                    self.send_plan(&agent_id, plan);
                }
            }
            Some("permission_requested") => {
                self.add_permission(&agent_id, event["request"].clone())
            }
            Some("permission_resolved") => {
                self.remove_permission(&agent_id, event["requestId"].as_str().unwrap_or_default())
            }
            Some("turn_completed") => self.turn_completed(&agent_id, event),
            _ => {}
        }
    }

    /// Puts the turn's metrics on its last assistant entry, where a fresh fetch puts them.
    fn turn_completed(&mut self, agent_id: &str, event: &Value) {
        let (Some(turn_id), true) = (event["turnId"].as_str(), event["metrics"].is_object()) else {
            return;
        };
        let Some(sync) = self.timelines.get_mut(agent_id) else {
            return;
        };
        let Some(entry) = sync.state.entries.iter_mut().rev().find(|entry| {
            entry["item"]["type"] == "assistant_message" && entry["turnId"] == turn_id
        }) else {
            return;
        };
        entry["turnMetrics"] = event["metrics"].clone();
        sync.state.version += 1;
        self.dirty_timelines.insert(agent_id.to_string());
    }

    fn add_permission(&mut self, agent_id: &str, request: Value) {
        let Some(extra) = self.extras.get(agent_id) else {
            return;
        };
        if extra.permissions.iter().any(|p| p["id"] == request["id"]) {
            return;
        }
        let mut permissions = extra.permissions.clone();
        permissions.push(request);
        self.set_permissions(agent_id, permissions);
    }

    fn remove_permission(&mut self, agent_id: &str, request_id: &str) {
        let Some(extra) = self.extras.get(agent_id) else {
            return;
        };
        let permissions: Vec<Value> = extra
            .permissions
            .iter()
            .filter(|p| p["id"] != request_id)
            .cloned()
            .collect();
        self.set_permissions(agent_id, permissions);
    }

    /// Stores an agent's open permission requests and reports new ones.
    fn set_permissions(&mut self, agent_id: &str, permissions: Vec<Value>) {
        let extra = self.extras.entry(agent_id.to_string()).or_default();
        if extra.permissions == permissions {
            return;
        }
        let is_new = permissions
            .iter()
            .any(|p| !extra.permissions.iter().any(|old| old["id"] == p["id"]));
        extra.permissions = permissions;
        if is_new && self.bootstrapped {
            self.events.push(ClientEvent::PermissionRequested {
                agent_id: agent_id.to_string(),
            });
        }
        let count = extra.permissions.len();
        if let Some(agent) = self.agents.iter_mut().find(|agent| agent.id == agent_id) {
            agent.pending_permissions = count;
        }
        if let Some(sync) = self.timelines.get_mut(agent_id) {
            sync.state.pending_permissions = extra.permissions.clone();
            sync.state.version += 1;
            self.dirty_timelines.insert(agent_id.to_string());
        }
        self.directory_changed = true;
    }

    // ---- project tree ----

    /// Rebuilds the tree in O(projects + workspaces + agents) and reports a change.
    fn rebuild_projects(&mut self) {
        let mut agents_by_workspace: HashMap<&str, Vec<String>> = HashMap::new();
        for agent in &self.agents {
            if let Some(workspace_id) = &agent.workspace_id {
                agents_by_workspace
                    .entry(workspace_id)
                    .or_default()
                    .push(agent.id.clone());
            }
        }
        let mut order: Vec<ProjectInfo> = self.project_list.clone();
        for project in &self.empty_projects {
            if !order.iter().any(|listed| listed.id == project.id) {
                order.push(project.clone());
            }
        }
        let mut workspaces_by_project: HashMap<String, Vec<WorkspaceNode>> = HashMap::new();
        for workspace in &self.workspaces {
            let Some(meta) = self.workspace_meta.get(&workspace.id) else {
                continue;
            };
            // A workspace whose project is not listed yet still shows, under its own project.
            if !order.iter().any(|project| project.id == meta.project.id) {
                order.push(meta.project.clone());
            }
            workspaces_by_project
                .entry(meta.project.id.clone())
                .or_default()
                .push(WorkspaceNode {
                    id: workspace.id.clone(),
                    name: workspace.name.clone(),
                    cwd: workspace.cwd.clone(),
                    branch: meta.branch.clone(),
                    pinned: meta.pinned_at.is_some(),
                    agent_ids: agents_by_workspace
                        .remove(workspace.id.as_str())
                        .unwrap_or_default(),
                    pull_request: meta.pull_request.clone(),
                    labels: meta
                        .label_keys
                        .iter()
                        .filter_map(|key| self.label_catalog.labels.get(key).cloned())
                        .collect(),
                    status_entered_at: meta.status_entered_at,
                });
        }
        let projects: Vec<ProjectNode> = order
            .into_iter()
            .map(|project| ProjectNode {
                workspaces: workspaces_by_project
                    .remove(&project.id)
                    .unwrap_or_default(),
                id: project.id,
                name: project.name,
                root: project.root,
            })
            .collect();
        if projects != self.projects {
            self.projects = projects;
            self.directory_events.push(DirectoryEvent::ProjectsChanged);
        }
    }
}

fn label_definition(label: &Value) -> Option<WorkspaceLabel> {
    Some(WorkspaceLabel {
        name: label["name"].as_str()?.to_string(),
        color: label["color"].as_str()?.to_string(),
    })
}

/// The pull request number of a change request URL, as the app's `parsePullRequestNumber`
/// reads it: GitHub `/pull/N`, Gitea and Forgejo `/pulls/N`, GitLab `/-/merge_requests/N`.
fn pull_request_number(url: &str) -> Option<u64> {
    let after_scheme = url.split_once("://").map_or(url, |(_, rest)| rest);
    let path = after_scheme
        .split(['?', '#'])
        .next()
        .unwrap_or_default()
        .split_once('/')?
        .1;
    let segments: Vec<&str> = path.split('/').collect();
    segments.windows(2).find_map(|pair| {
        let number = pair[1];
        (matches!(pair[0], "pull" | "pulls" | "merge_requests")
            && !number.is_empty()
            && number.bytes().all(|b| b.is_ascii_digit()))
        .then(|| number.parse().ok())
        .flatten()
    })
}

/// The `githubRuntime.pullRequest` of a workspace descriptor, as `selectPrHintFromStatus`
/// reads it. `None` without a URL or a number.
fn workspace_pull_request(workspace: &Value) -> Option<WorkspacePullRequest> {
    let pull_request = &workspace["githubRuntime"]["pullRequest"];
    let url = pull_request["url"].as_str().filter(|url| !url.is_empty())?;
    let number = pull_request_number(url).or_else(|| pull_request["number"].as_u64())?;
    let is_merged = pull_request["isMerged"] == true;
    let state = match pull_request["state"].as_str() {
        _ if is_merged => PullRequestState::Merged,
        Some("merged") => PullRequestState::Merged,
        Some("open") => PullRequestState::Open,
        _ => PullRequestState::Closed,
    };
    let statuses: Vec<&str> = pull_request["checks"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|check| check["status"].as_str().unwrap_or("pending"))
        .collect();
    Some(WorkspacePullRequest {
        number,
        title: pull_request["title"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        url: url.to_string(),
        state,
        is_merged,
        is_draft: pull_request["isDraft"] == true,
        forge: workspace["forge"]
            .as_str()
            .filter(|forge| !forge.is_empty())
            .unwrap_or("github")
            .to_string(),
        checks: select_check_summary(&statuses, pull_request["checksStatus"].as_str()),
    })
}

/// A project from a project or workspace descriptor; both carry the same project fields.
fn project_info(descriptor: &Value) -> Option<ProjectInfo> {
    Some(ProjectInfo {
        id: descriptor["projectId"].as_str()?.to_string(),
        name: descriptor["projectDisplayName"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        root: descriptor["projectRootPath"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        key: descriptor["projectKey"].as_str().map(str::to_string),
    })
}

fn agent_title(id: &str, agent: &Value) -> String {
    agent["title"]
        .as_str()
        .filter(|title| !title.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| id.chars().take(8).collect())
}

fn agent_summary(id: &str, agent: &Value) -> AgentSummary {
    AgentSummary {
        id: id.to_string(),
        title: agent_title(id, agent),
        status: agent["status"].as_str().unwrap_or("unknown").to_string(),
        cwd: agent["cwd"].as_str().unwrap_or_default().to_string(),
        workspace_id: agent["workspaceId"].as_str().map(str::to_string),
        provider: agent["provider"].as_str().unwrap_or_default().to_string(),
        model: agent["model"]
            .as_str()
            .or_else(|| agent["runtimeInfo"]["model"].as_str())
            .map(str::to_string),
        pending_permissions: agent["pendingPermissions"].as_array().map_or(0, Vec::len),
    }
}

/// A `fetch_agent_history_response` payload as a page.
fn history_page(payload: &Value) -> HistoryPage {
    let text = |value: &Value| value.as_str().map(str::to_string);
    let entries = payload["entries"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            let agent = &entry["agent"];
            let id = agent["id"].as_str()?;
            let project = &entry["project"];
            Some(HistoryEntry {
                agent_id: id.to_string(),
                title: agent_title(id, agent),
                provider: agent["provider"].as_str().unwrap_or_default().to_string(),
                workspace_id: text(&agent["workspaceId"]),
                workspace_name: text(&project["workspaceName"]),
                project_name: project["projectName"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                branch: text(&project["checkout"]["currentBranch"]),
                cwd: agent["cwd"].as_str().unwrap_or_default().to_string(),
                archived: !agent["archivedAt"].is_null(),
                last_activity_at: text(&agent["updatedAt"]),
                pending_permissions: agent["pendingPermissions"].as_array().map_or(0, Vec::len),
            })
        })
        .collect();
    let page_info = &payload["pageInfo"];
    HistoryPage {
        entries,
        next_cursor: text(&page_info["nextCursor"]),
        has_more: page_info["hasMore"] == true,
        search_truncated: payload["searchTruncated"] == true,
    }
}

/// The snapshot `features`, decoded one by one through the generated union, so a feature type
/// from a newer daemon (`Unrecognized`) or a malformed feature is skipped.
fn agent_features(features: &Value) -> Vec<AgentFeature> {
    features
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|feature| UpsertAgentFeaturesItem::deserialize(feature).ok())
        .filter_map(|feature| match feature {
            UpsertAgentFeaturesItem::Toggle {
                description,
                icon,
                id,
                label,
                tooltip,
                value,
            } => Some(AgentFeature {
                id,
                label,
                description,
                tooltip,
                icon,
                kind: AgentFeatureKind::Toggle { value },
            }),
            UpsertAgentFeaturesItem::Select {
                description,
                icon,
                id,
                label,
                options,
                tooltip,
                value,
            } => Some(AgentFeature {
                id,
                label,
                description,
                tooltip,
                icon,
                kind: AgentFeatureKind::Select {
                    value,
                    options: options
                        .into_iter()
                        .map(|option| AgentFeatureOption {
                            id: option.id,
                            label: option.label,
                            description: option.description,
                            is_default: option.is_default,
                            metadata: option.metadata,
                        })
                        .collect(),
                },
            }),
            UpsertAgentFeaturesItem::Unrecognized => None,
        })
        .collect()
}

fn queued_prompts(queue: &Value) -> Vec<QueuedPrompt> {
    queue["items"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| {
            Some(QueuedPrompt {
                id: item["id"].as_str()?.to_string(),
                text: item["text"].as_str().unwrap_or_default().to_string(),
            })
        })
        .collect()
}

/// The child timeline page size, as the app's `TIMELINE_FETCH_PAGE_SIZE`.
const PROVIDER_TIMELINE_LIMIT: u64 = 40;

/// The `agent.provider_subagents.timeline.get.request` fields for a child fetch plan.
fn provider_subagent_timeline_request(
    parent_agent_id: &str,
    subagent_id: &str,
    plan: &FetchPlan,
) -> Value {
    let cursor = |epoch: &str, seq: u64| json!({"epoch": epoch, "seq": seq});
    let (direction, cursor, limit) = match plan {
        FetchPlan::Tail => ("tail", None, PROVIDER_TIMELINE_LIMIT),
        FetchPlan::After { epoch, seq } | FetchPlan::Resume { epoch, seq } => {
            ("after", Some(cursor(epoch, *seq)), PROVIDER_TIMELINE_LIMIT)
        }
        FetchPlan::Before { epoch, seq } => {
            ("before", Some(cursor(epoch, *seq)), PROVIDER_TIMELINE_LIMIT)
        }
    };
    let mut request = json!({
        "type": "agent.provider_subagents.timeline.get.request",
        "parentAgentId": parent_agent_id,
        "subagentId": subagent_id,
        "direction": direction,
        "limit": limit,
    });
    if let Some(cursor) = cursor {
        request["cursor"] = cursor;
    }
    request
}

/// Maps a child timeline response into the page `TimelineSync` reads. The rows lack
/// `provider` and `collapsed`, and carry `seq` with optional start and end.
fn provider_subagent_page(payload: &Value, provider: &str) -> Value {
    let mut entries: Vec<Value> = payload["rows"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|row| {
            let seq = row["seq"].as_u64()?;
            let start = row["seqStart"].as_u64().unwrap_or(seq);
            let end = row["seqEnd"].as_u64().unwrap_or(seq);
            let ranges = row
                .get("sourceSeqRanges")
                .cloned()
                .unwrap_or_else(|| json!([{"startSeq": seq, "endSeq": seq}]));
            Some(json!({
                "provider": provider,
                "item": row["item"],
                "timestamp": row["timestamp"],
                "seqStart": start,
                "seqEnd": end,
                "sourceSeqRanges": ranges,
                "collapsed": [],
            }))
        })
        .collect();
    entries.sort_by_key(|entry| entry["seqStart"].as_u64().unwrap_or(0));
    let first = entries.first().and_then(|entry| entry["seqStart"].as_u64());
    let last = entries
        .iter()
        .filter_map(|entry| entry["seqEnd"].as_u64())
        .max();
    let cursor = |key: &str, seq: Option<u64>| {
        if payload[key].is_null() {
            seq.map(|seq| json!({"seq": seq})).unwrap_or(Value::Null)
        } else {
            payload[key].clone()
        }
    };
    let window = if payload["window"].is_object() {
        payload["window"].clone()
    } else {
        json!({"maxSeq": last.unwrap_or(0)})
    };
    json!({
        "epoch": payload["epoch"],
        "reset": payload["reset"],
        "entries": entries,
        "startCursor": cursor("startCursor", first),
        "endCursor": cursor("endCursor", last),
        "window": window,
        "hasOlder": payload["hasOlder"],
        "hasNewer": payload["hasNewer"],
        "error": payload["error"],
    })
}

/// The outcome of a project config response: `ok` or an error code such as
/// `stale_project_config`.
fn config_result(rpc_error: &Option<String>, payload: &Value) -> anyhow::Result<()> {
    if let Some(error) = rpc_error {
        anyhow::bail!("{error}");
    }
    if payload["ok"] == true {
        return Ok(());
    }
    let code = payload["error"]["code"]
        .as_str()
        .unwrap_or("project config request failed");
    anyhow::bail!("{code}")
}
