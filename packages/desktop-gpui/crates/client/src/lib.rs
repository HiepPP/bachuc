//! `bachuc-client`: a GPUI entity that connects to one daemon host, keeps agents, workspaces,
//! providers, and the viewed timelines in sync, and sends the requests the UI needs.
//!
//! The socket runs on its own thread (`connection`). Its events reach the entity in batches;
//! timeline changes are announced at most every 50 ms, terminal frames go straight to each
//! terminal's channel.

mod composer_data;
pub mod connection;
pub mod core;
mod files;
mod hosts;
mod model;
mod pending;
mod plugins;
mod terminal;
pub mod timeline;

pub use connection::{ConnectionConfig, WorkerEvent};
pub use core::ClientCore;
pub use hosts::{HostConfig, client_id, data_dir, load_hosts, save_hosts};
pub use model::{
    ActiveTurn, AddedProject, AgentUsage, BranchSuggestion, BranchValidation, CreateWorkspace,
    CreatedWorkspace, DiffCompare, DiffMode, DraftConfig, InitialAgent, OutgoingMessage,
    SubagentKind, SubagentRow, TaskEntry, TaskStatus, TurnMetrics, WorkspaceSource, WorktreeAction,
    resume_command,
};
pub use model::{AgentControls, AgentSummary, ClientEvent, ProviderSummary, WorkspaceSummary};
pub use model::{
    AgentFeature, AgentFeatureKind, AgentFeatureOption, AttentionReason, ChangedFileDetails,
    CheckoutDetails, CommitFile, CommitList, CommitSummary, DiffStat, FileDiff, FileDiffStatus,
};
pub use model::{
    BackgroundProcess, ChangedFile, CheckoutStatus, DirectoryEvent, FileContent, FileEntry,
    FileKind, ProjectNode, QueuedPrompt, SlashCommand, ThinkingControls, TurnBehavior,
    WorkspaceNode,
};
pub use model::{
    CheckSummary, CheckSummaryState, PullRequestState, WorkspaceLabel, WorkspacePullRequest,
    select_check_summary,
};
pub use model::{DirectorySuggestion, MatchMode, NewEntryKind};
pub use model::{ForkBoundary, ForkContext, ForkCursor, HistoryEntry, HistoryPage};
pub use model::{
    GitAction, GitActionReply, GitStatus, GitStatusEvent, MergeMethod, PrCheck, PrMergeCapability,
    PrStatus, PrStatusReply, PrTimelineItem, StashEntry, WorkspaceScript,
};
pub use model::{PromptEntry, PromptIndex};
pub use model::{RewindMode, RewindModes};
pub use pending::{PENDING_TURN_TTL_MS, PendingTurnsChanged};
pub use plugins::{
    PluginEvent, PluginListItem, PluginSettingsRead, PluginSettingsWrite, PluginStatus,
};
pub use terminal::{TerminalFrame, TerminalHandle};
pub use timeline::TimelineState;

use futures::StreamExt as _;
use gpui_kit::{App, AppContext as _, Context, Entity, EventEmitter, Task};
use std::collections::BTreeMap;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Timeline changes reach the UI at most this often.
const FLUSH_INTERVAL: Duration = Duration::from_millis(50);

/// One daemon host connection as a GPUI entity. It emits `ClientEvent`.
pub struct Client {
    host: HostConfig,
    core: ClientCore,
    last_flush: Option<Instant>,
    flush_task: Option<Task<()>>,
    pending_turns: pending::PendingTurns,
    _reader: Task<()>,
}

impl EventEmitter<ClientEvent> for Client {}
impl EventEmitter<DirectoryEvent> for Client {}
impl EventEmitter<PluginEvent> for Client {}
impl EventEmitter<GitStatusEvent> for Client {}
impl EventEmitter<PendingTurnsChanged> for Client {}

/// The timeline key of a provider subagent, built as the app's stream ID: `provider:`, then
/// both ids percent-encoded as `encodeURIComponent` does, so ids with `:` cannot collide.
pub fn provider_subagent_key(parent_agent_id: &str, subagent_id: &str) -> String {
    format!(
        "provider:{}:{}",
        encode_component(parent_agent_id),
        encode_component(subagent_id)
    )
}

/// Splits a `provider_subagent_key` back into its parent and child ids. A plain agent id
/// gives `None`.
pub fn parse_provider_subagent_key(key: &str) -> Option<(String, String)> {
    let rest = key.strip_prefix("provider:")?;
    let (parent, child) = rest.split_once(':')?;
    Some((decode_component(parent)?, decode_component(child)?))
}

/// The characters `encodeURIComponent` leaves as they are.
fn encode_component(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'_'
            | b'.'
            | b'!'
            | b'~'
            | b'*'
            | b'\''
            | b'('
            | b')' => out.push(byte as char),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn decode_component(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let hex = std::str::from_utf8(bytes.get(index + 1..index + 3)?).ok()?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            index += 3;
        } else {
            out.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// Milliseconds now from the wall clock, as `Date.now()`.
fn unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_millis() as i64)
}

/// Waits for a core reply on the background executor.
fn reply_task<T: Send + 'static>(
    receiver: futures::channel::oneshot::Receiver<anyhow::Result<T>>,
    cx: &mut Context<Client>,
) -> Task<anyhow::Result<T>> {
    cx.background_spawn(async move {
        receiver
            .await
            .unwrap_or_else(|_| Err(anyhow::anyhow!("client closed")))
    })
}

impl Client {
    /// Connects to `host` and keeps reconnecting until the entity is dropped.
    pub fn connect(host: HostConfig, cx: &mut App) -> Entity<Client> {
        let config = ConnectionConfig::new(host.url.clone(), client_id(), host.password.clone());
        Self::connect_with(host, config, cx)
    }

    /// `connect` with explicit connection settings.
    pub fn connect_with(
        host: HostConfig,
        config: ConnectionConfig,
        cx: &mut App,
    ) -> Entity<Client> {
        cx.new(|cx| {
            let (sender, mut receiver) = futures::channel::mpsc::unbounded();
            let core = ClientCore::new(config, move |event| sender.unbounded_send(event).is_ok());
            let reader = cx.spawn(async move |this, cx| {
                while let Some(event) = receiver.next().await {
                    let mut batch = vec![event];
                    while let Ok(event) = receiver.try_recv() {
                        batch.push(event);
                    }
                    let applied = this.update(cx, |client: &mut Client, cx| {
                        for event in batch {
                            client.core.handle(event);
                        }
                        client.publish(cx);
                    });
                    if applied.is_err() {
                        break;
                    }
                }
            });
            Client {
                host,
                core,
                last_flush: None,
                flush_task: None,
                pending_turns: pending::PendingTurns::default(),
                _reader: reader,
            }
        })
    }

    /// Emits pending events now and schedules the timeline flush.
    fn publish(&mut self, cx: &mut Context<Self>) {
        let events = self.core.take_events();
        let directory_events = self.core.take_directory_events();
        let plugin_events = self.core.take_plugin_events();
        let git_status_events = self.core.take_git_status_events();
        let changed = !events.is_empty() || !directory_events.is_empty();
        for event in events {
            cx.emit(event);
        }
        for event in directory_events {
            cx.emit(event);
        }
        for event in plugin_events {
            cx.emit(event);
        }
        for event in git_status_events {
            cx.emit(event);
        }
        if self.core.has_dirty_timelines() && self.flush_task.is_none() {
            let wait = self
                .last_flush
                .map(|last| FLUSH_INTERVAL.saturating_sub(last.elapsed()))
                .unwrap_or_default();
            if wait.is_zero() {
                self.flush(cx);
            } else {
                self.flush_task = Some(cx.spawn(async move |this, cx| {
                    cx.background_executor().timer(wait).await;
                    let _ = this.update(cx, |client: &mut Client, cx| {
                        client.flush_task = None;
                        client.flush(cx);
                    });
                }));
            }
        }
        if changed {
            cx.notify();
        }
    }

    fn flush(&mut self, cx: &mut Context<Self>) {
        self.last_flush = Some(Instant::now());
        for agent_id in self.core.take_dirty_timelines() {
            cx.emit(ClientEvent::TimelineChanged { agent_id });
        }
        cx.notify();
    }

    /// Marks `agent_id` as having a prompt the daemon has not confirmed. The mark lives only in
    /// this client and hides after `PENDING_TURN_TTL_MS`.
    pub fn mark_turn_pending(&mut self, agent_id: &str, cx: &mut Context<Self>) {
        self.pending_turns.mark(agent_id, unix_ms());
        cx.emit(PendingTurnsChanged);
    }

    /// Drops the mark of `agent_id`. Emits only when a mark was there.
    pub fn clear_turn_pending(&mut self, agent_id: &str, cx: &mut Context<Self>) {
        if self.pending_turns.clear(agent_id) {
            cx.emit(PendingTurnsChanged);
        }
    }

    /// The mark time of `agent_id` in UNIX milliseconds, until the limit passes.
    pub fn pending_turn(&self, agent_id: &str) -> Option<i64> {
        self.pending_turns.get(agent_id, unix_ms())
    }

    pub fn host(&self) -> &HostConfig {
        &self.host
    }

    /// True between `Connected` and `Disconnected`.
    pub fn is_connected(&self) -> bool {
        self.core.is_connected()
    }

    pub fn agents(&self) -> &[AgentSummary] {
        self.core.agents()
    }

    pub fn workspaces(&self) -> &[WorkspaceSummary] {
        self.core.workspaces()
    }

    pub fn providers(&self) -> &[ProviderSummary] {
        self.core.providers()
    }

    /// Starts syncing an agent's timeline and streaming its live rows.
    pub fn view_agent(&mut self, agent_id: &str, cx: &mut Context<Client>) {
        self.core.view_agent(agent_id);
        self.publish(cx);
    }

    /// Stops syncing an agent's timeline and drops its state.
    pub fn unview_agent(&mut self, agent_id: &str, cx: &mut Context<Client>) {
        self.core.unview_agent(agent_id);
        self.publish(cx);
    }

    pub fn timeline(&self, agent_id: &str) -> Option<&TimelineState> {
        self.core.timeline(agent_id)
    }

    /// The epoch of the agent's current timeline window; `None` before its first page.
    pub fn timeline_epoch(&self, agent_id: &str) -> Option<&str> {
        self.core.timeline_epoch(agent_id)
    }

    /// Fetches the page before the first entry when `has_older` is set.
    pub fn load_older(&mut self, agent_id: &str, cx: &mut Context<Client>) {
        self.core.load_older(agent_id);
        self.publish(cx);
    }

    /// Whether the host streams provider subagent timelines
    /// (`features.projectedSubagentTimeline`).
    pub fn supports_provider_subagent_timeline(&self) -> bool {
        self.core.supports_provider_subagent_timeline()
    }

    /// Sends a prompt. `images` are `{data, mimeType}` objects with base64 data.
    pub fn send_prompt(
        &mut self,
        agent_id: &str,
        text: String,
        images: Vec<serde_json::Value>,
        cx: &mut Context<Client>,
    ) {
        self.core.send_prompt(agent_id, text, images);
        self.publish(cx);
    }

    pub fn cancel_agent(&mut self, agent_id: &str, cx: &mut Context<Client>) {
        self.core.cancel_agent(agent_id);
        self.publish(cx);
    }

    pub fn set_mode(&mut self, agent_id: &str, mode_id: &str, cx: &mut Context<Client>) {
        self.core.set_mode(agent_id, mode_id);
        self.publish(cx);
    }

    pub fn set_model(&mut self, agent_id: &str, model_id: &str, cx: &mut Context<Client>) {
        self.core.set_model(agent_id, model_id);
        self.publish(cx);
    }

    pub fn agent_controls(&self, agent_id: &str) -> Option<AgentControls> {
        self.core.agent_controls(agent_id)
    }

    pub fn respond_permission(
        &mut self,
        agent_id: &str,
        request_id: &str,
        allow: bool,
        cx: &mut Context<Client>,
    ) {
        self.core.respond_permission(agent_id, request_id, allow);
        self.publish(cx);
    }

    /// Creates an agent and resolves to its id.
    pub fn create_agent(
        &mut self,
        provider: &str,
        model: Option<&str>,
        cwd: &str,
        prompt: &str,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<String>> {
        let receiver = self.core.create_agent(provider, model, cwd, prompt);
        self.publish(cx);
        cx.background_spawn(async move {
            receiver
                .await
                .unwrap_or_else(|_| Err(anyhow::anyhow!("client closed")))
        })
    }

    /// Creates a terminal in `cwd` and subscribes to it.
    pub fn create_terminal(
        &mut self,
        cwd: &str,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<TerminalHandle>> {
        let receiver = self.core.create_terminal(cwd);
        self.publish(cx);
        cx.background_spawn(async move {
            receiver
                .await
                .unwrap_or_else(|_| Err(anyhow::anyhow!("client closed")))
        })
    }

    /// Subscribes to an existing terminal. Its first frame is a `Snapshot`.
    pub fn attach_terminal(
        &mut self,
        terminal_id: &str,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<TerminalHandle>> {
        let receiver = self.core.attach_terminal(terminal_id);
        self.publish(cx);
        cx.background_spawn(async move {
            receiver
                .await
                .unwrap_or_else(|_| Err(anyhow::anyhow!("client closed")))
        })
    }

    pub fn terminal_input(&mut self, terminal_id: &str, bytes: &[u8]) {
        self.core.terminal_input(terminal_id, bytes);
    }

    /// Resizes the PTY. `claim` takes size ownership; later changes pass false.
    pub fn terminal_resize(&mut self, terminal_id: &str, rows: u16, cols: u16, claim: bool) {
        self.core.terminal_resize(terminal_id, rows, cols, claim);
    }

    pub fn kill_terminal(&mut self, terminal_id: &str, cx: &mut Context<Client>) {
        self.core.kill_terminal(terminal_id);
        self.publish(cx);
    }

    // ---- parity requests (TASK-146) ----

    /// The typed `server_info` of the current connection.
    pub fn server_info(&self) -> Option<&bachuc_protocol::ServerInfo> {
        self.core.typed_server_info()
    }

    /// Projects with their workspaces. `DirectoryEvent::ProjectsChanged` marks a change.
    pub fn projects(&self) -> &[ProjectNode] {
        self.core.projects()
    }

    /// Sets the workspace title; an empty title reverts to the derived name.
    pub fn rename_workspace(&mut self, workspace_id: &str, title: &str, cx: &mut Context<Client>) {
        self.core.rename_workspace(workspace_id, title);
        self.publish(cx);
    }

    pub fn pin_workspace(&mut self, workspace_id: &str, pinned: bool, cx: &mut Context<Client>) {
        self.core.pin_workspace(workspace_id, pinned);
        self.publish(cx);
    }

    pub fn archive_workspace(&mut self, workspace_id: &str, cx: &mut Context<Client>) {
        self.core.archive_workspace(workspace_id);
        self.publish(cx);
    }

    /// Removes a project and its workspaces from the host's sidebar; files on disk stay. A
    /// refused or failed removal sends [`ClientEvent::ProjectRemoveFailed`].
    pub fn remove_project(&mut self, project_id: &str, cx: &mut Context<Client>) {
        self.core.remove_project(project_id);
        self.publish(cx);
    }

    pub fn mark_workspace_unread(&mut self, workspace_id: &str, cx: &mut Context<Client>) {
        self.core.mark_workspace_unread(workspace_id);
        self.publish(cx);
    }

    /// Marks the agent's finished or failed turn read on the host.
    pub fn clear_agent_attention(&mut self, agent_id: &str, cx: &mut Context<Client>) {
        self.core.clear_agent_attention(agent_id);
        self.publish(cx);
    }

    pub fn archive_agent(&mut self, agent_id: &str, cx: &mut Context<Client>) {
        self.core.archive_agent(agent_id);
        self.publish(cx);
    }

    pub fn rename_agent(&mut self, agent_id: &str, title: &str, cx: &mut Context<Client>) {
        self.core.rename_agent(agent_id, title);
        self.publish(cx);
    }

    /// Lists a folder; `path` is relative to `cwd` (`.` for `cwd` itself).
    pub fn list_directory(
        &mut self,
        cwd: &str,
        path: &str,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<Vec<FileEntry>>> {
        let receiver = self.core.list_directory(cwd, path);
        self.publish(cx);
        reply_task(receiver, cx)
    }

    pub fn read_file(
        &mut self,
        cwd: &str,
        path: &str,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<FileContent>> {
        let receiver = self.core.read_file(cwd, path);
        self.publish(cx);
        reply_task(receiver, cx)
    }

    /// The branch and the uncommitted changed files of `cwd`. A folder outside git gives no
    /// branch and no files.
    pub fn checkout_status(
        &mut self,
        cwd: &str,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<CheckoutStatus>> {
        let receiver = self.core.checkout_status(cwd);
        self.publish(cx);
        reply_task(receiver, cx)
    }

    /// Emits `DirectoryEvent::CheckoutChanged` for `cwd` until `unwatch_checkout`.
    pub fn watch_checkout(&mut self, cwd: &str, cx: &mut Context<Client>) {
        self.core.watch_checkout(cwd);
        self.publish(cx);
    }

    pub fn unwatch_checkout(&mut self, cwd: &str, cx: &mut Context<Client>) {
        self.core.unwatch_checkout(cwd);
        self.publish(cx);
    }

    /// The unified diff of one uncommitted file. A file with status `too_large` or `binary`
    /// fails; a file without changes gives an empty string.
    pub fn file_diff(
        &mut self,
        cwd: &str,
        path: &str,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<String>> {
        let receiver = self.core.file_diff(cwd, path);
        self.publish(cx);
        reply_task(receiver, cx)
    }

    pub fn agent_thinking(&self, agent_id: &str) -> Option<ThinkingControls> {
        self.core.agent_thinking(agent_id)
    }

    pub fn set_thinking(&mut self, agent_id: &str, option_id: &str, cx: &mut Context<Client>) {
        self.core.set_thinking(agent_id, option_id);
        self.publish(cx);
    }

    pub fn list_commands(
        &mut self,
        agent_id: &str,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<Vec<SlashCommand>>> {
        let receiver = self.core.list_commands(agent_id);
        self.publish(cx);
        reply_task(receiver, cx)
    }

    /// The commands and skills of a draft: the daemon reads the project skills from its folder.
    pub fn list_draft_commands(
        &mut self,
        agent_id: &str,
        config: &DraftConfig,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<Vec<SlashCommand>>> {
        let receiver = self.core.list_draft_commands(agent_id, config);
        self.publish(cx);
        reply_task(receiver, cx)
    }

    /// Sends a prompt; `behavior` says what it does to a running turn. The app sends `Steer`.
    pub fn send_prompt_with(
        &mut self,
        agent_id: &str,
        text: String,
        images: Vec<serde_json::Value>,
        behavior: TurnBehavior,
        cx: &mut Context<Client>,
    ) {
        self.core.send_prompt_with(agent_id, text, images, behavior);
        self.publish(cx);
    }

    /// Whether the host queues prompts (`features.agentMessageQueue`).
    pub fn supports_queue(&self) -> bool {
        self.core.supports_queue()
    }

    /// Queues a prompt behind the running turn. `AgentsChanged` follows the queue change.
    pub fn enqueue_prompt(
        &mut self,
        agent_id: &str,
        text: String,
        images: Vec<serde_json::Value>,
        cx: &mut Context<Client>,
    ) {
        self.core.enqueue_prompt(agent_id, text, images);
        self.publish(cx);
    }

    pub fn queued_prompts(&self, agent_id: &str) -> Vec<QueuedPrompt> {
        self.core.queued_prompts(agent_id)
    }

    /// The agent's background processes, in the daemon's order.
    pub fn background_processes(&self, agent_id: &str) -> Vec<BackgroundProcess> {
        self.core.background_processes(agent_id)
    }

    /// Whether the host can stop a background process.
    pub fn supports_background_stop(&self) -> bool {
        self.core.supports_background_stop()
    }

    /// Stops one background process. A refused stop logs a warning.
    pub fn stop_background_process(
        &mut self,
        agent_id: &str,
        process_id: &str,
        cx: &mut Context<Client>,
    ) {
        self.core.stop_background_process(agent_id, process_id);
        self.publish(cx);
    }

    pub fn cancel_queued(&mut self, agent_id: &str, queued_id: &str, cx: &mut Context<Client>) {
        self.core.cancel_queued(agent_id, queued_id);
        self.publish(cx);
    }

    /// Answers a permission with a full `AgentPermissionResponse` object.
    pub fn respond_permission_with(
        &mut self,
        agent_id: &str,
        request_id: &str,
        response: serde_json::Value,
        cx: &mut Context<Client>,
    ) {
        self.core
            .respond_permission_with(agent_id, request_id, response);
        self.publish(cx);
    }

    /// The project's `paseo.json`, null when it has none.
    pub fn read_project_config(
        &mut self,
        root: &str,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<serde_json::Value>> {
        let receiver = self.core.read_project_config(root);
        self.publish(cx);
        reply_task(receiver, cx)
    }

    /// Writes the project's `paseo.json`. Read it first: the write fails with
    /// `stale_project_config` when the file changed since this client last read or wrote it.
    pub fn write_project_config(
        &mut self,
        root: &str,
        config: serde_json::Value,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<()>> {
        let receiver = self.core.write_project_config(root, config);
        self.publish(cx);
        reply_task(receiver, cx)
    }

    // ---- follow-up fields and requests (TASK-157) ----

    /// Whether the agent asks for attention, and why. `None` for an unknown agent.
    pub fn agent_attention(&self, agent_id: &str) -> Option<(bool, Option<AttentionReason>)> {
        self.core.agent_attention(agent_id)
    }

    /// The project's `projectKey`, else `["<serverId>","<projectId>"]` as the app builds it.
    pub fn project_view_key(&self, project_id: &str) -> Option<String> {
        self.core.project_view_key(project_id)
    }

    pub fn workspace_diff_stat(&self, workspace_id: &str) -> Option<DiffStat> {
        self.core.workspace_diff_stat(workspace_id)
    }

    /// The workspace `gitRuntime.isDirty`; `None` when absent, null, or the workspace is unknown.
    pub fn workspace_is_dirty(&self, workspace_id: &str) -> Option<bool> {
        self.core.workspace_is_dirty(workspace_id)
    }

    pub fn workspace_pinned_at(&self, workspace_id: &str) -> Option<String> {
        self.core.workspace_pinned_at(workspace_id)
    }

    /// The agent's provider features; a feature type this client does not know is left out.
    pub fn agent_features(&self, agent_id: &str) -> Vec<AgentFeature> {
        self.core.agent_features(agent_id)
    }

    /// Sets a provider feature; `value` is a bool for a toggle and an option id for a select.
    pub fn set_agent_feature(
        &mut self,
        agent_id: &str,
        feature_id: &str,
        value: serde_json::Value,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<()>> {
        let receiver = self.core.set_agent_feature(agent_id, feature_id, value);
        self.publish(cx);
        reply_task(receiver, cx)
    }

    /// `checkout_status` with the git flag and the full file fields.
    pub fn checkout_details(
        &mut self,
        cwd: &str,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<CheckoutDetails>> {
        let receiver = self.core.checkout_details(cwd);
        self.publish(cx);
        reply_task(receiver, cx)
    }

    /// Every uncommitted file of `cwd` with its diff, from one `checkout.diff.get` request.
    pub fn checkout_diff(
        &mut self,
        cwd: &str,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<Vec<FileDiff>>> {
        let receiver = self.core.checkout_diff(cwd);
        self.publish(cx);
        reply_task(receiver, cx)
    }

    pub fn commits(
        &mut self,
        cwd: &str,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<CommitList>> {
        let receiver = self.core.commits(cwd);
        self.publish(cx);
        reply_task(receiver, cx)
    }

    /// The diff of one file in one commit; `None` when the commit has no text diff for it.
    pub fn commit_file_diff(
        &mut self,
        cwd: &str,
        sha: &str,
        path: &str,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<Option<FileDiff>>> {
        let receiver = self.core.commit_file_diff(cwd, sha, path);
        self.publish(cx);
        reply_task(receiver, cx)
    }

    // ---- plugins (TASK-168) ----

    /// The installed plugins in the daemon's order. `PluginEvent::CatalogChanged` marks a change.
    pub fn list_plugins(
        &mut self,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<Vec<PluginListItem>>> {
        let receiver = self.core.list_plugins();
        self.publish(cx);
        reply_task(receiver, cx)
    }

    /// The ids of the loaded plugins.
    pub fn plugin_catalog_ids(
        &mut self,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<Vec<String>>> {
        let receiver = self.core.plugin_catalog_ids();
        self.publish(cx);
        reply_task(receiver, cx)
    }

    /// Calls a plugin RPC and resolves to its `output`.
    pub fn invoke_plugin_rpc(
        &mut self,
        plugin_id: &str,
        method: &str,
        input: serde_json::Value,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<serde_json::Value>> {
        let receiver = self.core.plugin_rpc(plugin_id, method, input);
        self.publish(cx);
        reply_task(receiver, cx)
    }

    /// Calls a plugin RPC with a typed input and output. An output that does not fit `O` fails.
    pub fn plugin_rpc<I: serde::Serialize, O: serde::de::DeserializeOwned + Send + 'static>(
        &mut self,
        plugin_id: &str,
        method: &str,
        input: &I,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<O>> {
        let receiver = match serde_json::to_value(input) {
            Ok(input) => self.core.plugin_rpc(plugin_id, method, input),
            Err(error) => {
                let (reply, receiver) = futures::channel::oneshot::channel();
                let _ = reply.send(Err(anyhow::anyhow!("{method} input: {error}")));
                receiver
            }
        };
        self.publish(cx);
        reply_task(receiver, cx)
    }

    pub fn read_plugin_settings(
        &mut self,
        plugin_id: &str,
        settings_id: &str,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<PluginSettingsRead>> {
        let receiver = self.core.read_plugin_settings(plugin_id, settings_id);
        self.publish(cx);
        reply_task(receiver, cx)
    }

    pub fn write_plugin_settings(
        &mut self,
        plugin_id: &str,
        settings_id: &str,
        revision: &str,
        values: serde_json::Value,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<PluginSettingsWrite>> {
        let receiver = self
            .core
            .write_plugin_settings(plugin_id, settings_id, revision, values);
        self.publish(cx);
        reply_task(receiver, cx)
    }

    pub fn reset_plugin_settings(
        &mut self,
        plugin_id: &str,
        settings_id: &str,
        revision: &str,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<PluginSettingsWrite>> {
        let receiver = self
            .core
            .reset_plugin_settings(plugin_id, settings_id, revision);
        self.publish(cx);
        reply_task(receiver, cx)
    }

    /// The agent's labels; `None` for an unknown agent.
    pub fn agent_labels(&self, agent_id: &str) -> Option<BTreeMap<String, String>> {
        self.core.agent_labels(agent_id)
    }

    /// Sets these labels; the agent's other labels stay.
    pub fn set_agent_labels(
        &mut self,
        agent_id: &str,
        labels: BTreeMap<String, String>,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<()>> {
        let receiver = self.core.set_agent_labels(agent_id, labels);
        self.publish(cx);
        reply_task(receiver, cx)
    }

    /// Closes the agent's provider runtime.
    pub fn close_agent_runtime(
        &mut self,
        agent_id: &str,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<()>> {
        let receiver = self.core.close_agent_runtime(agent_id);
        self.publish(cx);
        reply_task(receiver, cx)
    }

    // ---- composer, New workspace, and tab menu data (TASK-169) ----

    /// The running turn from the agent snapshot.
    pub fn agent_active_turn(&self, agent_id: &str) -> Option<ActiveTurn> {
        self.core.agent_active_turn(agent_id)
    }

    /// The metrics of the viewed agent's last turn; `None` when the newest turn has none.
    pub fn agent_last_turn_metrics(&self, agent_id: &str) -> Option<TurnMetrics> {
        self.core.agent_last_turn_metrics(agent_id)
    }

    pub fn agent_last_usage(&self, agent_id: &str) -> Option<AgentUsage> {
        self.core.agent_last_usage(agent_id)
    }

    /// The newest task list of a viewed agent; empty for an agent that is not viewed.
    pub fn agent_tasks(&self, agent_id: &str) -> Vec<TaskEntry> {
        self.core.agent_tasks(agent_id)
    }

    /// The track strip rows of a parent agent. A change emits `ClientEvent::AgentsChanged`.
    pub fn subagents(&self, parent_agent_id: &str) -> Vec<SubagentRow> {
        self.core.subagents(parent_agent_id)
    }

    pub fn agent_parent_id(&self, agent_id: &str) -> Option<String> {
        self.core.agent_parent_id(agent_id)
    }

    /// Sends a prompt with images and attachments; a refused send fails with its error.
    pub fn send_message(
        &mut self,
        agent_id: &str,
        message: OutgoingMessage,
        behavior: TurnBehavior,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<()>> {
        let receiver = self.core.send_outgoing(agent_id, message, behavior);
        self.publish(cx);
        reply_task(receiver, cx)
    }

    /// Queues a prompt with images and attachments behind the running turn.
    pub fn enqueue_message(
        &mut self,
        agent_id: &str,
        message: OutgoingMessage,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<()>> {
        let receiver = self.core.enqueue_outgoing(agent_id, message);
        self.publish(cx);
        reply_task(receiver, cx)
    }

    pub fn queue_held(&self, agent_id: &str) -> bool {
        self.core.queue_held(agent_id)
    }

    pub fn edit_queued(
        &mut self,
        agent_id: &str,
        queued_id: &str,
        text: &str,
        cx: &mut Context<Client>,
    ) {
        self.core.edit_queued(agent_id, queued_id, text);
        self.publish(cx);
    }

    /// Moves a queued prompt before `before_id`; `None` moves it to the end.
    pub fn reorder_queued(
        &mut self,
        agent_id: &str,
        queued_id: &str,
        before_id: Option<&str>,
        cx: &mut Context<Client>,
    ) {
        self.core.reorder_queued(agent_id, queued_id, before_id);
        self.publish(cx);
    }

    pub fn promote_queued(&mut self, agent_id: &str, queued_id: &str, cx: &mut Context<Client>) {
        self.core.promote_queued(agent_id, queued_id);
        self.publish(cx);
    }

    pub fn resume_queue(&mut self, agent_id: &str, cx: &mut Context<Client>) {
        self.core.resume_queue(agent_id);
        self.publish(cx);
    }

    pub fn provider_modes(
        &mut self,
        provider: &str,
        cwd: Option<&str>,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<Vec<(String, String)>>> {
        let receiver = self.core.provider_modes(provider, cwd);
        self.publish(cx);
        reply_task(receiver, cx)
    }

    pub fn draft_features(
        &mut self,
        config: DraftConfig,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<Vec<AgentFeature>>> {
        let receiver = self.core.draft_features(config);
        self.publish(cx);
        reply_task(receiver, cx)
    }

    pub fn default_model(&self, provider: &str) -> Option<String> {
        self.core.default_model(provider)
    }

    pub fn model_thinking(&self, provider: &str, model: &str) -> Option<ThinkingControls> {
        self.core.model_thinking(provider, model)
    }

    pub fn create_workspace(
        &mut self,
        request: CreateWorkspace,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<CreatedWorkspace>> {
        let receiver = self.core.create_workspace(request);
        self.publish(cx);
        reply_task(receiver, cx)
    }

    /// Creates an agent with its full config and resolves to its id.
    pub fn create_agent_with(
        &mut self,
        agent: InitialAgent,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<String>> {
        let receiver = self.core.create_agent_with(agent);
        self.publish(cx);
        reply_task(receiver, cx)
    }

    pub fn branch_suggestions(
        &mut self,
        cwd: &str,
        query: Option<&str>,
        limit: Option<u32>,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<Vec<BranchSuggestion>>> {
        let receiver = self.core.branch_suggestions(cwd, query, limit);
        self.publish(cx);
        reply_task(receiver, cx)
    }

    pub fn validate_branch(
        &mut self,
        cwd: &str,
        branch_name: &str,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<BranchValidation>> {
        let receiver = self.core.validate_branch(cwd, branch_name);
        self.publish(cx);
        reply_task(receiver, cx)
    }

    pub fn add_project(
        &mut self,
        cwd: &str,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<AddedProject>> {
        let receiver = self.core.add_project(cwd);
        self.publish(cx);
        reply_task(receiver, cx)
    }

    pub fn refresh_agent(
        &mut self,
        agent_id: &str,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<()>> {
        let receiver = self.core.refresh_agent(agent_id);
        self.publish(cx);
        reply_task(receiver, cx)
    }

    /// Unarchives the agent; the daemon also restores its workspace.
    pub fn unarchive_agent(
        &mut self,
        agent_id: &str,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<()>> {
        let receiver = self.core.unarchive_agent(agent_id);
        self.publish(cx);
        reply_task(receiver, cx)
    }

    /// Fetches one agent; an archived agent then shows in `archived_agent`.
    pub fn fetch_agent(
        &mut self,
        agent_id: &str,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<()>> {
        let receiver = self.core.fetch_agent(agent_id);
        self.publish(cx);
        reply_task(receiver, cx)
    }

    /// An archived agent this client has seen; archived agents are never in `agents`.
    pub fn archived_agent(&self, agent_id: &str) -> Option<&AgentSummary> {
        self.core.archived_agent(agent_id)
    }

    pub fn is_archived(&self, agent_id: &str) -> bool {
        self.core.is_archived(agent_id)
    }

    pub fn history_search_supported(&self) -> bool {
        self.core.history_search_supported()
    }

    /// One page of the agent history, archived agents included.
    pub fn fetch_agent_history(
        &mut self,
        search: Option<&str>,
        cursor: Option<&str>,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<HistoryPage>> {
        let receiver = self.core.fetch_agent_history(search, cursor);
        self.publish(cx);
        reply_task(receiver, cx)
    }

    /// Whether the host lists the prompts of a timeline (`features.agentTimelinePromptIndex`).
    pub fn prompt_index_supported(&self) -> bool {
        self.core.prompt_index_supported()
    }

    /// Every user prompt of the agent's timeline, loaded or not.
    pub fn list_prompts(
        &mut self,
        agent_id: &str,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<PromptIndex>> {
        let receiver = self.core.list_prompts(agent_id);
        self.publish(cx);
        reply_task(receiver, cx)
    }

    pub fn fork_supported(&self) -> bool {
        self.core.fork_supported()
    }

    pub fn fork_cursor_supported(&self) -> bool {
        self.core.fork_cursor_supported()
    }

    /// The chat history to fork. A host without `agentForkContextCursor` gets no cursor.
    pub fn fork_context(
        &mut self,
        agent_id: &str,
        boundary: ForkBoundary,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<ForkContext>> {
        let receiver = self.core.fork_context(agent_id, boundary);
        self.publish(cx);
        reply_task(receiver, cx)
    }

    /// The rewind modes of the agent. All false when the host lacks the `rewind` feature.
    pub fn rewind_modes(&self, agent_id: &str) -> RewindModes {
        self.core.rewind_modes(agent_id)
    }

    /// Rewinds the agent to a user message. Resolves on the host's answer; `Err` holds its
    /// error text. A rewind of the conversation reloads the timeline tail.
    pub fn rewind(
        &mut self,
        agent_id: &str,
        message_id: &str,
        mode: RewindMode,
        cx: &mut Context<Client>,
    ) -> Task<Result<(), String>> {
        let receiver = self.core.rewind(agent_id, message_id, mode);
        self.publish(cx);
        cx.background_spawn(async move {
            receiver
                .await
                .unwrap_or_else(|_| Err(anyhow::anyhow!("client closed")))
                .map_err(|error| error.to_string())
        })
    }

    pub fn agent_provider_session_id(&self, agent_id: &str) -> Option<String> {
        self.core.agent_provider_session_id(agent_id)
    }

    pub fn agent_updated_at(&self, agent_id: &str) -> Option<String> {
        self.core.agent_updated_at(agent_id)
    }

    pub fn agent_thinking_option_id(&self, agent_id: &str) -> Option<String> {
        self.core.agent_thinking_option_id(agent_id)
    }

    pub fn rename_terminal(
        &mut self,
        terminal_id: &str,
        title: &str,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<()>> {
        let receiver = self.core.rename_terminal(terminal_id, title);
        self.publish(cx);
        reply_task(receiver, cx)
    }

    /// Every changed file of `cwd` with its diff, against `compare`.
    pub fn checkout_diff_with(
        &mut self,
        cwd: &str,
        compare: DiffCompare,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<Vec<FileDiff>>> {
        let receiver = self.core.checkout_diff_with(cwd, compare);
        self.publish(cx);
        reply_task(receiver, cx)
    }

    // ---- git, PR, stash, and script calls (TASK-204) ----

    /// Runs a git or PR action. Auto-merge fails with "Update the host to use auto-merge
    /// actions." when the host serves no auto-merge request.
    pub fn git_action(
        &mut self,
        cwd: &str,
        action: GitAction,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<GitActionReply>> {
        let receiver = self.core.git_action(cwd, action);
        self.publish(cx);
        reply_task(receiver, cx)
    }

    /// The full git status of `cwd`. `GitStatusEvent::Changed` follows each change.
    pub fn git_status(
        &mut self,
        cwd: &str,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<GitStatus>> {
        let receiver = self.core.git_status(cwd);
        self.publish(cx);
        reply_task(receiver, cx)
    }

    pub fn pr_status(
        &mut self,
        cwd: &str,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<PrStatusReply>> {
        let receiver = self.core.pr_status(cwd);
        self.publish(cx);
        reply_task(receiver, cx)
    }

    pub fn pr_timeline(
        &mut self,
        cwd: &str,
        number: u64,
        repo_owner: &str,
        repo_name: &str,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<Vec<PrTimelineItem>>> {
        let receiver = self.core.pr_timeline(cwd, number, repo_owner, repo_name);
        self.publish(cx);
        reply_task(receiver, cx)
    }

    pub fn switch_branch(
        &mut self,
        cwd: &str,
        branch: &str,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<()>> {
        let receiver = self.core.switch_branch(cwd, branch);
        self.publish(cx);
        reply_task(receiver, cx)
    }

    pub fn stash_save(
        &mut self,
        cwd: &str,
        branch: Option<&str>,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<()>> {
        let receiver = self.core.stash_save(cwd, branch);
        self.publish(cx);
        reply_task(receiver, cx)
    }

    pub fn stash_pop(
        &mut self,
        cwd: &str,
        index: u32,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<()>> {
        let receiver = self.core.stash_pop(cwd, index);
        self.publish(cx);
        reply_task(receiver, cx)
    }

    pub fn stash_list(
        &mut self,
        cwd: &str,
        paseo_only: bool,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<Vec<StashEntry>>> {
        let receiver = self.core.stash_list(cwd, paseo_only);
        self.publish(cx);
        reply_task(receiver, cx)
    }

    /// The workspace's scripts. `GitStatusEvent::ScriptsChanged` follows each change.
    pub fn workspace_scripts(&self, workspace_id: &str) -> Vec<WorkspaceScript> {
        self.core.workspace_scripts(workspace_id)
    }

    /// Starts a workspace script and resolves to its terminal id.
    pub fn start_workspace_script(
        &mut self,
        workspace_id: &str,
        script_name: &str,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<String>> {
        let receiver = self.core.start_workspace_script(workspace_id, script_name);
        self.publish(cx);
        reply_task(receiver, cx)
    }

    // ---- file suggestion, entry, and discard calls (TASK-206) ----

    /// Files and folders under `cwd` that match `query`, at most `limit` (1 to 100).
    pub fn directory_suggestions(
        &mut self,
        cwd: &str,
        query: &str,
        include_files: bool,
        include_directories: bool,
        limit: u32,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<Vec<DirectorySuggestion>>> {
        let receiver =
            self.core
                .directory_suggestions(cwd, query, include_files, include_directories, limit);
        self.publish(cx);
        reply_task(receiver, cx)
    }

    /// `directory_suggestions` with an explicit `matchMode`, such as `suffix` for file links.
    #[allow(clippy::too_many_arguments)]
    pub fn directory_suggestions_matching(
        &mut self,
        cwd: &str,
        query: &str,
        include_files: bool,
        include_directories: bool,
        limit: u32,
        mode: MatchMode,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<Vec<DirectorySuggestion>>> {
        let receiver = self.core.directory_suggestions_matching(
            cwd,
            query,
            include_files,
            include_directories,
            limit,
            mode,
        );
        self.publish(cx);
        reply_task(receiver, cx)
    }

    pub fn discard_changes_supported(&self) -> bool {
        self.core.discard_changes_supported()
    }

    pub fn fs_entry_ops_supported(&self) -> bool {
        self.core.fs_entry_ops_supported()
    }

    pub fn fs_entry_duplicate_supported(&self) -> bool {
        self.core.fs_entry_duplicate_supported()
    }

    /// Fails with "Update the host to use this." when `fs_entry_ops_supported` is false.
    pub fn create_entry(
        &mut self,
        cwd: &str,
        parent_path: &str,
        name: &str,
        kind: NewEntryKind,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<()>> {
        let receiver = self.core.create_entry(cwd, parent_path, name, kind);
        self.publish(cx);
        reply_task(receiver, cx)
    }

    pub fn rename_entry(
        &mut self,
        cwd: &str,
        path: &str,
        name: &str,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<()>> {
        let receiver = self.core.rename_entry(cwd, path, name);
        self.publish(cx);
        reply_task(receiver, cx)
    }

    /// Needs `fs_entry_duplicate_supported`.
    pub fn duplicate_entry(
        &mut self,
        cwd: &str,
        path: &str,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<()>> {
        let receiver = self.core.duplicate_entry(cwd, path);
        self.publish(cx);
        reply_task(receiver, cx)
    }

    pub fn delete_entry(
        &mut self,
        cwd: &str,
        path: &str,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<()>> {
        let receiver = self.core.delete_entry(cwd, path);
        self.publish(cx);
        reply_task(receiver, cx)
    }

    /// Fails without a request when `paths` is empty.
    pub fn discard_changes(
        &mut self,
        cwd: &str,
        paths: Vec<String>,
        cx: &mut Context<Client>,
    ) -> Task<anyhow::Result<()>> {
        let receiver = self.core.discard_changes(cwd, paths);
        self.publish(cx);
        reply_task(receiver, cx)
    }
}
