# TASK-136 Client core: connection, requests, and sync

Group: B (writes only `crates/client`)
Class: risky

## Brief

Goal: Add the crate `bachuc-client`, a GPUI entity that connects to one daemon host, keeps agents, workspaces, providers, and the viewed timelines in sync, and sends every request the UI TASKs need.

Change: the prototype's thread-and-poll socket in one file -> a reusable client entity with reconnect, request tracking, and timeline sync.

Boundaries:

- One host at a time. A host is `HostConfig { name, url, password }` with a plain `ws://` URL. Planner call: hosts persist in `~/Library/Application Support/BachucGpui/hosts.json`, seeded with release `ws://127.0.0.1:6767/ws` and live `ws://127.0.0.1:6768/ws`.
- Send `hello` with `clientId` (a UUID kept in the hosts file folder), `clientType` `"cli"`, `protocolVersion` 1, optional password auth, and the capabilities `hello_rejection`, `all_providers`, `selective_agent_timeline`, `explicit_event_subscriptions`, `owned_subscriptions`, `timeline_replacement_invalidation`, `reasoning_merge_enum`, `custom_mode_icons`. Names come from [packages/protocol/src/client-capabilities.ts](packages/protocol/src/client-capabilities.ts).
- With `selective_agent_timeline`, the daemon streams only agents in the viewed set. Send `agent.timeline.set_subscription.request` with `agentIds` whenever the viewed set changes.
- Timeline sync follows [docs/timeline-sync.md](docs/timeline-sync.md):
  - First load: `fetch_agent_timeline_request` with `direction` `"tail"` and `projection` `"projected"`.
  - Live items: merge `agent_stream` timeline items in `seq` order.
  - A gap at the end: fetch `direction` `"after"` from the last cursor until `hasNewer` is false.
  - A gap in the middle or an epoch change: replace the state with a fresh tail page.
  - `agent.timeline.replacement`: replace the named range as the doc says.
  - Older pages: `direction` `"before"` from the first cursor.
- Directory data: `fetch_agents_request` with `subscribe` for agents (including each agent's pending permission count), `fetch_workspaces_request` for workspace names, `list_available_providers_request` and `list_provider_models_request` for the new-agent dialog and the model pill.
- Queue session messages until `server_info` arrives (see [watchtower/CONTEXT.md](watchtower/CONTEXT.md)).
- Reconnect with backoff from 0.5 seconds up to 10 seconds. After a reconnect, resend `hello`, the directory subscriptions, and the viewed set, then refetch viewed timelines and resubscribe open terminals.
- Read the socket off the main thread. Hand frames to the GPUI entity in batches; do not notify the UI per frame. Chat items flush at most every 50 ms; terminal frames flush every frame.
- Terminals: `create_terminal` creates and subscribes; `attach_terminal` subscribes to an existing terminal id. Both send `subscribe_terminal_request` with `restore` mode `"full-snapshot"`, and route frames by slot. Snapshot frames (`0x04`) reach the view as `TerminalFrame::Snapshot` with the parsed JSON grid.

How:

- Write the socket task, request tracking by `requestId`, and the reconnect loop.
- Write the agent, workspace, and provider state.
- Write the timeline state per viewed agent, with live merge, gap handling, replacement, and older pages.
- Write the request methods and terminal routing by slot.
- Add an example `examples/client_probe.rs` that prints agents and the live stream of one agent.

Files:

- [packages/desktop-gpui/crates/client/Cargo.toml](packages/desktop-gpui/crates/client/Cargo.toml) (new crate `bachuc-client`)
- [packages/desktop-gpui/crates/client/src](packages/desktop-gpui/crates/client/src) (client entity, state, requests)
- [packages/desktop-gpui/crates/client/tests](packages/desktop-gpui/crates/client/tests) (unit tests with a local fake server, and ignored live tests)
- [packages/desktop-gpui/crates/client/examples/client_probe.rs](packages/desktop-gpui/crates/client/examples/client_probe.rs) (probe example)

Interfaces:

- Consumes: `bachuc_protocol::WsInbound` and `bachuc_protocol::WsOutbound` (generated serde types of the two WebSocket envelopes) from TASK-135
- Consumes: `bachuc_protocol::decode_outbound(text: &str) -> Result<WsOutbound, serde_json::Error>` from TASK-135
- Consumes: `bachuc_protocol::encode_inbound(message: &WsInbound) -> String` from TASK-135
- Consumes: `bachuc_protocol::PROTOCOL_VERSION: u32` (value 1) from TASK-135
- Produces: `bachuc_client::HostConfig { name: String, url: String, password: Option<String> }` and `bachuc_client::load_hosts() -> Vec<HostConfig>` for TASK-141
- Produces: `bachuc_client::Client` (a GPUI entity) with `Client::connect(host: HostConfig, cx: &mut gpui_kit::App) -> gpui_kit::Entity<Client>` for TASK-138, TASK-139, TASK-140, TASK-141
- Produces: `bachuc_client::ClientEvent` (enum `Connected`, `Disconnected { reason: String }`, `AgentsChanged`, `TimelineChanged { agent_id: String }`, `PermissionRequested { agent_id: String }`, `AgentFinished { agent_id: String }`) emitted by `Client` for TASK-138, TASK-141
- Produces: `Client::agents(&self) -> &[AgentSummary]` with `AgentSummary { id: String, title: String, status: String, cwd: String, workspace_id: Option<String>, provider: String, model: Option<String>, pending_permissions: usize }` for TASK-141
- Produces: `Client::workspaces(&self) -> &[WorkspaceSummary]` with `WorkspaceSummary { id: String, name: String, cwd: String }` for TASK-141
- Produces: `Client::providers(&self) -> &[ProviderSummary]` with `ProviderSummary { id: String, label: String, available: bool, models: Vec<(String, String)> }` for TASK-141
- Produces: `Client::view_agent(&mut self, agent_id: &str, cx: &mut gpui_kit::Context<Client>)`, `Client::unview_agent(&mut self, agent_id: &str, cx: &mut gpui_kit::Context<Client>)`, `Client::timeline(&self, agent_id: &str) -> Option<&TimelineState>`, and `Client::load_older(&mut self, agent_id: &str, cx: &mut gpui_kit::Context<Client>)` for TASK-138
- Produces: `TimelineState { entries: Vec<serde_json::Value>, has_older: bool, version: u64, pending_permissions: Vec<serde_json::Value> }` (entries are projected timeline entries in order) for TASK-138
- Produces: `Client::send_prompt(&mut self, agent_id: &str, text: String, images: Vec<serde_json::Value>, cx: &mut gpui_kit::Context<Client>)`, `Client::cancel_agent(&mut self, agent_id: &str, cx: &mut gpui_kit::Context<Client>)`, `Client::set_mode(&mut self, agent_id: &str, mode_id: &str, cx: &mut gpui_kit::Context<Client>)`, `Client::set_model(&mut self, agent_id: &str, model_id: &str, cx: &mut gpui_kit::Context<Client>)`, and `Client::agent_controls(&self, agent_id: &str) -> Option<AgentControls>` with `AgentControls { modes: Vec<(String, String)>, current_mode: Option<String>, models: Vec<(String, String)>, current_model: Option<String>, running: bool }` for TASK-139
- Produces: `Client::respond_permission(&mut self, agent_id: &str, request_id: &str, allow: bool, cx: &mut gpui_kit::Context<Client>)` for TASK-138
- Produces: `Client::create_agent(&mut self, provider: &str, model: Option<&str>, cwd: &str, prompt: &str, cx: &mut gpui_kit::Context<Client>) -> gpui_kit::Task<anyhow::Result<String>>` for TASK-141
- Produces: `Client::create_terminal(&mut self, cwd: &str, cx: &mut gpui_kit::Context<Client>) -> gpui_kit::Task<anyhow::Result<TerminalHandle>>`, `Client::attach_terminal(&mut self, terminal_id: &str, cx: &mut gpui_kit::Context<Client>) -> gpui_kit::Task<anyhow::Result<TerminalHandle>>`, `TerminalHandle { id: String, frames: futures::channel::mpsc::UnboundedReceiver<TerminalFrame> }`, `TerminalFrame` (enum `Output(Vec<u8>)`, `Snapshot(serde_json::Value)`), `Client::terminal_input(&mut self, terminal_id: &str, bytes: &[u8])`, `Client::terminal_resize(&mut self, terminal_id: &str, rows: u16, cols: u16, claim: bool)`, and `Client::kill_terminal(&mut self, terminal_id: &str, cx: &mut gpui_kit::Context<Client>)` for TASK-140

Expected result:

- The probe example lists live agents and prints the stream of a mock agent.
- Dropping the socket (a local fake server closes it) leads to a reconnect and a refetch, with no duplicate timeline entries.
- A gap at the end triggers `after` fetches until `hasNewer` is false. A middle gap or an epoch change replaces the state with a tail page. Both end equal to a fresh fetch.
- An `agent.timeline.replacement` message replaces the named range.
- A session message sent before `server_info` is held and then sent; the daemon does not close the socket.
- A wrong password gives `Disconnected` with the `hello.rejected` reason, and retries no faster than the backoff. A correct password connects.
- `attach_terminal` on an existing terminal gives a `Snapshot` frame first, then `Output` frames.

Anti-goal: idle CPU of the connected probe stays at most 1% with no agent streaming; drift gauge; read with `measure.sh 60` before the first commit, after timeline sync lands, and at completion.

## Verify

- `packages/desktop-gpui/cargo.sh test -p bachuc-client` -> all tests pass, including reconnect, end gap, middle gap, epoch change, replacement, queue-before-hello, wrong password, and correct password tests against a local fake server
- `packages/desktop-gpui/cargo.sh run -p bachuc-client --example client_probe -- ws://127.0.0.1:6768/ws <mock agent id>` -> prints the agent list, then streamed timeline items for 10 seconds (needs the live daemon and a mock agent)
- `packages/desktop-gpui/cargo.sh test -p bachuc-client -- --ignored live_` -> live tests pass against port 6768, including a terminal attach that gets a snapshot first
- Anti-goal: `measure.sh 60 probe="client_probe"` while connected and idle -> CPU at most 1.0
