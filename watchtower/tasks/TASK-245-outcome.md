# TASK-245 Outcome

## Status

DONE (code checks pass; UI checker lines DEFERRED).

## Changed

- `packages/desktop-gpui/crates/composer/src/plugin_slots.rs`: `CornerPills` records the width of its pill column from a `canvas` child, `CornerPills::width() -> Pixels`, and notifies (`cx.defer`) only when the width changes by more than 0.5 px (`width_changed`). It resets to 0 and notifies when no pill shows. Zoom conversion of every `px(X)` in the file except `draft_pill_row`.
- `packages/desktop-gpui/crates/app/src/panes.rs`: `AgentPane` observes `CornerPills` and calls `TimelineView::set_corner_reserve`. Zoom conversion of every `px(X)` except the 1 px border.
- `packages/desktop-gpui/crates/timeline/src/lib.rs`: `corner_reserve` field, `set_corner_reserve`, `column_right_inset`, a private `column_width`, and the row uses `pl(COLUMN_PADDING)` and `pr(COLUMN_PADDING + inset)`.
- `packages/desktop-gpui/crates/timeline/tests/layout.rs`: `column_right_inset` tests.

## Contract

- `bachuc_timeline::column_right_inset(row_width: f32, pills_width: f32) -> f32` at this TASK; TASK-246 added a `zoom: f32` argument (see TASK-246).
- `CornerPills::width(&self) -> Pixels`; `TimelineView::set_corner_reserve(&mut self, width: Pixels, cx: &mut Context<Self>)`.
- The inset puts the text right edge at `pills_left - SPACE_3`, where `pills_left = row_width - SPACE_3 - pills_width`. It is 0 with no pills or a wide row, and never makes the text width negative.
- The row width is the timeline's last painted `bounds.width`. A change of the reserve calls `list.remeasure()`.

## Verified

- Group-level run: `./cargo.sh test -p bachuc-timeline -p bachuc-composer -p bachuc-app` -> pass, including the `column_right_inset` tests (no pills 0; wide row 0; 900 px row with 170 px pills; too narrow row never below 0) and `a_pill_width_passes_on_only_past_half_a_pixel`.
- Group-level: `./cargo.sh build -p bachuc-app` -> builds; `fmt --check` -> exit 0.

## Anti-goal

DEFERRED (UI checker): no frame loop, CPU under 1% over 10 s. By reading: the canvas sets the width and notifies only when it changes by more than 0.5 px, and the reserve is applied only when it changes by more than 0.5 px, so a stable layout stops after one extra frame.

## Lessons

- `draft_pill_row(pills)` in `plugin_slots.rs` still uses `px` for its two paddings: it has no `cx`, and its only caller is in `composer.rs` (owned by another group). It needs a `cx` argument from `composer.rs:3060`. Q-ID suggestion: convert it with TASK-247.
- Taffy does not need `relative()` on the absolute pill column for the `canvas` child to measure it.
