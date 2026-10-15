# TASK-135 Generate Rust protocol types from zod

Group: A (the owner approved one group for the workspace bootstrap; no source file is shared, only the generated `Cargo.lock`)
Class: risky

## Brief

Goal: Add the crate `bachuc-protocol` whose message types are generated from the zod WebSocket schemas, so the Rust client follows protocol changes without hand edits.

Change: no Rust protocol types -> JSON Schema export plus generated serde types with stable names on top.

Boundaries:

- Follows [ADR-0035](watchtower/decisions/ADR-0035-generated-protocol-types-over-hand-written-rust-types.md): generated types while the zod schemas can export JSON Schema.
- Do not change any file under [packages/protocol](packages/protocol). The export script imports the built package only.
- Schemas to export: `WSInboundMessageSchema` and `WSOutboundMessageSchema` from [packages/protocol/src/messages.ts](packages/protocol/src/messages.ts).
- About 14 schemas in [packages/protocol/src/messages.ts](packages/protocol/src/messages.ts) use `.transform`, `.pipe`, or custom checks (for example near line 3640). Export with `z.toJSONSchema(schema, { io: "input", unrepresentable: "any" })`, as [packages/server/src/server/plugins/plugin-process.ts](packages/server/src/server/plugins/plugin-process.ts) does with `io: "input"`, and record each schema that falls back to `any`.
- Generated types must accept unknown fields and unknown message types without failing the whole frame, per [docs/protocol-compatibility.md](docs/protocol-compatibility.md). Decode an unknown `type` to a fallback, not an error.
- Planner call: generate in `build.rs` from a committed `protocol.schema.json`, so a build does not need Node. The export script refreshes the JSON file.
- Planner call: use the `typify` crate. If it cannot compile a union after one fix attempt, record it in the outcome and stop; [ADR-0035](watchtower/decisions/ADR-0035-generated-protocol-types-over-hand-written-rust-types.md) `Revisit If` then holds.

How:

- Write `codegen/export-schema.mjs`. It loads the built protocol package and writes `codegen/protocol.schema.json` with `z.toJSONSchema`.
- Write `build.rs` that turns the schema into Rust in `OUT_DIR`.
- Expose stable names and the decode and encode helpers in `src/lib.rs`.
- Record JSON fixtures from the live daemon: `server_info`, `fetch_agents_response`, `fetch_agent_timeline_response`, `agent_stream` with each timeline item type, `agent_permission_request`, `create_terminal_response`. Add one test per fixture.

Files:

- [packages/desktop-gpui/crates/protocol/Cargo.toml](packages/desktop-gpui/crates/protocol/Cargo.toml) (new crate `bachuc-protocol`)
- [packages/desktop-gpui/crates/protocol/build.rs](packages/desktop-gpui/crates/protocol/build.rs) (code generation)
- [packages/desktop-gpui/crates/protocol/src/lib.rs](packages/desktop-gpui/crates/protocol/src/lib.rs) (stable names and helpers)
- [packages/desktop-gpui/crates/protocol/codegen/export-schema.mjs](packages/desktop-gpui/crates/protocol/codegen/export-schema.mjs) (zod to JSON Schema)
- [packages/desktop-gpui/crates/protocol/codegen/protocol.schema.json](packages/desktop-gpui/crates/protocol/codegen/protocol.schema.json) (committed export)
- [packages/desktop-gpui/crates/protocol/tests/fixtures.rs](packages/desktop-gpui/crates/protocol/tests/fixtures.rs) and `tests/fixtures/*.json` (fixture tests)
- [packages/desktop-gpui/Cargo.lock](packages/desktop-gpui/Cargo.lock) (generated; updated by cargo)

Interfaces:

- Produces: `bachuc_protocol::WsInbound` and `bachuc_protocol::WsOutbound` (generated serde types of the two WebSocket envelopes) for TASK-136
- Produces: `bachuc_protocol::decode_outbound(text: &str) -> Result<WsOutbound, serde_json::Error>` for TASK-136
- Produces: `bachuc_protocol::encode_inbound(message: &WsInbound) -> String` for TASK-136
- Produces: `bachuc_protocol::PROTOCOL_VERSION: u32` (value 1) for TASK-136

Expected result:

- `node packages/desktop-gpui/crates/protocol/codegen/export-schema.mjs` rewrites the schema file. A second run gives no diff.
- `cargo.sh test -p bachuc-protocol` decodes every fixture.
- A fixture with an extra unknown field decodes. A message with an unknown `type` decodes to the fallback.
- A `hello` built in Rust and encoded with `encode_inbound` is accepted by the live daemon, which answers with `server_info`.

Anti-goal: [packages/protocol](packages/protocol) stays unchanged; tripwire; read with `git diff --stat -- packages/protocol` before changes, after the export script works, and at completion.

## Verify

- `npm run build:client` -> exit 0, then `node packages/desktop-gpui/crates/protocol/codegen/export-schema.mjs` -> writes the schema; `git diff --exit-code -- packages/desktop-gpui/crates/protocol/codegen/protocol.schema.json` after a second run -> exit 0
- `packages/desktop-gpui/cargo.sh test -p bachuc-protocol` -> all fixture tests pass, including the unknown-field and unknown-type tests
- `packages/desktop-gpui/cargo.sh test -p bachuc-protocol --test fixtures -- --ignored live_hello` -> the ignored live test gets `server_info` from `ws://127.0.0.1:6768/ws` (needs the live daemon)
- `npm run lint -- packages/desktop-gpui/crates/protocol/codegen/export-schema.mjs` -> exit 0
- Anti-goal: `git diff --stat -- packages/protocol` -> empty
