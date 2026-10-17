# TASK-221 outcome

## Status

DONE (code). The screen check is DEFERRED (UI checker).

## Changed

- `packages/desktop-gpui/crates/timeline/src/lib.rs`: `footer_fork_boundary` reads `client.timeline_epoch(&self.agent_id)` instead of `let epoch = None;`, and passes it to `fork_boundary`. The stale comment about the missing epoch is gone. `fork_boundary` is unchanged.

## Contract

- Consumes: `Client::timeline_epoch(&self, agent_id: &str) -> Option<&str>` from TASK-220, unchanged.
- On a host with `agentForkContextCursor` and a loaded timeline page, the footer boundary has `cursor { epoch, seq: seqEnd }`, plus the message id when present. An assistant message without `messageId` now gets a fork button. With no cursor support and no message id, there is still no fork button.

## Verified

- `CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-timeline` -> pass (lib 23 tests, with `fork_boundary_follows_the_turn_boundary_rule`). Log: `/tmp/t221-tl.txt`.
- `rg -n "let epoch = None"` -> no match. `rg -n "timeline_epoch"` -> one match (line 480).
- Anti-goal: `rg -n "footer_fork_boundary"` -> the definition plus the same 2 call sites, `render_footer` and the line 1663 caller, as before. The added work is one hash-map read per footer draw.
- `./cargo.sh fmt --check -p bachuc-timeline` -> exit 0.
- `./cargo.sh metadata --format-version 1 > /dev/null` -> exit 0.
- `CARGO_BUILD_JOBS=4 ./cargo.sh build -p bachuc-app` -> exit 0. Log: `/tmp/t221-app.txt`.
- GitNexus impact for `footer_fork_boundary` -> UNKNOWN (stale index, target not found). `rg` confirmed the 2 callers.
- UI check: fork from a completed turn of a disposable agent opens a draft with the chat history chip -> DEFERRED (UI checker).

## Lessons

- `timeline_epoch` borrows from the client read guard, so it fits the existing `let client = self.client.read(cx);` scope with no clone.
