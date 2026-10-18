# TASK-245 Corner pills keep clear of message text

Group: T (timeline crate, plus `plugin_slots.rs` and `panes.rs`; shared files with TASK-243, TASK-244, TASK-246)
Class: risky

## Brief

Goal: in a 1200 px window, the Remove and "Remove & New Thread" pills never cover message text.

Change: the corner pills float at top 12, right 12 over the timeline ([plugin_slots.rs](packages/desktop-gpui/crates/composer/src/plugin_slots.rs) line 526) and text runs under them -> when pills show and the right margin beside the text column is narrower than the pills, the timeline column gets a right inset, so the text stops left of the pills.

Boundaries:

- Owner choice (2026-10-18): reserve a right gutter, not icon-only pills and not a move into the toolbar.
- Electron has the same overlap (`packages/app/src/plugins/buttons/view.tsx` line 1011); this is a GPUI-only fix.
- The inset is 0 when there are no corner pills, when the store hides them, and in a wide pane where the margin is already wide enough.
- Measure the pill column's real width after layout; do not guess label widths. Pass a new width on only when it changes by more than 0.5 px, so a frame does not trigger another frame forever.
- Put the inset math in a pure function next to `content_column_width` and test it in [crates/timeline/tests/layout.rs](packages/desktop-gpui/crates/timeline/tests/layout.rs).
- This TASK also owns the zoom conversion of `plugin_slots.rs` and `panes.rs` (Zoom Conversion Rule in [watchtower/CONTEXT.md](watchtower/CONTEXT.md)), so TASK-247 and TASK-248 do not touch them.
- Non-goal: the scrollbar and the scroll area stay full width.

How:

- `CornerPills` records the width of its `v_flex` (for example from a `canvas` or prepaint bounds) and notifies when it changes.
- `AgentPane` observes `CornerPills` and passes the width to the timeline.
- The row (`lib.rs:1207-1221`) hardcodes `ROW_PADDING`, `MAX_CONTENT_WIDTH`, and `COLUMN_PADDING`, and `content_column_width` has no production caller. Planner call: make the row use `content_column_width` and `column_right_inset(row_width, pills_width)` for its padding, so the tests prove the render math.

Files:

- [packages/desktop-gpui/crates/composer/src/plugin_slots.rs](packages/desktop-gpui/crates/composer/src/plugin_slots.rs) (width record; zoom conversion)
- [packages/desktop-gpui/crates/app/src/panes.rs](packages/desktop-gpui/crates/app/src/panes.rs) (observe and pass; zoom conversion)
- [packages/desktop-gpui/crates/timeline/src/lib.rs](packages/desktop-gpui/crates/timeline/src/lib.rs) (inset state, `column_right_inset`, column padding)
- [packages/desktop-gpui/crates/timeline/tests/layout.rs](packages/desktop-gpui/crates/timeline/tests/layout.rs)

Interfaces:

- Consumes: nothing from TASK-244; runs after it only because both write `lib.rs`.
- Produces: `bachuc_timeline::column_right_inset(row_width: f32, pills_width: f32) -> f32`, `CornerPills::width(&self) -> Pixels`, and `TimelineView::set_corner_reserve(&mut self, width: Pixels, cx: &mut Context<Self>)` for TASK-246.

Expected result:

- At `BACHUC_WINDOW=1200x800` with a finished Board run open, no text run intersects the pill boxes.
- At `BACHUC_WINDOW=1600x900` the text column is where it is today.

Anti-goal: no frame loop. With pills showing and no stream, CPU stays under 1% over 10 s idle; read from `ps -o %cpu` of the check bundle after the thread opens.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-timeline -p bachuc-composer -p bachuc-app` -> pass, with `column_right_inset` tests: no pills -> 0; wide row -> 0; 900 px row with 170 px pills -> inset puts the column right edge left of the pills plus `SPACE_3`; a row too narrow -> text width never below 0.
- UI checker at `BACHUC_WINDOW=1200x800`, a finished Board run thread -> AX bounds of the pills and of the message text do not intersect.
- UI checker at `BACHUC_WINDOW=1600x900` -> text column x and width equal the run before the change.
- UI checker: a run that is still running (no Remove pill) -> inset 0.
- UI checker: open a running Board run thread, then let it finish -> the gutter appears with the Remove pills; stop the daemon so the store goes offline -> pills hide and the gutter goes back to 0.
- Anti-goal: `ps -o %cpu -p <pid>` sampled 10 times over 10 s with the thread idle -> each under 1%.
