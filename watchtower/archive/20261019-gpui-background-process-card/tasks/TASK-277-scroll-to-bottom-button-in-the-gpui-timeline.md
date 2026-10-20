# TASK-277 Scroll-to-bottom button in the GPUI timeline

Group: F (shares `crates/timeline/src/lib.rs` with TASK-276, so one builder runs TASK-277 first, then TASK-276)
Class: code

## Brief

Goal: When Hiep scrolls up in a GPUI thread, a round down-arrow button shows at the bottom of the timeline, and a press jumps to the newest row and follows the stream again, as in Electron.

Change: no way back to the bottom except scrolling -> a centered "Scroll to bottom" button while the list does not follow the tail.

Design: Electron `AgentStreamView`, [packages/app/src/agent-stream/view.tsx](packages/app/src/agent-stream/view.tsx) lines 1250-1263 (button) and the `scrollToBottomContainer`, `scrollToBottomButton`, and `scrollToBottomIcon` styles near line 1799; the bottom offset is `resolveBottomOverlayControlOffset` (line 337, at least 16 px).

Boundaries:

- Look, from Electron: a 48 px circle, `surface2` fill, the `sm` shadow (`bachuc_theme::shadow(ShadowSize::Sm, cx)`), a lucide `ChevronDown` at 24 px in `foreground`, centered across the timeline, 16 px above its bottom edge. Scale every size with `zoomed(.., cx)`.
- Place it as an absolute child of the root `TimelineView::render` div (`crates/timeline/src/lib.rs` near line 3128; the root is `.relative()`), above the list and below the lightbox. It covers rows only where it is drawn.
- Show rule (Hiep chose in the `new` ask, 2026-10-20): show while `!self.list.is_following_tail()` and the list has rows; hide while it follows. A short timeline that does not fill the pane follows, so no button.
- Planner call: Electron shows the button past 64 px from the bottom; GPUI uses the list's own follow state, which `FollowMode::Tail` re-engages when the view returns to the bottom (gpui-pre `elements/list.rs` `set_follow_mode` docs).
- Planner call (Builder Check): derive the show state in `render` from `self.list.is_following_tail()`; keep no extra field. The list already notifies the view on every wheel or drag scroll (gpui-pre `elements/list.rs:959`), so scrolling up shows the button with no new notify.
- Scroll back to the bottom: the list re-engages the tail in its layout pass (`list.rs:1211-1217`), after the render, and the scroll handler runs before that. In the existing `set_scroll_handler` closure (`lib.rs` near line 500), when `event.visible_range.end` reaches the row count, defer one `cx.notify()` (`cx.on_next_frame` or `cx.defer`), so the next render sees the re-engaged tail and hides the button.
- Press: call `self.list.set_follow_mode(FollowMode::Tail)`, which sets following and scrolls to the end (`list.rs:617-635`), then notify. `scroll_to_end` alone does not re-engage following (`list.rs:603-611`). Keep `report_reading_position` working.
- No fade: Hiep chose no fade (2026-10-20), because each fade frame redraws the whole window (TASK-276, ADR-0052).
- AX: `Role::Button`, label "Scroll to bottom", and an explicit `AccessibleAction::Click` handler that runs the same press (MEMORY: every pressable element needs one).
- Hover: the composer icon-button hover fill is not in Electron for this button; draw no hover change. Pointer cursor on hover.
- The read-only subagent tab uses the same `TimelineView`, so it gets the button too.
- Keep: the 66 ms ticker and the CPU work of TASK-275 and TASK-276; no `.cached` view ([ADR-0053](watchtower/decisions/ADR-0053-keep-ax-nodes-over-cached-gpui-views.md)).

How:

- Derive the show state in `render`; add the deferred notify at the last row in the scroll handler.
- Render the button as an absolute child when shown.
- Wire the click and the AX click to `set_follow_mode(FollowMode::Tail)`.
- Add a unit test for a pure show-rule helper.

Files:

- [packages/desktop-gpui/crates/timeline/src/lib.rs](packages/desktop-gpui/crates/timeline/src/lib.rs)

Expected result:

- Scrolled up in a thread: the button shows; a press lands on the newest row, the button goes, and new streamed rows keep the view at the bottom.
- At the bottom, or in a short thread: no button.

Anti-goal: the scroll handler adds no `TimelineView` notify beyond the list's own per-scroll notify, except one deferred notify when the visible range reaches the last row; drift gauge by read check, and a 60 s CPU read while idle at the bottom within 0.3 points of the TASK-275 idle read (2.4%).

## Verify

- `cd packages/desktop-gpui && CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-timeline` -> all pass, including a new test of the show rule (rows 0 -> hidden; following -> hidden; not following with rows -> shown).
- `cd packages/desktop-gpui && CARGO_BUILD_JOBS=4 ./cargo.sh build` -> the workspace builds.
- Read check: the scroll handler adds only the one deferred notify at the last row; the press calls `set_follow_mode(FollowMode::Tail)`; the button has `Role::Button`, the label "Scroll to bottom", and an `AccessibleAction::Click` handler that runs the same press.
- UI checker on live (screen check): in a long thread, scroll up with an AX scroll action -> the button shows centered 16 px above the timeline bottom, 48 px at zoom 1 (scale by the zoom).
- UI checker on live (screen check): AX press on the button -> the newest row is in view and the button is gone.
- UI checker on live (screen check): a short thread that does not fill the pane -> no button.
- UI checker on live (screen check): scroll up, then scroll back down to the last row with AX scroll actions -> the button is gone without a press.
- UI checker on live (screen check): at zoom 1.25 the button is 60 px and 20 px above the bottom; switching to another thread while the button shows -> the new thread starts at the bottom with no button.
- UI checker on live (screen check): during a running turn, scroll up -> the button stays while new rows stream in; press -> the view follows the stream again.
- Anti-goal: one 60 s idle read at the bottom of a thread -> within 0.3 points of 2.4%.
