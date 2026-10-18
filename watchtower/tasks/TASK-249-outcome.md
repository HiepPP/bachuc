# TASK-249 Outcome

## Status

DONE (code). The UI checker lines are DEFERRED (UI checker).

## Changed

- `crates/explorer/src`: `ui.rs`, `file_view.rs`, `git_actions_ui.rs`, `branch_switcher.rs`, `diff_view.rs`, `explorer.rs`, `pull_request_view.rs`.
- `crates/terminal/src/view.rs`.
- `crates/history/src/screen.rs`.
- `crates/settings/src`: `ui.rs`, `settings.rs`, `appearance_preview.rs`, `pane_appearance.rs`, `pane_hosts.rs`, `pane_projects.rs`, `pane_providers.rs`, `pane_shortcuts.rs`.

## Contract

Every fixed `px(X)` of paddings, gaps, icons, control sizes, radii, menu and popup widths became `bachuc_theme::zoomed(X, cx)`. No new constants, no renames.

Private helpers gained a trailing `cx: &App` (or the zoom factor) because they had no `cx`:

- `explorer/ui.rs`: `file_icon_element`, `chevron`, `loading_marker`, `change_icon`.
- `explorer/branch_switcher.rs`: `popup`.
- `explorer/explorer.rs`: `file_menu_item`.
- `explorer/pull_request_view.rs`: `spinner`, `disclosure`, `check_icon`, `checks_ring`, `skeleton_bar`, `pane_skeleton`, `activity_skeleton`.
- `terminal/view.rs`: `focus_ring`.
- `history/screen.rs`: `provider_icon`.
- `settings/ui.rs`: `column`.
- `file_view.rs` `gutter_text_width` takes `zoom: f32`.

All callers are inside the same crate (the `ui` module is private). I confirmed this with `rg`; the GitNexus impact call was not run for these private helpers.

Behavior additions beyond a plain swap:

- `DiffView` has a `zoom` field. `render` rebuilds the list heights and the gutter when the zoom changes, because the uniform list keeps the old row height. Line height is `LINE_HEIGHT * zoom`, the code left padding is scaled, and the gutter's 12 px padding is scaled.
- `file_view.rs` scales the gutter pads while the digit width keeps following the zoomed code font.
- Menu closures that run without a captured `cx` (`PopupMenu::build`, `dropdown_menu_with_anchor`) read the `App` argument or a value computed before the closure.

## Verified

Group-level runs (one run, recorded for TASK-249 as the only task of group E):

- `./cargo.sh test -p bachuc-explorer -p bachuc-terminal -p bachuc-history -p bachuc-settings` -> exit 0, all suites ok (output `/tmp/e249-test.txt`).
- `./cargo.sh build -p bachuc-app` -> exit 0, no warning in changed files.
- `./cargo.sh fmt -p <crate>` then `fmt --check` for explorer, terminal, history, settings -> exit 0.
- Remaining `px(` sites: 34 (rg `(?<![\w.])px\(` minus `zoomed`):
  - 7 hairlines or 1 px nudges: `git_actions_ui.rs`, `explorer.rs`, `pull_request_view.rs`, `settings/ui.rs`, `terminal/view.rs`, `terminal/emulator.rs` x2.
  - 5 zero values `px(0.)`: `explorer.rs` x2, `diff_view.rs` x3.
  - 11 already zoomed: terminal `self.size`, `line_height`, `typeface.size`, wheel delta, find bar line height (6); `file_view.rs` line height and gutter (2); `diff_view.rs` zoomed line height, wheel delta, gutter (3).
  - 7 derived from a zoomed size: the PR checks ring in `pull_request_view.rs`.
  - 1 sideways scroll offset the user set: `diff_view.rs` `px(-offset)`.
  - 1 `PADDING` of the terminal pane. It is `0.0` and the grid math in `size.rs` uses it, so it stays `px`.
  - 2 `OVERDRAW` list render margins (`diff_view.rs`, `screen.rs`): a perf margin, not a visual size.

## Anti-goal

DEFERRED (UI checker): the 100% screenshot comparison. At zoom 1.0, `zoomed(X, cx) == px(X)`, so the code path gives the same sizes by construction.

## Lessons

- `/tmp` is shared between parallel builders: another builder overwrote a helper script with the same name. Use a task-specific script name.
- A `px(..)` swap inside a `move |menu, _, _|` closure of `PopupMenu::build` or `dropdown_menu_with_anchor` cannot capture `cx` (borrow escapes). Name the third closure argument and use it, or compute the zoomed value before the closure. A nested `PopupMenuItem::element(move |_, cx| ..)` has its own `cx`.
- A uniform `ListState` keeps the row height it was built with. A view that uses `reset_with_uniform_height` with a zoomed height must rebuild the list when the zoom changes (`DiffView.zoom`).
- A terminal `PADDING` change needs `size.rs` (`grid_size`, `cell_at`) to follow; it is zero today.
