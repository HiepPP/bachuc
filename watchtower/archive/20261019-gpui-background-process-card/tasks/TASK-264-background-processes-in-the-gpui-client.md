# TASK-264 Background processes in the GPUI client

Group: A (the `bachuc-client` crate; shares `core.rs`, `lib.rs`, and `tests/rpcs.rs` with TASK-270; no file shared with TASK-265)
Class: code

## Brief

Goal: The GPUI client keeps each agent's background processes from its snapshot and can ask the daemon to stop one.

Change: `upsert_agent` drops `backgroundProcesses` -> the client stores it per agent, gates Stop on the host feature, and sends `agent.background_process.stop.request`.

Design: [packages/app/src/agent-stream/use-background-processes.tsx](packages/app/src/agent-stream/use-background-processes.tsx) (Electron reads the same data and sends the same request).

Boundaries:

- The schema fields are in [packages/protocol/src/messages.ts](packages/protocol/src/messages.ts) (`AgentBackgroundProcessSchema`, line 837): `id`, `kind`, `description`, `command` (nullable), `status`, `toolCallId` (nullable), `startedAt`, `endedAt` (nullable), `exitCode` (nullable), `outputFile` (nullable), `tail`.
- `kind` and `status` stay plain strings, as the schema comment asks, so a later value does not break this client.
- [packages/desktop-gpui/crates/protocol/codegen/protocol.schema.json](packages/desktop-gpui/crates/protocol/codegen/protocol.schema.json) already holds `backgroundProcesses`, the stop request, and `agentBackgroundProcesses`. Do not run the export script.
- Planner call: parse the snapshot from `Value` in [packages/desktop-gpui/crates/client/src/core.rs](packages/desktop-gpui/crates/client/src/core.rs), as `queued_prompts` does for `queue`. Read the feature flag from the generated `ServerInfo`, as `supports_queue` does. This follows [ADR-0035](watchtower/decisions/ADR-0035-generated-protocol-types-over-hand-written-rust-types.md): no hand-written wire type.
- Planner call: a failed stop logs a warning through `Pending::Action`, as `cancel_agent` does. Electron shows a toast; GPUI has no toast host in the client.
- A missing or empty `backgroundProcesses` field gives an empty list, not an error.

How:

- Add `pub struct BackgroundProcess` to [packages/desktop-gpui/crates/client/src/model.rs](packages/desktop-gpui/crates/client/src/model.rs), with `Clone, Debug, PartialEq, Eq`, and export it from `lib.rs`.
- Parse times to Unix ms with the existing `parse_iso_ms` in `model.rs`. A `startedAt` that does not parse drops that process. An `endedAt` that does not parse gives `None`.
- `upsert_agent` returns early for a snapshot older than the stored `updatedAt` (`core.rs:3332`). Parse the processes after that check, so an old snapshot does not replace newer processes.
- Add a `background_processes: Vec<BackgroundProcess>` field to `AgentExtra` (`core.rs:257`). Fill it in `upsert_agent` (`core.rs:3313`) from `agent["backgroundProcesses"]`.
- Add `ClientCore::background_processes`, `ClientCore::supports_background_stop`, and `ClientCore::stop_background_process` next to `queued_prompts`, `supports_queue`, and `cancel_agent`.
- Add the `Client` wrappers in `lib.rs` next to `queued_prompts`. The stop wrapper calls `self.publish(cx)`, as `cancel_agent` does.
- Add tests to [packages/desktop-gpui/crates/client/tests/rpcs.rs](packages/desktop-gpui/crates/client/tests/rpcs.rs) with the existing harness.

Files:

- [packages/desktop-gpui/crates/client/src/model.rs](packages/desktop-gpui/crates/client/src/model.rs) (the `BackgroundProcess` struct)
- [packages/desktop-gpui/crates/client/src/core.rs](packages/desktop-gpui/crates/client/src/core.rs) (`AgentExtra` field, snapshot parse, feature gate, stop request)
- [packages/desktop-gpui/crates/client/src/lib.rs](packages/desktop-gpui/crates/client/src/lib.rs) (export and `Client` wrappers)
- [packages/desktop-gpui/crates/client/tests/rpcs.rs](packages/desktop-gpui/crates/client/tests/rpcs.rs) (new tests)

Interfaces:

- Produces: `bachuc_client::BackgroundProcess { id: String, kind: String, description: String, command: Option<String>, status: String, tool_call_id: Option<String>, started_at_ms: i64, ended_at_ms: Option<i64>, exit_code: Option<i64>, output_file: Option<String>, tail: Vec<String> }` for TASK-265
- Produces: `Client::background_processes(&self, agent_id: &str) -> Vec<BackgroundProcess>` for TASK-265
- Produces: `Client::supports_background_stop(&self) -> bool` for TASK-265
- Produces: `Client::stop_background_process(&mut self, agent_id: &str, process_id: &str, cx: &mut Context<Client>)` for TASK-265
- Produces: no name for TASK-270. TASK-270 follows by the shared client files.

Expected result:

- An `agent_update` upsert with two processes gives two `BackgroundProcess` values in daemon order, with all fields kept.
- A later upsert with an empty list or no field clears the list.
- `supports_background_stop` is true only when `server_info.features.agentBackgroundProcesses` is `true`.
- `stop_background_process("a1", "p1")` sends `{"type": "agent.background_process.stop.request", "agentId": "a1", "processId": "p1", "requestId": ...}`.
- A stop response with a non-null `error` logs a warning and does not panic.

Anti-goal: no existing `bachuc-client` test fails, and no struct literal in another crate breaks; tripwire; read from `./cargo.sh test -p bachuc-client` and `./cargo.sh build` before changes, after the struct change, and at completion.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-client` -> the new tests and every old test pass.
- New test in `rpcs.rs`: an upsert with two processes, one with `command: null` and `exitCode: 0` -> `background_processes("a1")` returns both, with `command: None` and `exit_code: Some(0)`.
- New test in `rpcs.rs`: an upsert with no `backgroundProcesses` field after one with a process -> `background_processes("a1")` is empty.
- New test in `rpcs.rs`: a process with `startedAt: "not a date"` next to a valid one -> only the valid one is kept; an unparsable `endedAt` gives `ended_at_ms: None`.
- New test in `rpcs.rs`: `server_info` with `agentBackgroundProcesses: true`, then with the flag absent -> `supports_background_stop` is true, then false.
- New test in `rpcs.rs`: `stop_background_process("a1", "p1")` -> the fake server receives the stop request with those IDs; a reply with `error: "gone"` leaves the core running.
- `cd packages/desktop-gpui && ./cargo.sh build` -> the workspace builds, so no struct literal in another crate broke.
- Anti-goal: the old test count before changes equals the old tests still passing at completion; record both counts.
