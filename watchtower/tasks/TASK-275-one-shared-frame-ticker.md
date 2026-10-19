# TASK-275 One shared 66 ms frame ticker for every running mark

Group: E (the `bachuc-theme` motion module, timeline `motion.rs`, sidebar `ring.rs`, the stop-orb face, and each active source of TASK-274; no file shared with Group B or C1)
Class: risky

## Brief

Goal: The GPUI app reads at most 10% CPU during a running turn with the release plugin set, because every running mark redraws from one shared 66 ms ticker.

Change: per-mark 33 ms timers and 60 fps `request_animation_frame` -> one ticker at 66 ms, per [ADR-0052](watchtower/decisions/ADR-0052-one-shared-66ms-frame-ticker-over-per-mark-timers.md).

Design: the shared ticker of the timeline `Motion` marks, [packages/desktop-gpui/crates/timeline/src/motion.rs](packages/desktop-gpui/crates/timeline/src/motion.rs) (`TICKER`, `register`, `tick_all`, `Motion::with_state`).

Boundaries:

- Move the ticker, the clock `motion_ms`, `FRAME_INTERVAL`, `SLEEP_AFTER`, and the `Motion` view from `bachuc-timeline` to a new module `bachuc_theme::motion`. `bachuc-theme` is the one crate that every GPUI crate and plugin crate depends on. Keep the loader, pulse, and shimmer functions in the timeline module, and re-export the moved names there, so the timeline call sites and `crates/timeline/tests/motion.rs` keep their paths.
- Add a ticker entry for a view that is not a `Motion`: `bachuc_theme::motion::request_tick`. A view calls it from its render or paint when it draws a moving frame; the ticker notifies that view once at the next tick. A view that is not painted for `SLEEP_AFTER` drops out.
- stop-orb face ([packages/desktop-gpui/plugins/stop-orb/src/face.rs](packages/desktop-gpui/plugins/stop-orb/src/face.rs) line 105): replace `window.request_animation_frame()` with `request_tick`. Keep the orb's own clock, its hover speed, and the `cancelling` stop.
- `RunningRing` ([packages/desktop-gpui/crates/sidebar/src/ring.rs](packages/desktop-gpui/crates/sidebar/src/ring.rs)): drop its own 33 ms task and use `request_tick`. Read the phase from `motion_ms`, so rings, the loader, and the pulse share one clock. Keep the 900 ms period.
- Each other source in the TASK-274 `## Active Sources` table moves to the ticker the same way. Sources in `## Other Sources` stay as they are (Hiep's call in the `new` interview).
- One-shot transitions keep `request_animation_frame`: the sidebar slide (`crates/sidebar/src/sidebar.rs` near line 2816), the composer drop fade (`crates/composer/src/composer.rs` near line 1444), and the next-prompt-actions width probe.
- Do not change the timeline sync, the 50 ms flush, or the step list ([ADR-0039](watchtower/decisions/ADR-0039-incremental-timeline-sync-over-throttle-or-budget.md), [ADR-0019](watchtower/decisions/ADR-0019-always-open-step-list-over-folding-group.md)).
- `App::reduce_motion()`: a mark draws time 0 and requests no tick, as `Motion::render` does today.
- Planner call: the ticker is thread-local, as today; GPUI runs every view on the main thread.
- Planner call: `request_tick` keeps an `EntityId` and the time of the request. An id with no new request within `SLEEP_AFTER` drops out, so a dropped view leaves at the next tick; `App::notify(EntityId)` exists in gpui-pre 0.3.9 (`app.rs:3052`).
- The `bachuc-theme` ticker tests need `gpui-kit` with its `test-support` feature as a dev-dependency only. Declare `gpui-kit` features only through `[workspace.dependencies]` if a feature must change, so gpui does not build twice (CLAUDE.md).
- A vendored gpui-base source from TASK-274 (for example the stream fade) is in scope only when it is in `## Active Sources`. Keep the unrelated uncommitted changes under `packages/desktop-gpui/vendor/gpui-base/src/input/**`, and record the patch in `BACHUC-PATCH.md` ([ADR-0044](watchtower/decisions/ADR-0044-vendored-gpui-base-patch-over-git-fork-or-log.md)).

How:

- Move the ticker and `Motion` to `bachuc_theme::motion` and re-export them from the timeline module.
- Add `request_tick` for plain views.
- Move the stop-orb face, `RunningRing`, and each TASK-274 active source to the ticker.
- Add unit tests for the ticker in `bachuc-theme`.
- Measure CPU as in TASK-274.

Files:

- [packages/desktop-gpui/crates/theme/src/motion.rs](packages/desktop-gpui/crates/theme/src/motion.rs) (new: ticker, clock, `Motion`, `request_tick`)
- [packages/desktop-gpui/crates/theme/src/lib.rs](packages/desktop-gpui/crates/theme/src/lib.rs) (`pub mod motion`)
- [packages/desktop-gpui/crates/theme/Cargo.toml](packages/desktop-gpui/crates/theme/Cargo.toml) (test dev-dependency only)
- [packages/desktop-gpui/crates/timeline/src/motion.rs](packages/desktop-gpui/crates/timeline/src/motion.rs) (keeps the mark functions; re-exports the moved names)
- [packages/desktop-gpui/crates/sidebar/src/ring.rs](packages/desktop-gpui/crates/sidebar/src/ring.rs)
- [packages/desktop-gpui/plugins/stop-orb/src/face.rs](packages/desktop-gpui/plugins/stop-orb/src/face.rs)
- Each file of a TASK-274 active source, as listed in its table.

Interfaces:

- Consumes: `TASK-274 outcome ## Active Sources`, a table with columns Source, path:line, Frame rate, CPU share in points; and `## Other Sources` with the same columns for sources off the default layout.
- Produces: `bachuc_theme::motion::{FRAME_INTERVAL: Duration, motion_ms() -> u128, Motion, request_tick(view: EntityId, cx: &mut App)}`; `bachuc_timeline::motion` re-exports `FRAME_INTERVAL`, `motion_ms`, and `Motion` with unchanged signatures.

Expected result:

- While a turn runs, the stop orb, the sidebar ring, the loader, the pulse, and the shimmer move at 15 fps on one clock.
- When no turn runs, the ticker sleeps and the app reads near idle.
- GPUI CPU during a running turn reads at most 10% with the release plugin set.

Anti-goal: GPUI app CPU during a running turn with the release plugin set, `measure.sh 60`, median of three reads at 1200x800 in front -> at most 10% ([ADR-0039](watchtower/decisions/ADR-0039-incremental-timeline-sync-over-throttle-or-budget.md) cap); tripwire. Also idle CPU after the turn ends stays within 0.5 points of the TASK-274 idle read.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-theme -p bachuc-timeline -p bachuc-sidebar -p bachuc-composer -p bachuc-plugin-stop-orb` -> all tests pass, in one command, including `crates/timeline/tests/motion.rs`.
- `cd packages/desktop-gpui && ./cargo.sh build` -> the workspace builds.
- Unit test in `bachuc-theme`: two views that request a tick in the same interval are notified in one tick pass; a view not painted for 200 ms is not notified; the ticker task ends when no view is left.
- Unit test in `bachuc-theme`: with reduce motion on, `Motion` draws time 0 and requests no tick.
- Unit test in `bachuc-theme`: a view that requested a tick and then dropped is not notified after `SLEEP_AFTER`, and the ticker restarts on the next `request_tick` after it slept.
- Read check: `RunningRing` and the stop-orb face request no tick when reduce motion is on, and the orb requests none while `cancelling`.
- UI checker on live (screen check): screenshots at 0 ms and 450 ms of one running ring -> the arc is about half a turn apart, so the 900 ms period holds.
- `rg -n "request_animation_frame" packages/desktop-gpui/plugins/stop-orb packages/desktop-gpui/crates/sidebar/src/ring.rs` -> no match.
- Read check: each source in the TASK-274 `## Active Sources` table uses `bachuc_theme::motion`, and no source in it keeps its own repeat timer or `with_animation(..).repeat()`.
- UI checker on live (screen check): during a running turn, two screenshots about 200 ms apart show the stop orb dots and the sidebar ring arc at different positions.
- UI checker on live (screen check): after the turn ends, the orb is gone and the ring is a still dot; a 60 s idle read is within 0.5 points of the TASK-274 idle read.
- Anti-goal: three `measure.sh 60 gpui=<check bundle pattern>` reads of the check bundle during a running `sleep 900` turn, 1200x800, in front, release plugin set -> median at most 10%; record each value.
