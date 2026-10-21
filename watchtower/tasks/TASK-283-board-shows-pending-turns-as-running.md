# TASK-283 Board shows pending turns as Running

Group: B (writes `packages/desktop-gpui/plugins/board/src/**` only; starts after TASK-282 by order)
Class: risky

## Brief

Goal: WHEN Hiep comes back to the Board after a What next Send, the sent thread's card is already in the Running column on the first frame.

Change: cards move only when a `board.snapshot` poll answers -> the store shows a marked agent's finished run as Running at once, then lets the next snapshot confirm it.

Boundaries:

- Follows [ADR-0058](watchtower/decisions/ADR-0058-gpui-pending-turn-mark-over-waiting-for-the-board-poll.md): the overlay reads `Client::pending_turn`; it never sends a prompt and never creates a card.
- Follows [ADR-0045](watchtower/decisions/ADR-0045-native-rust-plugin-clients-over-webview-panes-or-shared-logic.md): no dependency on the next-prompt-actions crate.
- The overlay changes only runs whose `agent_id` has a mark. It sets `status` to `Running`, `ended_at` to `None`, and `started_at` to the mark time in RFC 3339 UTC, so the card timer starts at 0 s.
- The board server keeps one run per agent: `start` in [store.ts](hiep-plugins/plugins/board/server/store.ts) drops the agent's older finished run. So the overlay changes at most one run per mark.
- A mark shows while the run is not running, and its `ended_at` is at or before the mark or cannot be read. An end at the mark time is the old turn. The 15,000 ms limit in `pending_turn` bounds the unreadable case.
- Pills read the same store snapshot, so a pending run hides the Remove pills too. This is intended: the thread is busy.
- Planner call: refresh only when an event makes a new card show as Running. A clear never refreshes, so a confirm cannot start a poll loop.
- Trap: `receive` in `store.rs`: an `Unchanged` answer skips the decode. Keep the raw snapshot, and build the shown snapshot from it on each answer and each `PendingTurnsChanged`.
- Trap: `interval` in `store.rs`: use the shown snapshot, so the 2,000 ms cadence runs while a mark shows.
- Trap: `revision` in `store.rs`: bump it only when the shown snapshot changes. Views rebuild on each bump.

How:

- Add `shows_pending` and `apply_pending` in a new `model/pending.rs`, and register the module in `model/mod.rs`.
- In `BoardStore` (`store.rs`), keep the raw snapshot, subscribe to `PendingTurnsChanged`, and rebuild the shown snapshot with `apply_pending`.
- After each answer, call `clear_turn_pending` for each marked agent whose run no longer `shows_pending`.
- Arm one timer at the earliest mark time plus `PENDING_TURN_TTL_MS`, and rebuild the shown snapshot when it fires.
- The subscription runs before the Board page first renders, because `open_surface` defers its host event (`emit` in [context.rs](packages/desktop-gpui/crates/plugin-host/src/context.rs)). If the store did not exist yet, the first answer applies the mark.

Files:

- [packages/desktop-gpui/plugins/board/src/model/pending.rs](packages/desktop-gpui/plugins/board/src/model/pending.rs) (new: `shows_pending`, `apply_pending`, unit tests)
- [packages/desktop-gpui/plugins/board/src/model/mod.rs](packages/desktop-gpui/plugins/board/src/model/mod.rs) (register `pending`)
- [packages/desktop-gpui/plugins/board/src/store.rs](packages/desktop-gpui/plugins/board/src/store.rs) (raw and shown snapshot, subscription, confirm, expiry timer)

Interfaces:

- Consumes: `bachuc_client::PendingTurnsChanged` (a unit struct event, `Clone, Copy, Debug, PartialEq, Eq`) emitted by `Client` from TASK-282
- Consumes: `bachuc_client::PENDING_TURN_TTL_MS: i64` (15_000) from TASK-282
- Consumes: `Client::clear_turn_pending(&mut self, agent_id: &str, cx: &mut Context<Client>)` from TASK-282
- Consumes: `Client::pending_turn(&self, agent_id: &str) -> Option<i64>` (the mark time in UNIX ms) from TASK-282

Expected result:

- WHEN an agent with a finished card gets a mark THE Board SHALL show that card in Running before any new `board.snapshot` answer.
- WHEN a snapshot shows the marked run running THE Board store SHALL clear the mark.
- WHEN a snapshot shows the marked run ended after the mark THE Board SHALL show it in Just finished and clear the mark.
- WHEN the mark is cleared or expires before a confirm THE Board SHALL show the card in Just finished again.
- IF a marked agent has no run in the snapshot THEN THE Board SHALL add no card.
- IF an event changes no card THEN THE Board store SHALL keep its revision.

Red tests:

File: [packages/desktop-gpui/plugins/board/src/model/pending.rs](packages/desktop-gpui/plugins/board/src/model/pending.rs)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::RunStatus;
    use crate::model::run::tests::card;

    fn mark() -> i64 {
        chrono::DateTime::parse_from_rfc3339("2026-10-21T00:00:00Z")
            .unwrap()
            .timestamp_millis()
    }

    fn finished(agent_id: &str, ended_at: &str) -> Run {
        let mut run = card(agent_id, "app", false);
        run.agent_id = agent_id.into();
        run.status = RunStatus::Completed;
        run.ended_at = Some(ended_at.into());
        run
    }

    fn snapshot(runs: Vec<Run>) -> BoardSnapshot {
        BoardSnapshot {
            runs,
            observing_since: "2026-10-01T00:00:00Z".into(),
        }
    }

    #[test]
    fn a_marked_finished_run_shows_as_running() {
        let input = snapshot(vec![
            finished("a", "2026-10-20T23:59:00Z"),
            finished("b", "2026-10-20T23:59:00Z"),
        ]);
        let shown = apply_pending(&input, |id| (id == "a").then(mark));
        assert_eq!(shown.runs[0].status, RunStatus::Running);
        assert_eq!(shown.runs[0].ended_at, None);
        let started = shown.runs[0].started_at.as_deref().unwrap();
        assert_eq!(
            chrono::DateTime::parse_from_rfc3339(started)
                .unwrap()
                .timestamp_millis(),
            mark()
        );
        assert_eq!(shown.runs[1], input.runs[1]);
    }

    #[test]
    fn a_failed_run_moves_too() {
        let mut run = finished("a", "2026-10-20T23:59:00Z");
        run.status = RunStatus::Failed;
        assert!(shows_pending(&run, mark()));
    }

    #[test]
    fn a_run_that_ended_after_the_mark_stays_finished() {
        let run = finished("a", "2026-10-21T00:00:05Z");
        assert!(!shows_pending(&run, mark()));
    }

    #[test]
    fn a_run_that_ended_at_the_mark_still_moves() {
        let run = finished("a", "2026-10-21T00:00:00Z");
        assert!(shows_pending(&run, mark()));
    }

    #[test]
    fn a_running_run_needs_no_overlay() {
        let mut run = finished("a", "2026-10-20T23:59:00Z");
        run.status = RunStatus::Running;
        run.ended_at = None;
        assert!(!shows_pending(&run, mark()));
    }

    #[test]
    fn an_unreadable_end_time_keeps_the_overlay() {
        assert!(shows_pending(&finished("a", "not a time"), mark()));
    }

    #[test]
    fn a_mark_without_a_run_adds_no_card() {
        let input = snapshot(vec![finished("a", "2026-10-20T23:59:00Z")]);
        let shown = apply_pending(&input, |id| (id == "missing").then(mark));
        assert_eq!(shown, input);
    }

    #[test]
    fn no_mark_returns_an_equal_snapshot() {
        let input = snapshot(vec![finished("a", "2026-10-20T23:59:00Z")]);
        assert_eq!(apply_pending(&input, |_| None), input);
    }
}
```

Anti-goal: Board redraws with no card change stay at 0 per `PendingTurnsChanged`, and marks add at most 1 `board.snapshot` request per Send; tripwire; read from the store's equality guard before `revision += 1`, the refresh rule in `store.rs`, and the test `no_mark_returns_an_equal_snapshot`, before the store change, after it, and at completion.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-plugin-board pending` -> the 8 red tests pass.
- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-plugin-board` -> all board tests pass.
- Read `store.rs` -> `interval` reads the shown snapshot, and `revision` grows only when the shown snapshot differs from the last one.
- Read `store.rs` -> `refresh` runs only when an agent newly shows as pending, never on a clear.
- Owner check on live GPUI (needs a human and a build with TASK-284): open a thread whose What next card can Send, click Send, and watch the Board open -> the card is in Running on the first frame, with no jump from Just finished.
- Owner check on live GPUI: send to a busy agent from What next -> the card goes back to Just finished when the send fails.
