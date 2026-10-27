# TASK-312 outcome

Status: DONE (Screen checks DEFERRED (UI checker))

## Changed

- `packages/desktop-gpui/crates/app/src/unread.rs` (new): `QueueEntry`, `unread_queue`, `ViewHistory`, `iso_to_ms`, the 7 spec tests.
- `packages/desktop-gpui/crates/app/src/unread_switcher.rs` (new): `UnreadSwitcher`, `SwitcherRow`, `UnreadSwitcherEvent`.
- `packages/desktop-gpui/crates/app/src/shell.rs`: `unread` slot, `view_history`, actions `agent.unread.switcher` and `agent.previous`, `open_next_unread`, `open_unread_switcher`, `open_previous_agent`, `sync_unread_switcher`; `reconnect` clears both; `sync_viewed_agent` calls `visit`; Escape in `on_key_capture` skips the page close while the switcher shows.
- `packages/desktop-gpui/crates/app/src/main.rs`: module declarations.
- `packages/desktop-gpui/crates/command/src/shortcuts.rs`: two `DEFAULTS` rows in `Workspaces`.
- `actions.rs`: not changed (the shell context already covers both actions).

## Contract

- `Shell::open_next_unread(&mut self, exclude: Option<&str>, window: &mut Window, cx: &mut Context<Shell>) -> bool` (`pub(crate)`). Opens the oldest queue entry other than `exclude`; `false` with no toast when none.
- `unread_queue(&[QueueEntry], Option<&str>) -> Vec<String>`; `ViewHistory::{visit, back}` as specified.
- Switcher: holds the modifiers of the opening press (`window.modifiers()`, Cmd/Ctrl/Alt only). It draws after 150 ms; release of a held modifier opens the selected row. With no modifier held it draws at once.
- The age is `compact_time_ago` of the ISO time, parsed by `iso_to_ms` (only the `...Z` form; else no age).

## Verified

- Red: before the code, `cargo test -p bachuc-app unread` failed to compile (missing `QueueEntry`, `unread_queue`, `ViewHistory`).
- `./cargo.sh test -p bachuc-app unread` -> 7 passed.
- `./cargo.sh test -p bachuc-app -p bachuc-command` -> 106 + 43 + 5 + 10 passed, 0 failed, including `every_assigned_shortcut_binds_and_digits_bind_nine_tabs`. Group-level run (shared with TASK-313).
- `./cargo.sh fmt --check -p bachuc-app -p bachuc-command` -> exit 0.
- Clippy `-p bachuc-app --no-deps -D warnings`: one error, `type_complexity` at `plugins.rs:10` (commit 33e7400add, known failure). No lint from the new files.
- `diff` of `shortcuts.rs` before/after: 12 lines added, 0 removed; only the two `Workspaces` rows.
- Screen checks: DEFERRED (UI checker).
- GitNexus impact: `sync_viewed_agent` HIGH (3 direct, 18 total, 0 flows); my edit there is one line after the early return. `run_action`, `open_agent_pane`, `reconnect`: LOW.

## Anti-goal

0 changed or lost shortcut bindings (PASS).

- Before: `DEFAULTS` copy at `/tmp/t312-shortcuts-before.rs`, read before any edit.
- After the shortcut rows: diff vs the copy is +12/-0, only the two new rows.
- Completion: same diff after `fmt`; `every_assigned_shortcut_binds_and_digits_bind_nine_tabs` passes.

## Lessons

- `iso_to_ms` has no test in the spec; the age text is unverified until a screen check.
- Cmd release is read from `window.modifiers()` at open. If a UI check shows the popover never closing on release, check that `on_modifiers_changed` reaches the focused root.

## UI Checks

Run 2026-10-27 by the UI checker (check bundle on live 6768, `cua_repl`, AX actions and background key events only). Live held about 30 older unread threads from earlier runs, so the first Cmd+J taps drained them oldest first before the uichk agents came up.

- PASS - 2 unread, focus a thread composer, tap Cmd+J: the oldest unread thread opened each tap in FIFO order; `uichk-U1` (older) opened before `uichk-U2`.
- UNVERIFIED - hold Cmd, press J twice, release Cmd: AX and key events cannot hold Cmd or release it later (`pressKey("super+j")` presses and releases at once, so the thread opens as a quick tap). Partial evidence with no Cmd held (switcher opened from the command center item "Open unread thread", 3 rows `uichk-U6/U7/U8`, each with workspace, "Finished", and age): Cmd+J moved the selection to row 2 then row 3, no thread opened, Enter opened the selected row (`uichk-U7`). Release-Cmd-opens and the 150 ms draw delay are not covered.
- PASS - Cmd+J with no unread agents: toast "No unread threads", view unchanged (screenshot).
- PASS - Cmd+Shift+J: opened the thread left last (U2 -> U1); a second Cmd+Shift+J returned (U1 -> U2).
- PASS - thread A, open the Board, Cmd+Shift+J: thread A (`uichk-U2`) opened.
- UNVERIFIED - hold Cmd, J then Shift+J: same held-modifier limit. Partial evidence with no Cmd held: in the open switcher Cmd+J moved down (U6 -> U7 -> U8), Cmd+Shift+J moved up (U8 -> U7), no thread opened until Enter.
- PASS (caveat) - 1 unread that is the viewed thread, Cmd+J: toast "No unread threads" with `uichk-U5` viewed after it finished. The daemon flag on the viewed agent was not read, so the client may have cleared it before the key.
- PASS - open the switcher, archive a listed agent from the CLI: the row `uichk-U8` dropped at once (3 rows -> 2 rows, screenshots); Esc closed it and the view stayed `uichk-U7`. The switcher was opened from the command center item, not by a held Cmd.
- PASS - open the switcher, Cmd+` : the host changed to macmini and the switcher closed. Cmd+` pressed back to live. The switcher was opened from the command center item.
- Anti-goal (shortcut chords): not run by the UI checker; no existing chord misfired (Cmd+K, Cmd+`, Escape, Return all worked in this run).

## Lead Record

Status: BLOCKED

Blocked: BUILT, UNVERIFIED (Q-196). The held-Cmd release and the 150 ms draw delay need Hiep's hand check. Code is committed in 28ecb4a64.

## Owner Check

Status: DONE

- Q-196 ANSWERED on 2026-10-27: Hiep held Cmd, pressed J twice, and released Cmd in Bachuc GPUI Check. PASS.
