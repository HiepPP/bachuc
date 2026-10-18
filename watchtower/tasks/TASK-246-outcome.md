# TASK-246 Outcome

## Status

DONE (code checks pass; UI checker lines DEFERRED).

## Changed

- `packages/desktop-gpui/crates/timeline/src/`: `lib.rs`, `card_view.rs`, `find_bar.rs`, `image_view.rs`, `plugin_view.rs`, `tool_view.rs`, `plugins.rs`. About 228 `px(X)` sites became `zoomed(X, cx)`.
- `content_column_width(row_width, zoom)` and `column_right_inset(row_width, pills_width, zoom)` take the zoom; `ROW_PADDING`, `COLUMN_PADDING`, `MAX_CONTENT_WIDTH`, and the corner offset scale with it.
- Helpers without `cx` take the zoom or a `&App`: `fence_spacing(style, zoom)`, `heading_style(level, content, border, zoom)`, `focus_ring(color, zoom)`, `notification(.., cx)`, `icon_button(.., t, cx)`, `plugins::toolbar_origin(.., gap: Pixels)`.
- `packages/desktop-gpui/crates/timeline/tests/layout.rs`, `tests/plugins.rs`, and the unit tests in `lib.rs` and `find_bar.rs` pass the zoom 1; new zoom 1.5 tests.

## Contract

- Consumes TASK-245 `column_right_inset` and `set_corner_reserve` and TASK-243 `with_list_spacing` as named (zoomed call kept).
- Produces: `content_column_width(row_width: f32, zoom: f32) -> f32` and `column_right_inset(row_width: f32, pills_width: f32, zoom: f32) -> f32`. The caller passes `bachuc_theme::zoom(cx)`.
- The image size is kept at 100% in `ImageLoad::Ready` and multiplied by the zoom when it draws.

## Verified

- Group-level run: `./cargo.sh test -p bachuc-timeline -p bachuc-composer -p bachuc-app` -> pass.
- Group-level: `./cargo.sh build -p bachuc-app` -> builds; no new warning in changed files.
- `rg` for remaining `px(` in `crates/timeline/src` (non-test), by kept case: 7 zero values, 3 hairlines (1 px), 2 line heights from an already zoomed font, 1 row inset in px from a measured width, 1 list overdraw (`px(800.)`), 2 image sizes (natural size and the 100% fallback, zoomed at draw), 12 helper sites that multiply by an explicit `zoom` (6 in `focus_ring`, 6 in `fence_spacing` and `heading_style`). Test code keeps `px`.

## Anti-goal

DEFERRED (UI checker): 0 changed pixel rows at zoom 100%. By reading: `zoomed(X, cx)` equals `px(X)` at zoom 1; the focus ring border now scales (`2 * zoom`), the same at 100%.

## Lessons

- A scripted `px(` replace also matches the method `.px(`; the first run produced `.zoomed(px(..), cx)` and needed a fix pass.
- A closure given to `code_block_actions` must be `Send + Sync`, so it cannot capture `cx`; compute the zoomed values before it.
- Other crates' files (`settings`, `composer/input.rs`) failed to compile during the group run for about 3 minutes, then passed; a second run was green.
