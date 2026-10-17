# TASK-221 Fork sends a boundary cursor

Group: T (shared file [packages/desktop-gpui/crates/timeline/src/lib.rs](packages/desktop-gpui/crates/timeline/src/lib.rs))
Class: code

## Brief

Goal: Fork works from a turn of any provider when the host supports cursors, as in Electron.

Change: the footer fork sends only the message id, so a GLM agent (no `messageId` in `packages/server/src/server/agent/providers/glm`) cannot fork -> the fork sends `boundaryCursor { epoch, seq }` when the host supports it, plus the message id when present.

Boundaries:

- `footer_fork_boundary` hard-codes `let epoch = None;` (`lib.rs` lines 470-485). Use `Client::timeline_epoch` instead. `fork_boundary` (lines 2443-2466) already matches Electron's rule; keep it.
- Electron rule ([packages/app/src/agent-stream/turn-boundary.ts](packages/app/src/agent-stream/turn-boundary.ts) lines 7-23): cursor when `supportsTimelineCursor`; message id when present; neither -> no fork button.
- Owner answer (2026-10-18): send the cursor by epoch. Q-129.

How:

- Read the epoch in `footer_fork_boundary` and pass it to `fork_boundary`.
- `fork_boundary` already has a no-message-id test with an epoch (`lib.rs` lines 2697-2703); it passes today. The change is the call site only, so check it with `rg` and the UI check.

Files:

- [packages/desktop-gpui/crates/timeline/src/lib.rs](packages/desktop-gpui/crates/timeline/src/lib.rs)

Interfaces:

- Consumes: `Client::timeline_epoch(&self, agent_id: &str) -> Option<&str>` from TASK-220.

Expected result:

- A completed turn of an agent without message ids shows the fork button on a host with `agentForkContextCursor`, and the fork request carries `boundaryCursor`.

Anti-goal: no extra work per frame; drift gauge; `footer_fork_boundary` runs only while the footer renders, as before (read the call site before and after).

## Verify

- `cd packages/desktop-gpui && CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-timeline` -> passes.
- `rg -n "let epoch = None" packages/desktop-gpui/crates/timeline/src/lib.rs` -> no match; `rg -n "timeline_epoch" packages/desktop-gpui/crates/timeline/src/lib.rs` -> one match.
- UI check on live: fork from a completed turn of a disposable agent -> a draft opens with the chat history chip.
- Anti-goal: `rg -n "footer_fork_boundary" packages/desktop-gpui/crates/timeline/src/lib.rs` shows the same call sites before and after.
