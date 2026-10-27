# TASK-314 Unread queue only from Board cards

Group: D (shared file packages/desktop-gpui/crates/app/src/shell.rs with TASK-312 and TASK-313)
Class: code

## Brief

Goal: Mod+J and After send Next unread list only unread threads that are Just finished cards on the Board, so old removed threads never show.

Change: the queue holds every unread agent of the host -> it also requires a Board run with a status other than `running`.

Boundaries:

- Follows [ADR-0072](watchtower/decisions/ADR-0072-board-cards-only-unread-queue.md), which supersedes ADR-0070.
- Read Board runs with `Client::invoke_plugin_rpc("board", "board.snapshot", json!({}))`. The answer is `{ runs: [{ agentId, status, ... }], observingSince }`; `status` is one of `running`, `completed`, `failed`, `cancelled`, `unknown`. Decode only `agentId` and `status`. Skip a run that does not decode.
- Keep the client attention filter, the FIFO sort, and the subagent, permission, and viewed-agent rules of `unread_queue`. The Board set is one more filter.
- Call the snapshot at each Mod+J that opens the switcher, and at each `Delivered` in Next unread mode. Cache nothing.
- Trap: the call is async. When the answer comes, read `window.modifiers()`. If Cmd is no longer held, open the oldest thread at once with no popover (a tap). If Cmd is held, open the switcher with that state. Count Mod+J and Mod+Shift+J presses that arrive while the call is pending, and apply them as `step` calls once the switcher opens. A second Mod+J while a call is pending must not start a second call.
- Trap: in `after_send`, `decide_after_send` runs after the snapshot answer. Recompute `still_viewed` at that time, so a thread left during the call does not move.
- If the Board plugin is inactive (`PluginHost::is_active("board")` is false) or the call fails: Mod+J shows the toast "Board is not installed" or the error text, and opens nothing. After send keeps the existing `BoardMissing` and `OpenBoard` steps.
- The empty-queue toast stays "No unread threads".
- Do not change the Board plugin, the protocol, or the server.

How:

- In `unread.rs`, add `board_finished_ids(snapshot: &serde_json::Value) -> HashSet<String>` and add `on_board: bool` to `QueueEntry`; `unread_queue` keeps only `on_board` entries.
- In `shell.rs`, fetch the snapshot before `open_unread_switcher` and before the Next unread step of `after_send`; pass the set into `unread_queue_ids` and `next_unread`.

Files:

- [packages/desktop-gpui/crates/app/src/unread.rs](packages/desktop-gpui/crates/app/src/unread.rs) (Board set decode, `on_board` field, tests)
- [packages/desktop-gpui/crates/app/src/shell.rs](packages/desktop-gpui/crates/app/src/shell.rs) (async snapshot before the switcher and Next unread)

Interfaces:

- Consumes: `Shell::open_next_unread(&mut self, exclude: Option<&str>, window: &mut Window, cx: &mut Context<Shell>) -> bool` from TASK-312. Its signature may gain the Board set; TASK-313 is its only caller.

Expected result:

- WHEN Hiep presses Mod+J THE switcher SHALL list only unread finished or error agents that are non-running Board runs.
- IF an unread agent is not a Board run, or its run is `running` THEN THE queue SHALL leave it out.
- IF the Board plugin is inactive THEN Mod+J SHALL show "Board is not installed" and open nothing.
- IF the snapshot call fails THEN Mod+J SHALL show the error toast and open nothing.
- WHEN Cmd is already up as the snapshot answer comes THE shell SHALL open the oldest queued thread with no popover.
- WHILE the mode is Next unread, WHEN a send is delivered THE shell SHALL open the oldest other queued Board thread, or the Board when none.

Red tests:

File: [packages/desktop-gpui/crates/app/src/unread.rs](packages/desktop-gpui/crates/app/src/unread.rs)

```rust
#[test]
fn board_finished_ids_keep_non_running_runs_and_skip_bad_rows() {
    let snapshot = serde_json::json!({
        "observingSince": "2026-10-27T00:00:00.000Z",
        "runs": [
            {"agentId": "done", "status": "completed"},
            {"agentId": "failed", "status": "failed"},
            {"agentId": "busy", "status": "running"},
            {"status": "completed"},
            "bad"
        ]
    });
    let ids = board_finished_ids(&snapshot);
    assert!(ids.contains("done") && ids.contains("failed"));
    assert!(!ids.contains("busy"));
    assert_eq!(ids.len(), 2);
    assert!(board_finished_ids(&serde_json::json!({})).is_empty());
}

#[test]
fn an_unread_agent_off_the_board_stays_out() {
    let mut off = entry("off", Some(AttentionReason::Finished), Some("1"));
    off.on_board = false;
    let entries = [off, entry("on", Some(AttentionReason::Finished), Some("2"))];
    assert_eq!(unread_queue(&entries, None), ["on"]);
}
```

Update the existing `entry` helper in the tests to set `on_board: true`.

Anti-goal: the 7 existing queue and history tests and the 4 `after_send` tests keep passing with no changed assertion, 0 failures; tripwire; read from `./cargo.sh test -p bachuc-app unread` and `./cargo.sh test -p bachuc-app after_send` before the change and at completion.

## Verify

- `cd packages/desktop-gpui && CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-app unread` -> 9 tests pass.
- `cd packages/desktop-gpui && CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-app` -> all tests pass.
- `cd packages/desktop-gpui && CARGO_BUILD_JOBS=4 ./cargo.sh clippy -p bachuc-app --no-deps -- -D warnings` -> only the known `plugins.rs:10` error.
- Screen check (UI checker, live, check bundle): with old unread agents not on the Board and 2 unread Board cards, open the switcher from the command center item "Open unread thread" -> only the 2 Board cards are listed, oldest first.
- Screen check: remove one of those cards on the Board, then press Cmd+J -> the removed thread does not open and is not listed.
- Screen check: mode Next unread, send a prompt -> the next unread Board card opens, never a thread off the Board.
