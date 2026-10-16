# TASK-135 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/protocol/Cargo.toml`: crate `bachuc-protocol`; build deps `typify =0.8.0`, `schemars 0.8`, `syn`, `quote`, `heck`, `prettyplease`; dev dep `tungstenite 0.24`.
- `packages/desktop-gpui/crates/protocol/build.rs`: schema adjustment, naming, typify, `Unrecognized` fallback variants.
- `packages/desktop-gpui/crates/protocol/src/lib.rs`: `generated` module, stable names, helpers.
- `packages/desktop-gpui/crates/protocol/codegen/export-schema.mjs`: zod to JSON Schema (draft-07, `io: "input"`, `unrepresentable: "any"`), then formats the output with `npm run format:files`.
- `packages/desktop-gpui/crates/protocol/codegen/protocol.schema.json`: committed export (3.4 MB).
- `packages/desktop-gpui/crates/protocol/tests/fixtures.rs` and 13 files in `tests/fixtures/`, recorded from live on 2026-10-15 and formatted with `npm run format:files`.
- `packages/desktop-gpui/Cargo.lock`: updated by cargo.

## Contract

- `bachuc_protocol::WsInbound` and `bachuc_protocol::WsOutbound`: generated serde enums, internally tagged by `type`.
- `bachuc_protocol::decode_outbound(text: &str) -> Result<WsOutbound, serde_json::Error>`.
- `bachuc_protocol::encode_inbound(message: &WsInbound) -> String`.
- `bachuc_protocol::PROTOCOL_VERSION: u32 = 1`.
- Extra stable names: `SessionInbound` (= `generated::WsInboundSessionMessage`) and `SessionOutbound` (= `generated::WsOutboundSessionMessage`). All generated types are in `bachuc_protocol::generated`.

## Verified

- `npm run build:client` -> exit 0.
- `node packages/desktop-gpui/crates/protocol/codegen/export-schema.mjs` -> writes the schema; two more runs gave the same `shasum` (`fd5fa32f...`). `git diff --exit-code -- .../protocol.schema.json` -> exit 0, but this is trivially true while the file is untracked (see Questions).
- `packages/desktop-gpui/cargo.sh test -p bachuc-protocol` -> 17 passed, 1 ignored. Each fixture decodes, no level is `Unrecognized`, and re-encoding gives the same JSON (nulls, empty arrays, and empty maps dropped; numbers compared as f64). Includes `unknown_fields_are_ignored`, `unknown_message_type_decodes_to_the_fallback`, `unknown_stream_event_decodes_to_the_fallback`.
- `packages/desktop-gpui/cargo.sh test -p bachuc-protocol --test fixtures -- --ignored live_hello` -> 1 passed against `ws://127.0.0.1:6768/ws` (hello built in Rust, `server_info` received).
- `rtk proxy npm run lint -- packages/desktop-gpui/crates/protocol/codegen/export-schema.mjs` -> 0 warnings, 0 errors. Plain `npm run lint` exits 2 through the rtk wrapper (known, MEMORY 2026-10-14).
- `npm run format:check:files -- <schema, fixtures, export-schema.mjs>` -> exit 0.
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-protocol` -> exit 0.
- Clean build of the crate after `cargo.sh clean -p bachuc-protocol`: 21 s, below the ADR-0035 3 minute limit.
- Group-level run (bundle, UI check, measure) is recorded in the TASK-134 outcome.
- Cleanup: recorder mock agent `ad9b52ad-...` stopped and archived; terminal `1e3e6a46` killed.

## Anti-goal

- `git diff --stat -- packages/protocol` and `git status --short -- packages/protocol`:
  - Before changes, 2026-10-15T09:32:31Z: empty.
  - After the export script worked, 2026-10-15T09:34:23Z: empty; again at 09:54:17Z after the formatted export: empty.
  - At completion, 2026-10-15T09:57:42Z: empty. PASS.

## Lessons

- Only 1 schema falls back to `any` with `io: "input"`: `WsOutbound` date at `/oneOf/1/properties/message/oneOf/30/properties/payload/properties/timestamp`. Transforms export their input side, and refinements are dropped, so the expected 14 fallbacks do not occur.
- typify 0.8 merges two types of the same name without a check, and names inline types after the parent. Without titles, every session message `payload` shared one type. `build.rs` gives each type a unique title and fails the build if typify names a type itself.
- Type names: tagged variants take their tag (`agent_stream` -> `AgentStream`, payload `AgentStreamPayload`, event `AgentStreamPayloadEvent`, timeline item `TimelineItem`). Identical schemas share one type, named with the shortest candidate. Names are generated, so check `target/debug/build/bachuc-protocol-*/out/protocol.rs`.
- `build.rs` drops `pattern`, `format`, length limits, `default`, and `additionalProperties: false`. Optional fields decode to `None`; optional arrays and maps default to empty.
- Nested unions are spliced and members with one tag value merge. `tool_call` is one `TimelineItem::ToolCall` variant with `status` as a string enum and `error` as `serde_json::Value`.
- Only internally tagged enums have `Unrecognized`. A new value in a string enum still fails its frame, as in the TypeScript client.
- `server_info` is `SessionOutbound::Status { payload: StatusPayload { status, extra } }`; its fields are in the untyped `extra` map, because the envelope schema types `status` payloads only as passthrough objects.
- `CheckoutStatusUpdatePayload` is a typify struct of flattened `Option` variants (a non-exclusive union plus an intersection). No fixture covers it.
- The live daemon broadcasts every agent's stream to a client whose `hello` has no subscription capabilities. The mock agent emits `todo` items for "emit 2 agent stream updates" and a permission request for "emit a synthetic plan approval".

## Questions

- The Verify line `git diff --exit-code -- .../protocol.schema.json` cannot fail while the file is untracked. Replace it with a `shasum` comparison of two runs, or run it after the lead stages the file.
- The `server_info` payload is untyped. Should a later TASK also export `ServerInfoStatusPayloadSchema` as a third root for TASK-136?
- During the work, one import line in `build.rs` was edited with `sed -i` against the builder rule; the content is correct and was rebuilt and tested.
