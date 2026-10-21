# TASK-282 Client pending turn mark

Group: A (writes `packages/desktop-gpui/crates/client/src/pending.rs` and `lib.rs` only)
Class: code

## Brief

Goal: The GPUI `Client` keeps a client-only "turn pending" mark per agent, so plugins can show a sent prompt before the daemon confirms it.

Change: no shared pending state -> `Client::mark_turn_pending`, `clear_turn_pending`, `pending_turn`, and a `PendingTurnsChanged` event.

Boundaries:

- Follows [ADR-0058](watchtower/decisions/ADR-0058-gpui-pending-turn-mark-over-waiting-for-the-board-poll.md): the mark lives only in client memory, with a 15,000 ms limit.
- The mark is not agent status. Do not change `AgentSummary.status`, the sidebar, or any daemon message.
- Do not add a `ClientEvent` variant. Several crates match `ClientEvent`, so a new variant touches files outside this group. Use a separate event type with its own `EventEmitter`, as `DirectoryEvent` does.
- Times are UNIX milliseconds from the wall clock, so they compare with the Board's `now_ms` in [text.rs](packages/desktop-gpui/plugins/board/src/model/text.rs).
- Trap: `publish` in `lib.rs`: it drains core events. The pending event does not come from `ClientCore`, so emit it straight from the new `Client` methods.

How:

- Add `PendingTurns` and `PENDING_TURN_TTL_MS` in a new `pending.rs`. `mark` prunes expired marks; `get` hides a mark at or past the limit.
- Add a `pending_turns` field to `Client` in `lib.rs`, the three methods, and `impl EventEmitter<PendingTurnsChanged> for Client`.
- Re-export `PendingTurnsChanged` and `PENDING_TURN_TTL_MS` from `lib.rs`.

Files:

- [packages/desktop-gpui/crates/client/src/pending.rs](packages/desktop-gpui/crates/client/src/pending.rs) (new: `PendingTurns`, `PendingTurnsChanged`, `PENDING_TURN_TTL_MS`, unit tests)
- [packages/desktop-gpui/crates/client/src/lib.rs](packages/desktop-gpui/crates/client/src/lib.rs) (module, field, methods, emitter, re-exports)

Interfaces:

- Produces: `bachuc_client::PendingTurnsChanged` (a unit struct event, `Clone, Copy, Debug, PartialEq, Eq`) emitted by `Client` for TASK-283
- Produces: `bachuc_client::PENDING_TURN_TTL_MS: i64` (15_000) for TASK-283
- Produces: `Client::mark_turn_pending(&mut self, agent_id: &str, cx: &mut Context<Client>)` for TASK-284
- Produces: `Client::clear_turn_pending(&mut self, agent_id: &str, cx: &mut Context<Client>)` for TASK-283 and TASK-284
- Produces: `Client::pending_turn(&self, agent_id: &str) -> Option<i64>` (the mark time in UNIX ms) for TASK-283

Expected result:

- WHEN a caller marks an agent THE `Client` SHALL return the mark time from `pending_turn` and emit `PendingTurnsChanged`.
- WHEN 15,000 ms pass after the mark THE `Client` SHALL return `None` from `pending_turn`.
- WHEN a caller clears a marked agent THE `Client` SHALL drop the mark and emit `PendingTurnsChanged`.
- IF a caller clears an agent with no mark THEN THE `Client` SHALL emit nothing.
- WHEN a caller marks an agent again THE `Client` SHALL keep the newer mark time.

Red tests:

File: [packages/desktop-gpui/crates/client/src/pending.rs](packages/desktop-gpui/crates/client/src/pending.rs)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_000_000;

    #[test]
    fn a_mark_shows_until_the_limit() {
        let mut marks = PendingTurns::default();
        marks.mark("a", NOW);
        assert_eq!(marks.get("a", NOW), Some(NOW));
        assert_eq!(marks.get("a", NOW + PENDING_TURN_TTL_MS - 1), Some(NOW));
        assert_eq!(marks.get("a", NOW + PENDING_TURN_TTL_MS), None);
        assert_eq!(marks.get("b", NOW), None);
    }

    #[test]
    fn clear_reports_whether_a_mark_was_removed() {
        let mut marks = PendingTurns::default();
        assert!(!marks.clear("a"));
        marks.mark("a", NOW);
        assert!(marks.clear("a"));
        assert_eq!(marks.get("a", NOW), None);
    }

    #[test]
    fn a_new_mark_replaces_the_old_time() {
        let mut marks = PendingTurns::default();
        marks.mark("a", NOW);
        marks.mark("a", NOW + 5_000);
        assert_eq!(
            marks.get("a", NOW + PENDING_TURN_TTL_MS + 1),
            Some(NOW + 5_000)
        );
    }
}
```

Anti-goal: `ClientEvent` keeps its current variants, so no other crate changes for this TASK; tripwire; read from `git diff main -- packages/desktop-gpui/crates/client/src/model.rs` before the change, after the `lib.rs` change, and at completion.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-client pending` -> the 3 red tests pass.
- `cd packages/desktop-gpui && ./cargo.sh check -p bachuc-client` -> exit 0 with no new warning.
- Read `clear_turn_pending` in `lib.rs` -> it emits `PendingTurnsChanged` only when `PendingTurns::clear` returns `true`.
- `git diff main -- packages/desktop-gpui/crates/client/src/model.rs` -> empty (anti-goal).
