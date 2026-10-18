# TASK-236 Outcome

## Status

DONE, with both UI checker lines DEFERRED.

## Changed

- `packages/desktop-gpui/crates/settings/src/appearance.rs`: `Appearance.tab_close_position`, saved as `"tabClosePosition": "left" | "right"`. A file without the key, or with a value it cannot read, gives Left. `tab_close_label`. `apply()` calls `set_tab_close_position`. A test was added.
- `packages/desktop-gpui/crates/settings/src/pane_appearance.rs`: a Tabs section with a "Close button" Left or Right dropdown.
- `packages/desktop-gpui/crates/workspace/src/workspace.rs`: in Left mode the icon slot (`TAB_ICON_WIDTH`) holds the glyph and the close button, which swap by opacity on hover. Right mode keeps the old overlay, moved into `close_overlay`. The close button gets an explicit `AccessibleAction::Click` handler.
- `packages/desktop-gpui/crates/theme/src/lib.rs` (a TASK-234 file): `TabClosePosition`, `set_tab_close_position`, and `tab_close_position`.

## Contract

- Produces `bachuc_theme::TabClosePosition { Left (default), Right }`, `set_tab_close_position(TabClosePosition, &mut App)`, and `tab_close_position(&App) -> TabClosePosition`.
- The setting goes through `bachuc-theme`, because `bachuc-settings` does not depend on `bachuc-workspace`. The font settings use the same route. That is why the theme file changed, although it is not in this spec's Files.
- No hover toggles `display`. Both layers stay in the layout.

## Verified

- `cd packages/desktop-gpui && CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-settings -p bachuc-command -p bachuc-workspace -p bachuc-theme -p bachuc-app -p bachuc-icons` -> PASS, exit 0. `the_tab_close_button_is_left_unless_the_file_says_right` covers the old-file default, unknown values, and a save round trip.
- `./cargo.sh build -p bachuc-app` -> PASS. `fmt --check -p bachuc-workspace -p bachuc-settings` -> exit 0.
- UI checker: hover a tab in Left mode -> x in the icon slot, AX click closes the tab -> DEFERRED (UI checker).
- UI checker: switch to Right -> x on the right overlay -> DEFERRED (UI checker).

## Anti-goal

- Checkpoint "tab width stays the same in Left mode, 0 px change": read check, value 0 px. The chip width comes from `layout.widths` (`tab_layout`), which this TASK did not change. The icon slot is a `flex_none` box of `TAB_ICON_WIDTH`, the same width as the glyph it replaces. The close layer is absolutely positioned, so it adds no width. Source: `crates/workspace/src/workspace.rs` `render_tab`, 2026-10-18. A measured before and after read is DEFERRED (UI checker).

## Lessons

- A setting that a view crate reads, but that the settings crate cannot reach by a dependency, goes through a `bachuc-theme` global.
