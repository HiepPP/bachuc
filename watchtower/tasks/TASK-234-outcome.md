# TASK-234 Outcome

## Status

Blocked: BUILT, UNVERIFIED (Q-152).

DONE, with the UI checker line DEFERRED and the owner hand test UNVERIFIED. The zoom is a whole-UI zoom only in part: text, gpui-kit rem sizes, radii, and the header, tab bar, and tab chip heights scale. Most fixed `px` paddings and icons in the view crates do not scale yet; see Not scaled.

## Changed

- `packages/desktop-gpui/crates/theme/src/lib.rs`: a `zoom` factor in the theme state. `ui_font_size`, `code_font_size`, `content_font_size`, and the gpui-kit radii apply it. New `set_zoom`, `zoom`, `zoomed`, and `rem_size`.
- `packages/desktop-gpui/crates/settings/src/appearance.rs`: `Appearance.zoom`, saved as `"zoom"` in `appearance.json`, with `ZOOM_STEPS`, `ZoomDirection`, `nearest_zoom_step`, and `zoom_step`. `apply()` calls `set_zoom`. Tests added.
- `packages/desktop-gpui/crates/settings/src/pane_appearance.rs`: `persist` keeps the zoom on screen, so a later Appearance change does not undo a zoom made while Settings shows.
- `packages/desktop-gpui/crates/command/src/shortcuts.rs`: defaults `view.zoom.in` Cmd+=, `view.zoom.out` Cmd+-, and `view.zoom.reset` Cmd+0 (General section). `crates/command/tests/shortcuts.rs`: a chord and empty-override test.
- `packages/desktop-gpui/crates/app/src/actions.rs`: `ZOOM_IN`, `ZOOM_OUT`, `ZOOM_RESET`. `binding_specs` also binds `cmd-+` for zoom in when its chord ends in `=` without Shift (`plus_alias`). Tests added.
- `packages/desktop-gpui/crates/app/src/shell.rs`: the `run_action` arm steps the zoom, sets the rem size, and saves `appearance.json`. `Shell::render` sets `window.set_rem_size(bachuc_theme::rem_size(cx))`.
- `packages/desktop-gpui/crates/app/src/chrome.rs` and `crates/workspace/src/workspace.rs` (group S files of TASK-236 and TASK-237): the header rows, the tab bar, the tab chip, and the tab drag preview use `zoomed(...)` for their height, so zoomed text does not clip.

## Contract

- Method: GPUI has `Window::set_rem_size` (`gpui-pre-0.3.9/src/window.rs:3113`), but the app sizes almost everything in `px` (about 2,100 sites). The display scale factor can be set only in GPUI tests (`window.rs:3096`). So the zoom is the rem size plus a theme-level multiplier on the `bachuc-theme` font sizes and radii.
- Produces `bachuc_settings::appearance::zoom_step(current: f32, direction: ZoomDirection) -> f32`, `nearest_zoom_step(f32) -> f32`, `ZOOM_STEPS`, `ZoomDirection { In, Out, Reset }`.
- Produces `bachuc_theme::{set_zoom, zoom, zoomed, rem_size}`.
- Action ids `view.zoom.in`, `view.zoom.out`, `view.zoom.reset`. Cmd+Shift+= arrives in GPUI as `cmd-+` (`gpui-pre-macos-0.3.9/src/events.rs:476`), so that is the alias.
- A stored zoom off-step reads as the nearest step. Out of range reads as 50% or 200%. A non-number or a missing value reads as 100%.

## Not scaled

- Every `px(metrics::SPACE_*)`, `px(metrics::ICON_*)`, `CONTROL_*`, and layout width in the view crates: timeline, composer, sidebar, explorer, terminal, command, settings, and all plugin crates.
- 68 `line_height(px(..))` sites and 15 `text_size(px(..))` sites, for example `crates/terminal/src/view.rs:1068`, `crates/sidebar/src/sidebar.rs:1518` and `1602`, `plugins/board/src/header_chip.rs:99`, `plugins/workspace-spaces/src/space_menu.rs:572`, and `plugins/thread-branch/src/card.rs:306`.
- `crates/app/src/plugin_views.rs:139` (group B) keeps `px(HEADER_HEIGHT)` for the plugin surface header.
- Tab widths keep `TAB_MIN_WIDTH` and `TAB_MAX_WIDTH`, so labels truncate sooner at a large zoom.
- To convert one, replace `px(metrics::X)` with `bachuc_theme::zoomed(metrics::X, cx)`.

## Verified

- `cd packages/desktop-gpui && CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-settings -p bachuc-command -p bachuc-workspace -p bachuc-theme -p bachuc-app -p bachuc-icons` -> PASS, exit 0, 17 result lines, 0 failures. New tests: `zoom_steps_in_and_out_and_stops_at_50_and_200_percent`, `a_stored_zoom_reads_as_the_nearest_step_or_100_percent` (off-step, out of range, `"NaN"`, null, missing), `zoom_has_default_chords_and_an_empty_override_unassigns_it`, `zoom_in_binds_cmd_equal_and_cmd_plus`, and `an_empty_zoom_override_binds_no_key`. The known-failure timing test passed in this run.
- `CARGO_BUILD_JOBS=4 ./cargo.sh build -p bachuc-app` -> PASS. One warning, `next_theme` is never used (`state.rs`), was there before this group.
- `./cargo.sh fmt --check -p <crate>` for settings, command, workspace, theme, app, and icons -> exit 0.
- Render path read -> PASS: `Shell::render` adds one global read and `set_rem_size`, which only stores a value. The zoom arm calls `set_zoom` only. It does not call `set_font_families`, so no `all_font_names` call runs per step or per frame.
- UI checker: Cmd+= twice, then Cmd+0 -> DEFERRED (UI checker).
- Owner hand test: zoom to 150%, quit, reopen -> UNVERIFIED (owner hand test).

## Anti-goal

- Checkpoint "no per-frame font query": value none added. Source: read of `Shell::render` (`crates/app/src/shell.rs:3588`), the zoom arm (`shell.rs:3317`), and `bachuc_theme::set_zoom` (`crates/theme/src/lib.rs:188`). Time: 2026-10-18.
- Checkpoint "a zoom step redraws in under 1 frame of extra work": DEFERRED (UI checker). A step writes the gpui-kit theme once and refreshes the windows once. Text at a new size is shaped again on the first frame, as with any font size change.

## Lessons

- GPUI app code in px does not follow `set_rem_size`. A true whole-window zoom needs the display scale factor, which gpui-pre sets only under `test-support`. Getting that needs a vendored gpui-pre patch, which would also scale the mouse positions and the viewport.

## UI Checks

- Line 45 (Cmd+= twice, then Cmd+0): PASS. Keys went to the background window with no focus change. After two Cmd+= the sidebar, tabs, message text, and composer padding are larger (`/tmp/wt-ui/chk-234-after2.png`); after Cmd+0 the layout equals the start (`/tmp/wt-ui/chk-234-before.png`, `/tmp/wt-ui/chk-234-reset.png`).
