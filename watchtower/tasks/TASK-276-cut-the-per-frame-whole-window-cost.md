# TASK-276 Cut the per-frame whole-window cost of a running mark

Group: F (the GPUI view boundaries that the profile names; no file shared with an open group)
Class: risky

## Brief

Goal: GPUI reads at most 10% CPU during a running turn with the release plugin set, by making each 15 fps mark frame cheap, not rarer.

Change: each mark frame copies and allocates more than it draws -> the render and layout of the shell, sidebar, timeline, and composer skip the per-frame copies and allocations the profile names.

Design: [ADR-0053](watchtower/decisions/ADR-0053-keep-ax-nodes-over-cached-gpui-views.md) rules out `.cached` views; [ADR-0052](watchtower/decisions/ADR-0052-one-shared-66ms-frame-ticker-over-per-mark-timers.md) keeps the 66 ms ticker; [ADR-0039](watchtower/decisions/ADR-0039-incremental-timeline-sync-over-throttle-or-budget.md) sets the 10% cap and rejects a slower stream redraw, a higher budget, and folded steps.

Boundaries:

- Measured start: after TASK-275 (`fff8e792d`) the median is 14.9% (reads 14.9, 14.8, 15.2), from about 1.3% idle floor plus about 13.6 points of 15 fps frames ([TASK-275 outcome](watchtower/tasks/TASK-275-outcome.md), `## UI Checks`). The top main-thread frames under `Window::draw` in `/tmp/gpui-sample-275.txt`: `_platform_memmove` 132, taffy `compute_flexbox_layout` 64, `_xzm_xzone_malloc_tiny` 39, taffy block `compute_inner` 26, `_xzm_free_main` 25, gpui `BoundsTree::insert` 23, gpui `Style::refine` 22. So the cost is whole-window layout and element rebuild per frame.
- Why: gpui-pre 0.3.9 `Window::mark_view_dirty` (`window.rs:2252`) marks every ancestor of a notified view dirty, and a dirty parent re-renders every child view that is not cached. No GPUI view in `packages/desktop-gpui` uses `.cached(..)` today (`rg -n "\.cached\(" packages/desktop-gpui/crates packages/desktop-gpui/plugins` -> no match). `AnyView::cached` and `Entity::cached` exist at gpui-pre `view.rs:46` and `view.rs:250`; a cached view that is not dirty reuses its last layout and paint.
- Step 1, profile: split the frame cost by view. Take a new `sample` during a running turn (the TASK-274 method, check bundle in front, 1200x800) and attribute `Window::draw` samples to the render and layout of the main subtrees: sidebar, tab bar, timeline list, composer, and the window chrome. Write the table into the outcome before any code change.
- Step 2, cut: remove the avoidable work the Step 1 table shows per frame: clones of large vectors, strings, or row models in a `render`, rebuilt style or text runs that do not change, and per-frame `HashMap` or `Vec` allocations. Fix the largest item first and re-read the sample after each fix. Follow the MEMORY lessons on render paths (no system queries, sorted vectors over small maps).
- No `.cached(..)` view boundary, per [ADR-0053](watchtower/decisions/ADR-0053-keep-ax-nodes-over-cached-gpui-views.md) (Q-177): in gpui-pre 0.3.9 a reused cached view loses its AX nodes and click actions (`view.rs:530-545`, `window.rs:3996` and `:4061` skip `window.a11y`; `a11y.rs:275` clears the nodes each frame).
- Keep: the 66 ms ticker and its states (ADR-0052), the 50 ms flush and incremental sync (ADR-0039), the always-open step list (ADR-0019), and every visible pixel.
- Do not patch gpui-pre. Do not lower the frame rate. Do not add a new view only to split the tab bar out of `Shell::render`. Do not touch Hiep's uncommitted files under `packages/desktop-gpui/vendor/gpui-base/**` and `packages/desktop-gpui/crates/composer/tests/`.
- Planner call: if the cap is still not met after the safe boundaries, stop, record the read and the remaining top frames, and set the TASK `BLOCKED` with a question; do not reach for a rejected option.

How:

- Profile the frame cost per subtree and record the table.
- Remove the largest per-frame copy or allocation, re-sample, and repeat.
- Keep each changed render's output the same; cover a changed pure helper with a unit test.
- Measure CPU as in TASK-274 and TASK-275.

Files:

- The owner views the Step 1 table names, expected among [packages/desktop-gpui/crates/app/src/shell.rs](packages/desktop-gpui/crates/app/src/shell.rs), [packages/desktop-gpui/crates/sidebar/src/sidebar.rs](packages/desktop-gpui/crates/sidebar/src/sidebar.rs), [packages/desktop-gpui/crates/timeline/src/lib.rs](packages/desktop-gpui/crates/timeline/src/lib.rs), and [packages/desktop-gpui/crates/composer/src/composer.rs](packages/desktop-gpui/crates/composer/src/composer.rs).
- [watchtower/tasks/TASK-276-outcome.md](watchtower/tasks/TASK-276-outcome.md) (new)

Interfaces:

- Consumes: `bachuc_theme::motion::{FRAME_INTERVAL: Duration, motion_ms() -> u128, Motion, request_tick(view: EntityId, cx: &mut App)}`; `bachuc_timeline::motion` re-exports `FRAME_INTERVAL`, `motion_ms`, and `Motion` with unchanged signatures.
- Produces: no new public API.

Expected result:

- While a turn runs, the orb, rings, loader, pulse, and shimmer move as after TASK-275, and the app reads at most 10% CPU.
- Every pixel and every AX node stays as after TASK-275.

Anti-goal: GPUI CPU during a running turn with the release plugin set, `measure.sh 60 check=pids:<pid>`, median of three reads at 1200x800 in front -> at most 10%; tripwire. Drift gauge: the AX node count of the window during a running turn, read by the UI checker on the `fff8e792d` bundle and on the new bundle, stays equal.

## Verify

- `cd packages/desktop-gpui && CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-app -p bachuc-sidebar -p bachuc-timeline -p bachuc-composer` -> all pass, in one command.
- `cd packages/desktop-gpui && CARGO_BUILD_JOBS=4 ./cargo.sh build` -> the workspace builds.
- Outcome check: the Step 1 table splits the `Window::draw` samples by subtree before any code change.
- UI checker on live (screen check): binary proof as in TASK-275 (bundle md5 equals `target/release`, differs from BASE).
- UI checker on live (screen check): AX dump during a running turn has the same node count and click actions on the `fff8e792d` bundle and the new bundle.
- Read check: `rg -n "\.cached\(" packages/desktop-gpui/crates packages/desktop-gpui/plugins` -> no match.
- UI checker on live (screen check): during a running turn, typing in the composer shows the caret and the text, and streamed text lands in the timeline.
- UI checker on live (screen check): during a running turn, the streamed text, the orb, the ring, and the loader all move in two screenshots about 200 ms apart.
- UI checker on live (screen check): after a theme change, a zoom change, and a window resize, every subtree shows the new state in one screenshot.
- Anti-goal: three `measure.sh 60` reads during a running turn -> median at most 10%; record each value and a new `sample` top 10.
