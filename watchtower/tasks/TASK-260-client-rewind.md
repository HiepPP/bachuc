# TASK-260 Client rewind call and capabilities

Group: C (writes [packages/desktop-gpui/crates/client/src/core.rs](packages/desktop-gpui/crates/client/src/core.rs) and [packages/desktop-gpui/crates/client/src/lib.rs](packages/desktop-gpui/crates/client/src/lib.rs); shared files with TASK-259)
Class: risky

## Brief

Goal: the GPUI client can rewind an agent to a user message, with the same modes Electron offers.

Change: no rewind support -> a `rewind` call and per-agent rewind modes.

Boundaries:

- Wire: request `{ type: "agent.rewind.request", agentId, messageId, mode: "conversation" | "files" | "both", requestId }`. Response `agent.rewind.response` with `payload { requestId, agentId, ok, error }`. Source: [packages/protocol/src/messages.ts](packages/protocol/src/messages.ts) (lines 2118-2136).
- Modes come from `agent.capabilities`: `supportsRewindConversation`, `supportsRewindFiles`, `supportsRewindBoth`, default false ([packages/protocol/src/agent-types.ts](packages/protocol/src/agent-types.ts) lines 163-165). Parse them in `upsert_agent`.
- Gate on the server feature `rewind` with `server_feature` ([packages/desktop-gpui/crates/client/src/core.rs](packages/desktop-gpui/crates/client/src/core.rs) line 1256). An old host gives no modes.
- Follow the fork pattern (see [watchtower/CONTEXT.md](watchtower/CONTEXT.md)): a `Pending` variant, `request_reply`, and an `ok`/`error` reply.
- After a successful `conversation` or `both` rewind, refresh the timeline tail the way the client already re-syncs, as Electron does in [packages/app/src/components/rewind/use-rewind-agent-mutation.ts](packages/app/src/components/rewind/use-rewind-agent-mutation.ts) (lines 37-50).

Files:

- [packages/desktop-gpui/crates/client/src/core.rs](packages/desktop-gpui/crates/client/src/core.rs) (capabilities parse, request, reply, tail refresh)
- [packages/desktop-gpui/crates/client/src/lib.rs](packages/desktop-gpui/crates/client/src/lib.rs) (public API)
- [packages/desktop-gpui/crates/client/src/model.rs](packages/desktop-gpui/crates/client/src/model.rs) (`RewindMode`, `RewindModes`)
- [packages/desktop-gpui/crates/client/tests/rpcs.rs](packages/desktop-gpui/crates/client/tests/rpcs.rs) (tests)

Interfaces:

- Produces: `Client::rewind_modes(&self, agent_id: &str) -> RewindModes` (`conversation`, `files`, `both`: bool) for TASK-263
- Produces: `Client::rewind(&self, agent_id: &str, message_id: &str, mode: RewindMode, cx: &mut App) -> Task<Result<(), String>>` for TASK-263

Expected result:

- With a host that supports rewind, `rewind_modes` matches the agent flags and `rewind` sends the request and resolves on the reply.

Anti-goal: a failed rewind changes no client state; the timeline item count after an `ok: false` reply equals the count before; read in the error-path test.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-client` -> new tests pass: the request payload matches the wire shape; `ok: true` resolves `Ok`; `ok: false` resolves `Err` with the error text and keeps the timeline count; no `rewind` feature gives all modes false.
