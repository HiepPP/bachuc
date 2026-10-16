//! `bachuc-client`: a GPUI entity that connects to one daemon host, keeps agents, workspaces,
//! providers, and the viewed timelines in sync, and sends the requests the UI needs.
//!
//! The socket runs on its own thread (`connection`). Its events reach the entity in batches;
//! timeline changes are announced at most every 50 ms, terminal frames go straight to each
//! terminal's channel.

pub mod connection;
pub mod core;
mod files;
mod hosts;
mod model;
mod terminal;
pub mod timeline;

pub use connection::{ConnectionConfig, WorkerEvent};
pub use core::ClientCore;
pub use hosts::{HostConfig, client_id, data_dir, load_hosts, save_hosts};
pub use model::{AgentControls, AgentSummary, ClientEvent, ProviderSummary, WorkspaceSummary};
pub use model::{
    ChangedFile, CheckoutStatus, DirectoryEvent, FileContent, FileEntry, FileKind, ProjectNode,
    QueuedPrompt, SlashCommand, ThinkingControls, TurnBehavior, WorkspaceNode,
};
pub use terminal::{TerminalFrame, TerminalHandle};
pub use timeline::TimelineState;

use futures::StreamExt as _;
use gpui_kit::{App, AppContext as _, Context, Entity, EventEmitter, Task};
use std::time::{Duration, Instant};

/// Timeline changes reach the UI at most this often.
const FLUSH_INTERVAL: Duration = Duration::from_millis(50);

/// One daemon host connection as a GPUI entity. It emits `ClientEvent`.
pub struct Client {
    host: HostConfig,
    core: ClientCore,
    last_flush: Option<Instant>,
    flush_task: Option<Task<()>>,
    _reader: Task<()>,
}

impl EventEmitter<ClientEvent> for Client {}
impl EventEmitter<DirectoryEvent> for Client {}

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
                _reader: reader,
            }
        })
    }

    /// Emits pending events now and schedules the timeline flush.
    fn publish(&mut self, cx: &mut Context<Self>) {
        let events = self.core.take_events();
        let directory_events = self.core.take_directory_events();
        let changed = !events.is_empty() || !directory_events.is_empty();
        for event in events {
            cx.emit(event);
        }
        for event in directory_events {
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

    /// Fetches the page before the first entry when `has_older` is set.
    pub fn load_older(&mut self, agent_id: &str, cx: &mut Context<Client>) {
        self.core.load_older(agent_id);
        self.publish(cx);
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

    pub fn mark_workspace_unread(&mut self, workspace_id: &str, cx: &mut Context<Client>) {
        self.core.mark_workspace_unread(workspace_id);
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
}
