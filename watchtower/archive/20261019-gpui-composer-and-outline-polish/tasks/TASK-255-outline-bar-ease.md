# TASK-255 Outline bars ease over 140 ms

Group: O (timeline crate, outline only)
Class: code

## Brief

Goal: the Chat outline bars change width, height, and color over 140 ms ease-out, as in Electron.

Change: bars jump to their new size at once -> each change of hover, attention, or the active prompt animates over 140 ms, then stops.

Boundaries:

- Q-168 (owner, 2026-10-19): add the ease. Electron: `transitionProperty: "width, height, background-color"`, `transitionDuration: "140ms"`, `transitionTimingFunction: "ease-out"` on the pill in `packages/app/src/agent-stream/chat-outline/rail.web.tsx`.
- Animate only while a change runs; draw no frame when nothing changes. Use a finite GPUI animation (for example `with_animation` with a 140 ms `Animation` keyed by the new target), not a repeating one.
- Reduced motion on: no animation and no magnification, as today (`cx.reduce_motion()`, `outline.rs:458`).
- Planner call: `tick` is stateless today. Store a per-bar from-value (size and color) in `OutlineRail`, so a change in mid-ease restarts from the drawn value, not from rest.
- Planner call: use CSS `ease-out`, `cubic-bezier(0, 0, 0.58, 1)`, not the vendored `ease_out_cubic`.
- Code fact: `with_animation(id, Animation::new(..).repeat(), ..)` is used at `crates/command/src/center.rs:1058`; use the finite form (no `.repeat()`), and confirm it in the gpui-pre source.
- Put the easing and the interpolation of size and color in pure functions, tested in [packages/desktop-gpui/crates/timeline/tests/outline.rs](packages/desktop-gpui/crates/timeline/tests/outline.rs).

Files:

- [packages/desktop-gpui/crates/timeline/src/outline.rs](packages/desktop-gpui/crates/timeline/src/outline.rs)
- [packages/desktop-gpui/crates/timeline/tests/outline.rs](packages/desktop-gpui/crates/timeline/tests/outline.rs)

Interfaces:

- Produces: no public API beyond the tested pure helpers.

Expected result:

- Moving the pointer along the rail shows the bulge growing and shrinking smoothly; the lit bar fades in over 140 ms when a jump lands.

Anti-goal: no frame loop. With the rail showing and no pointer movement, CPU stays under 1% over 10 s.

## Verify

- `cd packages/desktop-gpui && CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-timeline --test outline` -> pass, with tests for ease-out (0 at start, 1 at 140 ms, monotonic) and the size and color interpolation.
- `cd packages/desktop-gpui && CARGO_BUILD_JOBS=4 ./cargo.sh build -p bachuc-app` -> builds.
- Screen check: two screenshots about 70 ms apart after a hover on `mock TASK-252 outline` -> a bar size between rest and magnified in the first.
- `cd packages/desktop-gpui && CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-timeline --test outline` -> also covers a retarget in mid-ease (starts from the drawn value).
- Screen check: a fast sweep across 10 bars, then stop -> bars settle; no bar stays stuck mid-size.
- Screen check: hide the rail (narrow the window) during an ease -> no panic; the rail shows settled bars when it returns.
- Anti-goal: `ps -o %cpu`, 10 samples over 10 s, rail showing, no pointer movement -> each under 1%.
