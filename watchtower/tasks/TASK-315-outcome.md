# TASK-315 outcome

Status: DONE (screen checks DEFERRED to the UI checker)

## Changed

- `packages/desktop-gpui/crates/app/src/unread.rs`: `board_finished_runs`, `QueueEntry::ended_at`, `switch_cycle`; `board_finished_ids` now reads its keys; 5 red tests added, `entry` helper has `ended_at: None`.
- `packages/desktop-gpui/crates/app/src/shell.rs`: `queue_entries` (shared), `switch_cycle_ids` (Mod+J), `unread_rows` sets `unread`, toast "No finished threads". `unread_queue_ids` and After send are unchanged in behavior.
- `packages/desktop-gpui/crates/app/src/unread_switcher.rs`: `SwitcherRow::unread`, "Read" label in muted color.

## Contract

- `board_finished_runs(&serde_json::Value) -> HashMap<String, Option<String>>`.
- `switch_cycle(&[QueueEntry], Option<&str>) -> Vec<String>`.
- `SwitcherRow { unread: bool }`.
- `pending_permissions > 0` is folded into the entry as reason Permission in `Shell::switch_cycle_ids` only, so After send is untouched.

## Verified

- Red: before the change the unread tests failed to compile (`ended_at`, `switch_cycle` missing).
- `./cargo.sh test -p bachuc-app`: 113 passed, 0 failed (group-level run). Baseline before the change: 108 passed.
- `./cargo.sh clippy -p bachuc-app --no-deps -- -D warnings`: only the known `plugins.rs:10` error.
- `./cargo.sh fmt --check -p bachuc-app`: exit 0.
- Screen checks: DEFERRED (UI checker).

## Anti-goal

Existing `unread` and `after_send` tests: 0 failures, no assertion changed. Before: 108 pass (2026-10-27, before edits). After: 113 pass (108 + 5 new).

## Lessons

- GitNexus impact returned UNKNOWN for `QueueEntry` and `SwitcherRow` (not indexed); `rg` shows them used only in `unread.rs`, `shell.rs`, `unread_switcher.rs`.
- Switcher aria labels still say "Unread threads".

## UI Checks

Run 2026-10-27 by the UI checker (check bundle rebuilt at 5c774634c on live 6768, `cua_repl`, AX actions and background key events only; `End` key press forces a redraw before each AX read). Agents `t315-R1..R3`, `t315-U1..U3` (mock/ten-second-stream) were made for the run and archived after it.

Board cleanup: live held 7 read Just finished cards (`skill demo`, `branch menu check`, `Hi.`, and 4 x `Reply with the single word ok.`). All 7 were hidden with the Board Remove button to get a clean Board. No agent was deleted or archived; a card returns on its next turn.

- PASS - 3 read cards (`t315-R3`, `R2`, `R1`, all "seen"), none unread, from the Board tap Cmd+J 4 times: opened R3, R2, R1, then R3 again (each once, fourth returns to the first).
- PASS - 1 unread (`t315-U3`) and 2 read (`t315-U1`, `t315-R3`) on the Board, tap Cmd+J: `t315-U3` opened first.
- PASS - 2 unread `t315-U1` (older), `t315-U2` and 1 read `t315-R3`, from the Board: tap 1 opened U1, tap 2 opened U2 (not R3), tap 3 opened U1 (read by then, ahead of R3 in the Read order after viewing U2). Caveat: the pure read card R3 was not reached in this run; tap 3 landed on U1, which was already read. Tap 2 reaching U2 rather than R3 shows the viewed U1 still counted as unread at the snapshot.
- UNVERIFIED - the "Read" label and the "No finished threads" toast in the switcher list: the list needs Cmd held, which AX and key events cannot do. Taps open at once with no list.

Restored: `sidebar.json`, `prompt-history.json`; `agent-defaults.json` unchanged; `after-send.json` was absent and is still absent. The check bundle stays open in the background.

## Lead Record

Status: BLOCKED

Blocked: BUILT, UNVERIFIED (Q-196). The Read label and the "No finished threads" toast show only in the list, which needs Cmd held. The tap cycle and unread-first order passed on live. The lead fixed the cycle so a just-read card no longer pushes unread cards to the end (test `unread_cards_lead_even_after_the_viewed_card_turns_read`). Code is in 5c774634c.
