# TASK-315 Mod+J cycles read Board cards too

Group: D (shared file packages/desktop-gpui/crates/app/src/shell.rs with TASK-312 to TASK-314)
Class: code

## Brief

Goal: Mod+J reaches every Board Just finished card, read ones included, and repeated taps cycle through them.

Change: the Mod+J list holds unread Board cards only -> it holds all non-running Board cards, unread first, rotated to start after the viewed thread.

Boundaries:

- Follows [ADR-0073](watchtower/decisions/ADR-0073-board-finished-cycle-over-unread-only.md), which supersedes ADR-0072.
- Order: unread agents first (`requires_attention` with reason `Finished` or `Error`), by `attention_at` ascending, then agent id. Read agents next, by the Board run `endedAt` descending; a run with no `endedAt` goes last; ties by agent id.
- Leave out subagents, agents with reason `Permission` or `pending_permissions > 0`, and agents that are not non-running Board runs.
- Rotation: build the full order including the viewed agent. The rows start at the entry after the viewed agent and wrap; the viewed agent is not a row. With the viewed agent not in the order, rows start at the first entry.
- The tap path opens the first row, so taps cycle.
- A read row shows the text "Read" in a muted color where an unread row shows "Finished" or "Error". Add `unread: bool` to `SwitcherRow`.
- With no rows, the toast is "No finished threads".
- `unread_queue` and After send Next unread stay unread-only. Do not change `after_send.rs`.
- Keep the async snapshot, the pending step count, the host-switch drop, and the Board-inactive toast from TASK-314.
- Trap: `board_finished_ids` returns only ids. Add `board_finished_runs(snapshot: &serde_json::Value) -> HashMap<String, Option<String>>` (agent id to `endedAt`) and build the id set from its keys, so both paths read one decode.

How:

- In `unread.rs`, add `board_finished_runs`, the field `ended_at: Option<String>` on `QueueEntry`, and `switch_cycle(entries: &[QueueEntry], viewed: Option<&str>) -> Vec<String>`.
- In `shell.rs`, build the Mod+J rows from `switch_cycle`; keep `unread_queue` for After send.
- In `unread_switcher.rs`, add `SwitcherRow::unread` and draw the "Read" label.

Files:

- [packages/desktop-gpui/crates/app/src/unread.rs](packages/desktop-gpui/crates/app/src/unread.rs) (decode, `ended_at`, `switch_cycle`, tests)
- [packages/desktop-gpui/crates/app/src/shell.rs](packages/desktop-gpui/crates/app/src/shell.rs) (Mod+J rows from the cycle, toast text)
- [packages/desktop-gpui/crates/app/src/unread_switcher.rs](packages/desktop-gpui/crates/app/src/unread_switcher.rs) (`unread` field, Read label)

Expected result:

- WHEN Hiep taps Mod+J with no unread cards and 3 read cards THE shell SHALL open the read card after the viewed one, wrapping at the end.
- WHEN Hiep taps Mod+J 3 times on a Board with 3 read cards THE shell SHALL visit each card once and return to the first.
- WHILE unread cards exist, WHEN Hiep taps Mod+J from a non-card view THE shell SHALL open the oldest unread card.
- WHEN the switcher shows THE read rows SHALL show "Read".
- IF the Board has no Just finished card other than the viewed one THEN Mod+J SHALL show "No finished threads".
- WHILE the mode is Next unread, WHEN a send is delivered THE shell SHALL still open only unread cards.

Red tests:

File: [packages/desktop-gpui/crates/app/src/unread.rs](packages/desktop-gpui/crates/app/src/unread.rs)

```rust
fn card(id: &str, unread: bool, at: Option<&str>, ended: Option<&str>) -> QueueEntry {
    QueueEntry {
        agent_id: id.into(),
        requires_attention: unread,
        reason: unread.then_some(AttentionReason::Finished),
        is_subagent: false,
        attention_at: at.map(String::from),
        on_board: true,
        ended_at: ended.map(String::from),
    }
}

#[test]
fn the_cycle_puts_unread_oldest_first_then_read_latest_first() {
    let entries = [
        card("r-old", false, None, Some("1")),
        card("u-new", true, Some("5"), Some("5")),
        card("r-new", false, None, Some("3")),
        card("u-old", true, Some("4"), Some("4")),
        card("r-none", false, None, None),
    ];
    assert_eq!(switch_cycle(&entries, None), ["u-old", "u-new", "r-new", "r-old", "r-none"]);
}

#[test]
fn the_cycle_starts_after_the_viewed_card_and_wraps() {
    let entries = [
        card("a", false, None, Some("3")),
        card("b", false, None, Some("2")),
        card("c", false, None, Some("1")),
    ];
    assert_eq!(switch_cycle(&entries, Some("b")), ["c", "a"]);
    assert_eq!(switch_cycle(&entries, Some("c")), ["a", "b"]);
    assert_eq!(switch_cycle(&entries, Some("other")), ["a", "b", "c"]);
}

#[test]
fn the_cycle_leaves_out_permission_subagents_and_off_board_agents() {
    let mut sub = card("sub", false, None, Some("1"));
    sub.is_subagent = true;
    let mut off = card("off", false, None, Some("1"));
    off.on_board = false;
    let mut perm = card("perm", true, Some("1"), Some("1"));
    perm.reason = Some(AttentionReason::Permission);
    let entries = [sub, off, perm, card("keep", false, None, Some("1"))];
    assert_eq!(switch_cycle(&entries, None), ["keep"]);
}

#[test]
fn only_the_viewed_card_gives_an_empty_cycle() {
    let entries = [card("a", false, None, Some("1"))];
    assert!(switch_cycle(&entries, Some("a")).is_empty());
}

#[test]
fn board_finished_runs_keep_ended_at() {
    let snapshot = serde_json::json!({"runs": [
        {"agentId": "a", "status": "completed", "endedAt": "2026-10-27T01:00:00.000Z"},
        {"agentId": "b", "status": "failed"},
        {"agentId": "c", "status": "running", "endedAt": "x"}
    ]});
    let runs = board_finished_runs(&snapshot);
    assert_eq!(runs.len(), 2);
    assert_eq!(runs["a"].as_deref(), Some("2026-10-27T01:00:00.000Z"));
    assert_eq!(runs["b"], None);
}
```

Add `ended_at: None` to the existing `entry` helper. Keep every existing assertion.

Anti-goal: the existing `unread` and `after_send` tests keep passing with no changed assertion, 0 failures; tripwire; read from `./cargo.sh test -p bachuc-app` before the change and at completion.

## Verify

- `cd packages/desktop-gpui && CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-app unread` -> all unread tests pass, including the 5 new ones.
- `cd packages/desktop-gpui && CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-app` -> all tests pass.
- `cd packages/desktop-gpui && CARGO_BUILD_JOBS=4 ./cargo.sh clippy -p bachuc-app --no-deps -- -D warnings` -> only the known `plugins.rs:10` error.
- Screen check (UI checker, live, check bundle): with 3 read Board cards and none unread, tap Cmd+J 3 times -> each card opens once, and the fourth tap returns to the first.
- Screen check: with 1 unread and 2 read cards, from the Board page tap Cmd+J -> the unread card opens first.
