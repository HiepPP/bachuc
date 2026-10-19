# Learn 20261019-gpui-text-shortcuts-sidebar-footer

## Summary

Discrepancy: 4 found. All 8 TASKs shipped. Two specs had a wrong check, one TASK shipped a bug that a later screen check caught, and screen checks often ran against a window that did not redraw.

## Per TASK

- TASK-256: match. The owner still needs to compare blur by eye; the owner's `appearance.json` sets `uiFont` to "Be Vietnam Pro", which can also make GPUI differ from Electron.
- TASK-257: plan: native menu bar with working Window items -> shipped: menus, but Cmd+M, Window > Zoom, and Full Screen did nothing. Mistake: `on_main_window` updated the window inside its own action dispatch, and `let _ =` hid the error. Fix: defer with `cx.defer` (commit `928fb05e2`). Its Verify line on duplicate keystrokes could not pass on correct code and was rewritten.
- TASK-258: match. The builder also edited `lib.rs` and the sidebar `Cargo.toml`, which the spec's Files list missed.
- TASK-259: match. Adding fields to `WorkspaceNode` broke struct literals in 7 files outside Files; `Default` plus `..Default::default()` fixed them.
- TASK-260: plan: `rewind(&self, ..., cx: &mut App)` -> shipped `rewind(&mut self, ..., cx: &mut Context<Client>)`, which follows the fork pattern. Mistake: the spec guessed the signature. Fix: copy the signature of the pattern the spec names.
- TASK-261: match, with gpui-kit limits: values read "Grouping: Project", and Show toggles close the menu.
- TASK-262: match in tests. PR, Checks, and Labels items were never seen on screen, because live has no such data.
- TASK-263: match. The owner passed the hand check.

## Plan-Level

- Not in plan: the Board repo link did nothing in GPUI, because `open_new_workspace` did not close the page surface. Fixed in `1a0fae39d`.
- Screen checks were slow and misleading. GPUI draws no frames while another window covers the check window, so the AX tree and the pixels disagree, and scroll does nothing. A first "menu Cut fails" result was a stale AX read.
- Shell loops with `set -- $x` on a quoted word created stray files such as `TASK-261 Q-171-outcome.md`; one was committed and later removed.

## Lessons

- Before a screen check, make sure the check window is in front and uncovered, or have the owner check by hand with prepared mock data.
- After a menu action, press one harmless key before reading AX values.
- Never hide a window `update` error with `let _ =`; defer window calls made from action handlers.
- Copy function signatures from the pattern a spec names instead of guessing them.
