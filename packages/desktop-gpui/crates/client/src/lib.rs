//! `bachuc-client`: a GPUI entity that connects to one daemon host, keeps agents, workspaces,
//! providers, and the viewed timelines in sync, and sends the requests the UI needs.
//!
//! The socket runs on its own thread (`connection`). Its events reach the entity in batches;
//! timeline changes are announced at most every 50 ms, terminal frames go straight to each
//! terminal's channel.

pub mod connection;
pub mod core;
mod hosts;
mod model;
mod terminal;
pub mod timeline;

pub use connection::{ConnectionConfig, WorkerEvent};
pub use core::ClientCore;
pub use hosts::{HostConfig, client_id, data_dir, load_hosts};
pub use model::{AgentControls, AgentSummary, ClientEvent, ProviderSummary, WorkspaceSummary};
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
        let changed = !events.is_empty();
        for event in events {
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
}
