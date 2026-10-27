# TASK-314 Outcome

Status: DONE (code checks; screen checks DEFERRED (UI checker))

## Changed

- `packages/desktop-gpui/crates/app/src/unread.rs`: `board_finished_ids`, `QueueEntry.on_board`, `unread_queue` filter, 2 red tests, `entry` helper sets `on_board: true`.
- `packages/desktop-gpui/crates/app/src/shell.rs`: async `board.snapshot` before the switcher and before the Next unread step of After send.

## Contract

- `pub fn board_finished_ids(snapshot: &serde_json::Value) -> HashSet<String>`; `QueueEntry.on_board: bool`.
- `Shell::open_next_unread(&mut self, exclude: Option<&str>, board: &HashSet<String>, window, cx) -> bool`.
- Mod+J: `open_unread_switcher` starts the call and sets `unread_pending = Some(vec![])`; Mod+J and Mod+Shift+J presses meanwhile push to it (no second call). `show_unread_switcher` takes the steps; Cmd/Ctrl/Alt all up -> opens the oldest thread with no popover; else opens the switcher and applies the steps.
- Board inactive: toast "Board is not installed", nothing opens. Call error: toast with the error text. Empty queue: "No unread threads".
- After send: Next unread calls the snapshot, then `finish_after_send` recomputes `still_viewed` and the queue. A failed call gives an empty set, so the Board opens (or `BoardMissing`). Stay and Board modes call nothing.

## Verified

- Red: before the code, `cargo test -p bachuc-app unread` failed to compile (missing `on_board`, `board_finished_ids`).
- Baseline before: 8 tests match `unread` (7 queue/history + 1 after_send name), 4 `after_send`: all pass.
- After: `test -p bachuc-app` -> 108 passed, 0 failed (includes the 2 new tests, 4 `after_send`, 7 old queue/history, no assertion changed).
- `fmt --check -p bachuc-app` -> exit 0.
- Clippy `--no-deps -D warnings` -> only the known `plugins.rs:10` `type_complexity` error.
- Screen checks (3): DEFERRED (UI checker).

## Anti-goal

Tripwire not crossed. Checkpoints (`cargo.sh test -p bachuc-app unread` / `after_send`): before 8 ok / 4 ok; at completion 10 ok (8 + 2 new) / 4 ok; 0 failures.

## Lessons

- `bachuc-app` has no `anyhow` or `serde` dependency; `board_snapshot` maps the error to `String` via `cx.spawn`.
- GitNexus has no index entry for these symbols (stale index); callers confirmed by `rg`: all in `shell.rs`.
- The held test for a tap is `platform || control || alt` (same as `UnreadSwitcher::new` hold).

## UI Checks

Run 2026-10-27 by the UI checker (rebuilt check bundle at e4791f21d on live 6768, `cua_repl`, AX actions and background key events only). Agents `t314-A` to `t314-F` (mock/ten-second-stream) were made for the run and archived after it. Board cards of those agents were seen in the Board as "Just finished, new". The AX tree of the background window updates one frame behind, so a key press (`End`) was used to force a read.

- UNVERIFIED (list view) / PASS (queue order) - "Open unread thread" from the command center with 2 new Board cards (`t314-A` older, `t314-B`): the item opened `t314-A` at once with no popover, as the code does when Cmd is up as the snapshot answers (`show_unread_switcher`). The switcher list needs Cmd held, which AX and key events cannot do, so "only the 2 Board cards are listed" was not seen. Partial evidence: the next Cmd+J tap opened `t314-B`, the third showed "No unread threads". Live still held older unread agents off the Board (per the lead; not readable from the CLI), and none of them opened.
- PASS - cards `t314-C` (older) and `t314-D` unread; removed `t314-C` through its Remove button on the Board; Cmd+J (tap) opened `t314-D`, not `t314-C`; the next Cmd+J showed "No unread threads". `t314-C` stayed unread on the daemon and never opened or showed.
- PASS - `after-send.json` set to `next-unread`, unread Board cards `t314-E` (older) and `t314-F`, viewing `t314-D`: send "hello again" opened `t314-E`; send "second" from `t314-E` opened `t314-F`. The off-Board unread agent `t314-C` (older than both) never opened. Not run: the send with no other unread Board card ("or the Board when none").

Restored: `sidebar.json`, `prompt-history.json`; `agent-defaults.json` and `appearance.json` were unchanged; `after-send.json` was absent at the start and its file was removed. The check bundle stays open in the background.

## Lead Record

Status: BLOCKED

Blocked: BUILT, UNVERIFIED (Q-196). The list view needs Cmd held. The tap path and Next unread passed on live. Code is committed in e4791f21d. The lead also dropped a Board answer that arrives after a host switch.
