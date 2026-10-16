# TASK-136 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/client/Cargo.toml`: new crate `bachuc-client` (deps: `bachuc-protocol`, `gpui-kit =0.7.3`, `anyhow`, `futures`, `log`, `polling 3`, `serde`, `serde_json`, `tungstenite 0.24`, `uuid`).
- `packages/desktop-gpui/crates/client/src/lib.rs`: the `Client` GPUI entity, its batching, and the 50 ms timeline flush.
- `packages/desktop-gpui/crates/client/src/connection.rs`: the socket thread (`polling` wakeups, no read timeout loop), `hello`, the hold queue until `server_info`, and reconnect with backoff from 0.5 s to 10 s.
- `packages/desktop-gpui/crates/client/src/core.rs`: `ClientCore`, the GPUI-free state machine: request tracking by `requestId`, directory, subscriptions, permissions, terminals by slot.
- `packages/desktop-gpui/crates/client/src/timeline.rs`: `TimelineState`, `TimelineSync` (tail, live merge, end gap, reset, epoch change, resume, older pages), and the daemon's projection merge rules.
- `packages/desktop-gpui/crates/client/src/hosts.rs`, `model.rs`, `terminal.rs`: hosts file and client id, public summary types, and frame parsing.
- `packages/desktop-gpui/crates/client/tests/common/mod.rs`, `tests/sync.rs`, `tests/live.rs`: fake daemon, harness, 13 fake-server tests, 2 ignored live tests.
- `packages/desktop-gpui/crates/client/examples/client_probe.rs`: the probe.
- `packages/desktop-gpui/Cargo.lock`: updated by cargo.

## Contract

- `bachuc_client::HostConfig { name: String, url: String, password: Option<String> }` and `bachuc_client::load_hosts() -> Vec<HostConfig>`. The file is `~/Library/Application Support/BachucGpui/hosts.json` (a JSON array), seeded with `release` `ws://127.0.0.1:6767/ws` and `live` `ws://127.0.0.1:6768/ws`. The client id is in `client-id` in the same folder.
- `bachuc_client::Client` with `Client::connect(host: HostConfig, cx: &mut gpui_kit::App) -> gpui_kit::Entity<Client>`.
- `bachuc_client::ClientEvent` (`Connected`, `Disconnected { reason: String }`, `AgentsChanged`, `TimelineChanged { agent_id: String }`, `PermissionRequested { agent_id: String }`, `AgentFinished { agent_id: String }`), emitted by `Client`.
- `Client::agents(&self) -> &[AgentSummary]` with `AgentSummary { id, title, status, cwd, workspace_id, provider, model, pending_permissions }`.
- `Client::workspaces(&self) -> &[WorkspaceSummary]` with `WorkspaceSummary { id, name, cwd }`.
- `Client::providers(&self) -> &[ProviderSummary]` with `ProviderSummary { id, label, available, models: Vec<(String, String)> }`.
- `Client::view_agent`, `Client::unview_agent`, `Client::timeline(&self, agent_id: &str) -> Option<&TimelineState>`, and `Client::load_older`, each with the spec signature.
- `TimelineState { entries: Vec<serde_json::Value>, has_older: bool, version: u64, pending_permissions: Vec<serde_json::Value> }`.
- `Client::send_prompt`, `Client::cancel_agent`, `Client::set_mode`, `Client::set_model`, and `Client::agent_controls(&self, agent_id: &str) -> Option<AgentControls>` with `AgentControls { modes, current_mode, models, current_model, running }`.
- `Client::respond_permission(&mut self, agent_id: &str, request_id: &str, allow: bool, cx: &mut gpui_kit::Context<Client>)`.
- `Client::create_agent(&mut self, provider: &str, model: Option<&str>, cwd: &str, prompt: &str, cx) -> gpui_kit::Task<anyhow::Result<String>>`.
- `Client::create_terminal(cwd, cx)` and `Client::attach_terminal(terminal_id, cx)` return `gpui_kit::Task<anyhow::Result<TerminalHandle>>`. `TerminalHandle { id: String, frames: futures::channel::mpsc::UnboundedReceiver<TerminalFrame> }` and `TerminalFrame` (`Output(Vec<u8>)`, `Snapshot(serde_json::Value)`). `Client::terminal_input`, `Client::terminal_resize(terminal_id, rows: u16, cols: u16, claim: bool)`, and `Client::kill_terminal` use the spec signatures.
- Extra public items: `Client::connect_with(host, ConnectionConfig, cx)`, `Client::host`, `Client::is_connected`, `ClientCore` (the same API without GPUI, used by the tests), `ConnectionConfig`, `WorkerEvent`, `client_id()`, `data_dir()`, and the `timeline` module (`merge_entry`, `LiveRow`, `TimelineSync`, `FetchPlan`).
- Consumed from TASK-135, unchanged: `WsInbound`, `WsOutbound`, `decode_outbound` (handshake frames), `encode_inbound` (every outgoing frame, after a check against the generated types), and `PROTOCOL_VERSION`.

## Verified

- `packages/desktop-gpui/cargo.sh test -p bachuc-client` -> 9 unit tests and 13 fake-server tests pass. The fake-server tests are queue-before-hello, wrong password with backoff gaps of at least 0.5, 1, and 2 s, correct password, directory and controls, live merge, end gap with 3 or more `after` pages (page cap 2), middle gap, epoch change, replacement, reconnect, older pages, terminal attach, and terminal workspace. The sync tests also passed in 3 more runs.
- `packages/desktop-gpui/cargo.sh run -p bachuc-client --example client_probe -- ws://127.0.0.1:6768/ws <mock agent id> 12` -> exit 0. It printed 12 agents, then 128 streamed timeline lines over 12 s: the user message, assistant text, reasoning, and tool calls. Mock agent `d01c5278` was stopped and archived after the run; the earlier run used `7b5780a1`, also stopped and archived.
- `packages/desktop-gpui/cargo.sh test -p bachuc-client -- --ignored live_` -> 2 passed against 6768.
  - `live_stream_lists_agents_and_matches_a_fresh_fetch`: it creates a mock agent with `create_agent` and streams until the first tool calls. Then it cancels the agent, and the live-merged entries equal a second client's fresh tail over their overlap. Timestamps match too.
  - `live_terminal_attach_gets_a_snapshot_first`: the first frame after create and after attach is a `Snapshot`. The snapshot holds earlier output, and `Output` frames follow.
  - The tests stop and archive their agent and kill their terminal; `npm run cli -- ls -g` shows nothing running and `terminal ls --all` is empty.
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-client` -> exit 0.
- `packages/desktop-gpui/cargo.sh clippy -p bachuc-client --all-targets` -> no warnings in `bachuc-client`; the 2 remaining warnings are in `crates/protocol/build.rs`, which this TASK does not own.
- `npm run format:check:files -- watchtower/tasks/TASK-136-outcome.md` -> exit 0. No JS or TS files changed, so no lint run.
- GitNexus impact: not run, because the TASK edits no existing symbol; every file is new.

## Anti-goal

Gauge: idle CPU of the connected probe, at most 1.0%, read with `packages/desktop-gpui/measure.sh 60`.

- Started 2026-10-15T10:08:09Z. Live daemon on 6768 was running (`npm run cli -- daemon status`).
- Checkpoint 1, first build of the probe (connection, directory, and request code; no agent viewed), 2026-10-15T10:22:31Z to 10:23:31Z: `measure.sh 60 probe="examples/client_probe"` against `target/debug/examples/client_probe ws://127.0.0.1:6768/ws` -> CPU 0.0%, RSS 35 MB, footprint 10 MB. PASS (<= 1.0).
- Checkpoint 2, after timeline sync landed (13 fake-server tests green), 2026-10-15T10:26:22Z to 10:27:23Z: the probe viewing the idle live agent `7b5829ce` (tail page loaded and printed, timeline subscription held), `measure.sh 60 probe="examples/client_probe"` -> CPU 0.0%, RSS 35 MB, footprint 10 MB. PASS.
- Checkpoint 3, at completion (after `fmt` and the `clippy --fix` edits, rebuilt probe), 2026-10-15T10:33:26Z to 10:34:27Z: the probe viewing idle agent `7b5829ce`, `measure.sh 60 probe="examples/client_probe"` -> CPU 0.0%, RSS 34 MB, footprint 11 MB. PASS.
- Result: PASS. No tripwire crossed.

## Lessons

- Hold the `Entity<Client>` strongly. A `cx.subscribe` holds its emitter weakly, and when the last handle drops, the socket thread stops. The first probe run closed its socket 1 ms after `hello` for this reason.
- `TimelineChanged` comes at most every 50 ms; read `timeline(id)` on it. `version` grows on every change, including permission changes. `AgentsChanged` also covers workspace and provider changes.
- Every `TerminalFrame::Snapshot` replaces the screen. One comes first after create and attach, and again after each reconnect, because the client resubscribes open terminals.
- `create_terminal` needs a folder the daemon knows as a workspace. Otherwise the daemon answers `workspaceId is required`; `/tmp` fails, `/tmp/gpui-bench` works. The client passes `workspaceId` when a workspace's `cwd` equals the folder.
- The hello capability list leaves out `timeline_notifications` and `plugin_timeline_items`. The daemon then omits those items, and their sequence numbers show as end gaps, which one `after` fetch closes.
- `PermissionRequested` fires for permissions that appear after the first agent list, not for those pending at start. `AgentFinished` fires when an agent's status leaves `running`. `respond_permission` sends the first action whose behavior matches, so plan approvals get `implement` or `dismiss`.
- Provider labels are a local map (`Claude`, `Codex`, `GLM`, `Mock`, ...), because `list_available_providers` has no labels.
- `ClientCore` with `tests/common/mod.rs` (fake daemon and harness) tests client behavior without GPUI; `gpui-kit` test-support was not needed.

## Questions

- The spec asks for `subscribe_terminal_request` with `restore` mode `"full-snapshot"` and for `0x04` snapshot frames. A live check on 2026-10-15 showed that with `full-snapshot` the daemon sends opcode `0x05` (ANSI text), and only without `restore` does it send the `0x04` JSON grid. The full-snapshot options and the legacy path read the same full state. The client sends no `restore` to keep the `TerminalFrame::Snapshot` contract that TASK-140 needs, and plays any `0x05` frame as `Output`. Should the boundary line be changed, or should TASK-140 take ANSI restore bytes instead?
- `agent.timeline.replacement` carries only `agentId` and `epoch`, with no range. The client replaces the state with a fresh tail page, as the app does (`packages/app/src/timeline/timeline-replacement.ts`).
- Fire-and-forget request errors (send, cancel, mode, model, kill) go to `log::warn` only. Should a later TASK add an error event for the UI?
