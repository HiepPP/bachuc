# TASK-239 Outcome

## Status

Blocked: BUILT, UNVERIFIED (Q-150).

DONE, with the owner hand test UNVERIFIED and the `measure.sh` CPU read DEFERRED. One spec item, the next Space's list drawn next to the current one during the drag, needs a plugin-host API outside this TASK's files; see Questions.

## Changed

- `packages/desktop-gpui/crates/sidebar/src/plugin.rs`: `WheelGesture` is replaced by the state machine `SwipeGesture`, which returns a `SwipeStep`. New pure functions: `rubber_band`, `drag_frame`, `slide_out_ms`, and `slide_frame(target, from, elapsed)`, which now also draws a snap back.
- `packages/desktop-gpui/crates/sidebar/src/sidebar.rs`: the list follows the finger. A release commits or snaps back, and a commit slides on from the finger. One capture-phase wheel listener on the fixed tree replaces the per-row `on_scroll_wheel`. A quiet-timeout task ends gestures that have no `Ended`.
- `packages/desktop-gpui/crates/sidebar/tests/plugin.rs`: the old wheel and slide tests are rewritten for the new functions. This file holds the crate's tests for `plugin.rs`, so the old tests did not compile without this change.
- `packages/desktop-gpui/plugins/workspace-spaces/src/model.rs`: `swipe_target(snap, selected, swipes: &[SwipeDirection])` applies queued swipes in order and stops at the ends. Tests are updated and added.
- `packages/desktop-gpui/plugins/workspace-spaces/src/state.rs`: a swipe during a save goes into `queued_swipes`. The queue runs when the save ends, on success, on failure, and on a no-op save.
- TASK-232's `min_w_0` in `plugins/workspace-spaces/src/page.rs` is untouched.

## Contract

- `SwipeGesture::push(dx, dy, phase: TouchPhase, time_ms, width) -> SwipeStep`, `SwipeGesture::timeout_ms() -> Option<f64>`, and `SwipeGesture::tick(time_ms, width) -> SwipeStep`.
- `SwipeStep` is one of `Ignore`, `Drag { offset }`, `Commit { direction, offset }`, or `SnapBack { offset }`. Offsets are web-sign pixels: positive points at Next.
- Snap rule: a release at half the width or more commits. A flick also commits: velocity of 0.5 px/ms or more toward the offset, with 24 px or more of travel, and a last move no more than 60 ms before the lift. Any other release snaps back. The axis locks after 8 px. A gesture is horizontal when `|x| >= 1.5 * |y|`; it is vertical when `y >= 8` and it is not horizontal.
- The `ProjectFilter` API did not change: the sidebar still calls `on_swipe(direction)` once for each commit.

## GPUI scroll phases (vendored gpui-pre 0.3.9)

- `gpui-pre-macos-0.3.9/src/events.rs:259` maps the NSEvent phase as follows: MayBegin and Began give `TouchPhase::Started`, and Ended gives `Ended`. Every other phase gives `Moved`, which includes Changed, Cancelled, Stationary, and the momentum events (phase None).
- GPUI does not report a momentum flag. `TouchPhase::Cancelled` exists, but macOS never sends it.
- Handling: after a release, the gesture goes into a tail state that ignores `Moved` events until the next `Started` or a 120 ms quiet gap. A gesture with no `Started` (a mouse wheel) ends after a 120 ms quiet gap. A phased gesture that loses its `Ended` (Cancelled arrives as `Moved`) ends after 1500 ms and decides by travel alone. A `Started` during a drag snaps the old drag back.

## Design notes

- Hit-testing: the old handler sat on each row. A drag moves the rows, so the row under the cursor would slide away and the drag would stop getting events. The new listener is a canvas hitbox (`HitboxBehavior::Normal`, so row hover still works) in the capture phase over the fixed tree. It stops propagation for horizontal events and for every event during a drag, so the list never scrolls sideways. Vertical scroll still bubbles to the list.
- A missing answer: the plugin answers a commit with `SidebarFilterChanged { Some(direction) }` in the same effect cycle (`PluginContext::request` uses `cx.defer`). If no answer arrives by the next render, the list snaps back. The sidebar then remembers that edge (`swipe_edge`), and later drags that way rubber-band until a filter change with a direction clears the edge. This can happen at the first or last Space, or during a save, when the swipe is queued. A queued swipe slides in later as a normal slide.
- CPU: a drag frame calls only `cx.notify()`. No `refresh` and no `build_rows` run per event. The only `refresh` is at the first drag event, when a slide was still running. While dragging, the view requests no animation frames; frames come only from wheel events.

## Verified

- `cd packages/desktop-gpui && CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-sidebar -p bachuc-plugin-workspace-spaces` -> PASS, exit 0, 0 failures (51 + 2 + 1 + 22 + 32 tests). The Verify cases map to these tests:
  - snap: `a_slow_release_past_half_the_width_switches`
  - snap-back: `a_slow_release_short_of_half_snaps_back`, `a_snap_back_returns_to_the_start`
  - flick: `a_fast_flick_switches_short_of_half`
  - vertical-ignore: `a_vertical_scroll_is_never_a_swipe`
  - busy-queue and two queued swipes: `swipes_queued_during_a_save_run_in_order_and_stop_at_the_ends`
  - past the first or last Space: `a_swipe_moves_one_space_and_stays_past_either_end`, `a_drag_past_the_last_space_rubber_bands`
  - direction reversed mid-drag: `the_direction_follows_the_finger_when_it_reverses`
  - also: `the_list_follows_the_finger_one_to_one`, `the_momentum_tail_never_swipes_again_and_the_next_gesture_does`, `ten_quick_swipes_each_switch`, `a_wheel_without_phases_ends_after_a_quiet_gap`, `a_phased_drag_waits_for_its_lift_and_a_lost_one_ends_by_travel`, `a_cancel_or_a_new_start_mid_drag_snaps_back`, `a_released_swipe_slides_on_from_the_finger`
- `./cargo.sh fmt --check -p bachuc-sidebar -p bachuc-plugin-workspace-spaces` -> exit 0.
- `./cargo.sh clippy -p bachuc-sidebar -p bachuc-plugin-workspace-spaces --tests` -> exit 0. It reports no warning in the changed code; the old warnings are in untouched lines.
- GitNexus impact (index 4 commits behind HEAD): `swipe_target` LOW (2 direct), `slide_frame` LOW (1 direct), `WheelGesture` UNKNOWN (no callers resolved). An `rg` check confirmed that `sidebar.rs` and `tests/plugin.rs` are the only users.
- Owner hand test (swipe slowly, then fast, ten times) -> UNVERIFIED (owner hand test): needs a real trackpad swipe.
- `packages/desktop-gpui/measure.sh` during swipes, under 15% CPU -> DEFERRED (UI checker): it launches the app, and a real swipe needs the pointer.

## Anti-goal

| Checkpoint                         | Value                                                                                                      | Source                                                        | Time       |
| ---------------------------------- | ---------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------- | ---------- |
| Sidebar CPU during a swipe, before | DEFERRED (UI checker)                                                                                      | `measure.sh` launches the GUI app; builders open no window    | 2026-10-18 |
| Sidebar CPU during a swipe, after  | UNVERIFIED (unattended): needs pointer                                                                     | a real two-finger swipe is needed; `cua_repl` cannot make one | 2026-10-18 |
| Per-frame list rebuild tripwire    | Not present by reading: a drag event calls only `cx.notify()`; `refresh`/`build_rows` do not run per event | `crates/sidebar/src/sidebar.rs` `apply_swipe`                 | 2026-10-18 |

## Questions

- During the drag, the spec asks the sidebar to draw the current list next to the next Space's list. The sidebar can read only `ProjectFilter::is_visible` for the selected Space. It cannot read the next Space's projects, and it cannot tell if a next Space exists. Both need a plugin-host API, for example `ProjectFilter::peek(direction) -> Option<visible fn>`, in `crates/plugin-host`, which is outside this TASK's files. Safe default taken: the current list follows the finger and fades, and the incoming side is empty. On commit, the list slides on from the finger, the rows change, and the new list slides in from the far side.
- Because of the same API gap, rubber-band resistance starts only after one swipe found no Space in that direction. The first drag past an end goes 1:1 and then snaps back.

## Lessons

- Over a list that moves with the gesture, put the wheel handler on a fixed parent in the capture phase, not on the rows. Moved rows change hit-testing in the middle of the gesture.
- GPUI on macOS sends momentum and Cancelled scroll events as `TouchPhase::Moved`. Gesture code needs a tail state and a timeout; it cannot rely on phases alone.
