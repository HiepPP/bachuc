# TASK-223 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/explorer/src/branch_switcher.rs`: pure rules `next_highlight`, `initial_highlight`, and `enter_row`, with 3 new tests. `Search` holds the highlight and a `ScrollHandle`. The key intercept takes Escape, Up, and Down while the filter has focus. Enter picks the highlighted row. The highlighted row has the `surface1` background and `aria_selected`, and scrolls into view. `has_branch` is gone.
- `packages/desktop-gpui/crates/explorer/src/explorer.rs`: `render_changes_header` always draws the switcher, which draws an empty element for a detached HEAD. The passive "Unknown" label, its `label` closure, and the `branch` parameter are gone.

## Contract

- No "Produces:" or "Consumes:" line. The explorer public API is unchanged; `has_branch` and `render_changes_header` were crate-private.
- Highlight rules are ports of Electron `getNextActiveIndex` (`combobox-keyboard.ts`) and `resolveInitialActiveIndex` (`combobox.tsx`, rows under the search). Up and Down wrap. With no highlight, Down goes to the first row and Up to the last.
- The highlight resets when the list opens, the query changes, and the branches load. With an empty query it starts on the current branch, else on the first row. So "Down, Down, Enter picks the third row" holds when the current branch is the first row.
- Enter picks the highlighted row, or the first row when there is no highlight. Arrow keys with modifiers are also taken, as in Electron's `dispatchDesktopKey`.
- The match tiers and the toast notices from TASK-222 are unchanged. No inline error came back.
- Deviation: the spec's Files: lists only `branch_switcher.rs`, but the "Unknown" label lives in `explorer.rs`. The Group line claims the explorer crate, so the builder edited `explorer.rs` to remove it.

## Verified

- `CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-explorer branch` -> 12 passed, including `branch_highlight_moves_and_wraps`, `branch_highlight_starts_on_current_branch_or_first_match`, and `branch_down_down_enter_picks_the_third_row`.
- `CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-explorer` -> 51 + 38 + 21 passed.
- `./cargo.sh fmt --check -p bachuc-explorer` -> exit 0.
- `CARGO_BUILD_JOBS=4 ./cargo.sh clippy -p bachuc-explorer --tests` -> exit 0, no warnings in the changed files.
- `./cargo.sh metadata --format-version 1` -> exit 0.
- `rg -c "branch_suggestions\(" crates/explorer/src` -> 1 call, still only in `BranchSwitcher::load`.
- GitNexus impact: the index is stale; `has_branch` and `render_changes_header` were not found. `rg` showed one caller each, both in `explorer.rs`.
- UI check, arrow keys move the highlight and Enter switches to it: DEFERRED (UI checker).
- UI check, detached HEAD shows no switcher and no label: DEFERRED (UI checker).
- Anti-goal, one `branch_suggestions_request` per open in the live `daemon.log`: DEFERRED (UI checker).

## Lessons

- The composer slash menu is the nearby pattern for list keys over an input: `intercept_keystrokes`, then `stop_propagation` when the list takes the key, plus `ScrollHandle::scroll_to_item` on a `track_scroll` list.
