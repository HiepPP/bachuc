# TASK-246 Zoom the timeline sizes

Group: T (timeline crate; shared files with TASK-243, TASK-244, TASK-245)
Class: code

## Brief

Goal: at zoom 150%, the paddings, gaps, icons, and fixed text sizes of the thread timeline grow with the text, as Q-151 answered.

Change: fixed `px(X)` sizes stay the same at every zoom -> they use `bachuc_theme::zoomed(X, cx)`, so they scale with Cmd+= and Cmd+-.

Boundaries:

- Q-151 (owner, 2026-10-18): convert with `zoomed`, not a gpui-pre scale patch.
- Follow the Zoom Conversion Rule in [watchtower/CONTEXT.md](watchtower/CONTEXT.md): what to convert, what keeps `px`, and no new constants.
- Count at plan time: about 269 `px(` sites in these crates, including tests and hairlines.
- `ROW_PADDING`, `COLUMN_PADDING`, and `MAX_CONTENT_WIDTH` scale, so the column math must take the zoom. Keep the list spacing and paragraph gap zoomed.
- Non-goal: no layout redesign; only the size source changes.

Files:

- [packages/desktop-gpui/crates/timeline/src](packages/desktop-gpui/crates/timeline/src) (all view files)
- [packages/desktop-gpui/crates/timeline/tests/layout.rs](packages/desktop-gpui/crates/timeline/tests/layout.rs) (`content_column_width` and `column_right_inset` with a zoom)

Interfaces:

- Consumes: `bachuc_timeline::column_right_inset(row_width: f32, pills_width: f32) -> f32` and `TimelineView::set_corner_reserve(&mut self, width: Pixels, cx: &mut Context<Self>)` from TASK-245; `TextViewStyle::with_list_spacing(self, top: Rems, bottom: Rems) -> Self` from TASK-243.
- Produces: no API for other TASKs. `content_column_width` and `column_right_inset` may gain a `zoom: f32` argument; update their tests.

Expected result:

- At zoom 150%, every converted padding, icon, and control height is 1.5 times its 100% size, and no label clips.
- At zoom 100%, the screens look as before.

Anti-goal: at zoom 100%, a screenshot of each screen in Verify differs from the same shot taken before the change by 0 changed pixel rows outside of text cursors and timestamps; read by the UI checker before and after.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-timeline` -> pass, in one cargo call.
- `cd packages/desktop-gpui && ./cargo.sh build -p bachuc-app` -> builds; no new warnings in changed files.
- `rg -n "\bpx\(" packages/desktop-gpui/crates/timeline/src` -> every remaining site is one of the kept cases of the Zoom Conversion Rule; the outcome lists the count per case.
- UI checker at `BACHUC_WINDOW=1200x800`, zoom 150%: a thread with a heading, list, code block, tool call step list, image, and plugin card -> paddings and icons are larger in proportion, no label clips, nothing overlaps.
- UI checker at `BACHUC_WINDOW=900x800`, zoom 200% -> message text wraps inside the column and no step or card label clips.
- Anti-goal: the same screens at zoom 100%, before and after -> 0 changed pixel rows outside text cursors and timestamps.
