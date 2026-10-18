# TASK-232 Outcome

## Status

DONE, with UI checker lines DEFERRED. The horizontal root cause is proven from source. The vertical root cause is not proven from source; see Vertical below.

## Changed

- `packages/desktop-gpui/crates/timeline/src/card_view.rs`: the question option text column, plan card title, question tab and tab header, and permission action button and label get `min_w_0` (tabs and buttons also `max_w_full`; the tab header truncates).
- `packages/desktop-gpui/crates/timeline/src/lib.rs`: the row content column, todo task text, and notification message get `min_w_0`. New `pub fn content_column_width(row_width: f32) -> f32` and the constants `ROW_PADDING` and `COLUMN_PADDING` that the row now uses.
- `packages/desktop-gpui/crates/timeline/tests/layout.rs`: new tests for the width rule.
- `packages/desktop-gpui/crates/settings/src/pane_projects.rs`, `packages/desktop-gpui/plugins/board/src/runs.rs`, `packages/desktop-gpui/plugins/workspace-spaces/src/page.rs`: one `min_w_0` each.

## Contract

- Produces `bachuc_timeline::content_column_width(row_width: f32) -> f32`: the row width less `2 * ROW_PADDING`, clamped to `0..=MAX_CONTENT_WIDTH`, less `2 * COLUMN_PADDING`, never below 0.
- No other public names changed. ADR-0039 holds: no new remeasure call, no per-frame measure of all rows.

## Root causes

### Horizontal (AskUserQuestion option descriptions cut at the right edge)

- GPUI text answers a min-content probe with its unwrapped line width: `wrap_width` is `None` unless the width is definite (`gpui-pre-0.3.9/src/elements/text.rs:658`).
- Taffy sets a flex item's automatic minimum width to that min-content size. It clamps it only by a definite `width` (`taffy-0.13.0/src/compute/flexbox.rs:833`).
- The option text column was `flex_1` with no `min_w_0` and no definite width (`card_view.rs:649` before the fix). So its minimum was the full description on one line, and it ran past the card. `min_w_0` lets it shrink, and the final layout then wraps the text at the definite width.

### Vertical (the last bullet draws over the Recap card)

- Disproved: a list row height cached from an older, wider width. The GPUI list drops every cached height when its width changes (`gpui-pre-0.3.9/src/elements/list.rs:1543`), and it lays out each visible row again on every layout (`list.rs:1073`). Rows are placed by the heights measured in the same pass (`list.rs:1119`, `1138`). A stale list height cannot shift a visible row.
- The Recap card is a plugin item inside the same row as the message (`plugin_view.rs:261`). So the overlap is text painted past its own box inside one row, not two rows that overlap.
- Not found by reading: a path where a text box gets a height measured at a different width than the one it paints at. Taffy calls the leaf measure at the final width in the perform pass (`taffy-0.13.0/src/compute/leaf.rs:135`). The gpui-base flow and inline caches are keyed by wrap width (`vendor/gpui-base/src/text/inline_flow.rs:124`, `inline.rs:244`).
- Fix applied on the closest real fault: the row content column had no `min_w_0`. Each layout ran a min-content probe through every visible row's text, and that probe uses the cross-frame text cache, which answers it with the last frame's size (`text.rs:695`). After this fix, no probe runs from the row column. Whether this alone removes the overlap is DEFERRED to the UI checker resize check.

## Scan

Method: two `rg`-based scans over `crates/*/src` and `plugins/*/src`: (1) every `flex_1` / `flex_shrink` item with no `min_w_0`, `truncate`, or width; (2) every `h_flex` or `flex_row` child `div` that holds one text child with no guard.

Fixed:

- `crates/timeline/src/card_view.rs`: option text column (`flex_1`), plan card title (`flex_shrink(1.)`), question tab and header, permission action button and label.
- `crates/timeline/src/lib.rs`: row content column, todo task text, notification message.
- `crates/settings/src/pane_projects.rs:509`: "Run as a service" column. The text is fixed, but its 64-character description is long for a narrow pane.
- `plugins/board/src/runs.rs:219`: the runs notice message (dynamic).
- `plugins/workspace-spaces/src/page.rs:447`: header column with the dynamic subtitle.

Left unchanged, with the reason:

- Column-direction `flex_1` panes, where the main axis is vertical and `min_h_0` already holds: `crates/app/src/plugin_views.rs:201`, `crates/command/src/center.rs:951`, `crates/command/src/shortcuts_dialog.rs:270`, `crates/explorer/src/branch_switcher.rs:711`, `diff_view.rs:919`, `explorer.rs:1359/1365/1381/1389/1549/1707/1723`, `file_view.rs:449/468`, `pull_request_view.rs:1447/1482/1686`, `ui.rs:50/81/111`, `crates/history/src/screen.rs:498/555`, `plugins/board/src/page.rs:358`, `recaps.rs:208`, `runs.rs:276/294/305/310`, `timeline.rs:653`, `plugins/thread-branch/src/panel.rs:449/475/491/520`, `plugins/watchtower-board/src/decision_modal.rs:273`, `plugins/workspace-spaces/src/page.rs:541/566`.
- Spacers and rules with no text: `crates/app/src/chrome.rs:276/819`, `crates/timeline/src/lib.rs:1400`, `plugins/board/src/timeline.rs:411/650`, `plugins/thread-branch/src/card.rs:138`, `panel.rs:657`, `crates/command/src/center.rs:933`.
- Fixed short labels or counts: `crates/settings/src/pane_appearance.rs:312` (theme name), `crates/explorer/src/explorer.rs:1762` (commit count), `plugins/board/src/recaps.rs:246` (day), `plugins/watchtower-board/src/task_list.rs:247` (Active, Blocked, Todo, Done, Unknown), `plugins/watchtower-board/src/run_cells.rs:61` (state word), `crates/settings/src/pane_providers.rs:121` (provider label), the +N and -N counts in `crates/explorer/src/ui.rs:179/184`, `crates/sidebar/src/sidebar.rs:1031/1036`, `crates/timeline/src/lib.rs:1904/1909`, `plugins/thread-branch/src/panel.rs:646/651`.
- Already guarded or scroll on purpose: `crates/app/src/chrome.rs:807` (truncates), `crates/explorer/src/diff_view.rs:660` (row has `min_w_0`), `crates/terminal/src/view.rs:890` (input in a `min_w_0` box), `crates/timeline/src/tool_view.rs:431` and the code and diff areas (`whitespace_nowrap` with `overflow_scroll`), the tool header summary (`tool_view.rs:265`, truncates), the live box (`lib.rs` `render_live_box`, truncates), user message, turn header, and step rows (already `min_w_0`).
- `crates/app/src/chrome.rs:197` host menu row: a host name and a `host:port` URL, which are short. The row clips with `overflow_hidden`. Group S owns that file.
- `crates/composer/src/composer.rs:2326/2499`: slash names and picker identifiers are `flex_none` next to a truncating column. TASK-238 rebuilds the slash picker.
- `crates/workspace/src/workspace.rs:180`: menu row hint, a short shortcut string.
- `crates/prototype/src/main.rs:461/539/585`: the prototype crate, not part of the app.

## Verified

- `cd packages/desktop-gpui && CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-timeline -p bachuc-settings -p bachuc-plugin-board -p bachuc-plugin-workspace-spaces` -> PASS, exit 0, 0 failures. The new `tests/layout.rs` ran 3 tests and all passed.
- `CARGO_BUILD_JOBS=4 ./cargo.sh build -p bachuc-app` -> PASS. One warning, "function `next_theme` is never used", is in `bachuc-app`, which this TASK did not edit.
- `./cargo.sh fmt --check -p bachuc-timeline`, `-p bachuc-settings`, `-p bachuc-plugin-board`, `-p bachuc-plugin-workspace-spaces` -> exit 0 each.
- UI checker at 700 px, AskUserQuestion option text wraps inside the card: DEFERRED (UI checker).
- UI checker, resize from 1600 px to 700 px with a long assistant message above a Recap card, no overlap: DEFERRED (UI checker). `BACHUC_WINDOW=<w>x<h>` sets the window size (TASK-155 outcome).
- UI checker at 700 px, a 200-character path with no spaces breaks or scrolls inside its block: DEFERRED (UI checker).
- `measure.sh` CPU before and after: DEFERRED (UI checker); see Anti-goal.

## Anti-goal

- Checkpoint "timeline CPU on the mock stream, base commit": DEFERRED (UI checker). `measure.sh` launches the GUI app, which builders must not open. Source: rule in `/tmp/wt-builder-rules.md`. Time: 2026-10-18.
- Checkpoint "timeline CPU on the mock stream, after the fix": DEFERRED (UI checker), same reason.
- Code read for the tripwire: the change adds no `remeasure` or `remeasure_items` call and no per-frame measure. The row `min_w_0` removes one min-content probe per visible row per layout, so CPU should not rise.

## Lessons

- In GPUI, a flex item that holds text needs `min_w_0`, unless it has a definite width. Text answers the min-content probe with its unwrapped width, and taffy uses that as the flex item's minimum.
- The GPUI list re-measures visible rows on every layout and drops all cached heights on a width change. If text overlaps inside a timeline row, look inside the row, not at the list.
