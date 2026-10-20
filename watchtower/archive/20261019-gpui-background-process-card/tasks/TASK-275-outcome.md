# TASK-275 outcome

Status: DONE (CPU cap met after TASK-276: median 7.2%)

## Changed

- `packages/desktop-gpui/crates/theme/src/motion.rs` (new): ticker, `motion_ms`, `FRAME_INTERVAL`, `SLEEP_AFTER`, `Motion`, `frame_draws`, `request_tick`, 7 unit tests.
- `packages/desktop-gpui/crates/theme/src/lib.rs`: `pub mod motion`.
- `packages/desktop-gpui/crates/theme/Cargo.toml`: dev-dependency `gpui` (`gpui-pre =0.3.9`, `test-support`). REVERTED in Fix Round 1.
- `packages/desktop-gpui/Cargo.lock` (REVERTED in Fix Round 1): 224 added lines for the test-only dependencies of that feature (`proptest`, `rand 0.10`, `postage`, and others); no version of an existing package changed.
- `packages/desktop-gpui/crates/timeline/src/motion.rs`: ticker, clock, and `Motion` removed; re-exports `FRAME_INTERVAL`, `Motion`, `frame_draws`, `motion_ms`. Loader, pulse, and shimmer functions stay.
- `packages/desktop-gpui/crates/sidebar/src/ring.rs`: own 33 ms task removed; `request_tick` from the paint; phase from `motion_ms`, period 900 ms.
- `packages/desktop-gpui/plugins/stop-orb/src/face.rs`: `window.request_animation_frame()` -> `request_tick(cx.entity_id(), cx)`.

## Contract

- `bachuc_theme::motion::{FRAME_INTERVAL: Duration, motion_ms() -> u128, Motion, request_tick(view: EntityId, cx: &mut App)}` as specified. Also public: `SLEEP_AFTER`, `frame_draws`.
- `bachuc_timeline::motion` re-exports `FRAME_INTERVAL`, `motion_ms`, `Motion`, `frame_draws` with unchanged signatures; `crates/timeline/tests/motion.rs` passes unchanged.
- `request_tick` keeps `(EntityId, Instant)`. The next tick notifies a view once, then the view leaves; a view must request again from its next render or paint. A request older than `SLEEP_AFTER` at tick time drops without a notify. The ticker task ends on the first tick that finds no fresh view and no painted `Motion`, and `request_tick` or `Motion` render restarts it.
- Reduced motion: `request_tick` returns at once; `Motion::render` draws time 0 and does not re-register; the ring draws phase 0.

## Active Sources (TASK-274) check

| Source                                                                                          | Result                                                           |
| ----------------------------------------------------------------------------------------------- | ---------------------------------------------------------------- |
| Stop-orb face                                                                                   | on `request_tick`; none while `cancelling` or with reduce motion |
| Sidebar `RunningRing`                                                                           | on `request_tick`, shared clock                                  |
| Tab-bar `StatusBadge` ring                                                                      | covered: `badge.rs` builds `RunningRing`                         |
| Timeline marks                                                                                  | `Motion` moved to `bachuc_theme::motion`, same behavior          |
| Timeline clock and composer live metrics (1 fps, `timeline/src/lib.rs:1782`, `composer.rs:866`) | unchanged, see Questions                                         |

## Verified

- `cargo.sh test --no-fail-fast -p bachuc-theme -p bachuc-timeline -p bachuc-sidebar -p bachuc-composer -p bachuc-plugin-stop-orb` -> every result `ok` (theme 28 incl. 7 new motion tests, composer 125 + 4 of Hiep's `text_replacement`, sidebar model 43, stop-orb 9).
- First run of the same command failed `bachuc-sidebar` `building_500_workspaces_in_status_mode_stays_under_2_ms` (known timing test, load 4.4). Alone it failed once and passed 3 of 3 afterward; the second full run passed.
- `cargo.sh build` (workspace) -> Finished. `cargo.sh fmt --check -p` for theme, timeline, sidebar, stop-orb -> exit 0.
- Unit tests in `bachuc-theme`: two views notified in one pass; a request older than 200 ms not notified; ticker task ends when no view is left; dropped view not notified and ticker restarts on the next request; reduce motion requests no tick; `Motion` draws time 0 with reduce motion.
- `rg request_animation_frame packages/desktop-gpui/plugins/stop-orb packages/desktop-gpui/crates/sidebar/src/ring.rs` -> no match.
- Read check: no `with_animation(..).repeat()` or own repeat timer in `ring.rs`, `badge.rs`, or `face.rs`.
- GitNexus impact: `RunningRing` LOW (3 callers, `sidebar.rs`, `badge.rs`, `lib.rs`); `OrbFace` 0 callers; `Motion` not indexed, confirmed with `rg` (only `timeline/src/lib.rs`).
- Hiep's uncommitted files under `vendor/gpui-base/src/input/**`, `vendor/gpui-base/Cargo.toml`, `BACHUC-PATCH.md`, and `crates/composer/tests/` untouched (same `git status` entries before and after).

DEFERRED (UI checker): ring arc half a turn apart at 0 ms and 450 ms; orb dots and ring arc differ in two screenshots 200 ms apart; after the turn the orb is gone, the ring is a still dot, and a 60 s idle read is within 0.5 points of the TASK-274 idle read (2.7%).

## Anti-goal

DEFERRED (UI checker): three `measure.sh 60 gpui=<check bundle pattern>` reads during a running `sleep 900` turn, median at most 10%. Not measured here; no app window was opened. Expected frames: orb, both rings, and timeline marks now land on one 66 ms tick, at most 15 fps against 60 fps (orb) and 30 fps (rings) before. Whether that reaches 10% is UNVERIFIED.

## Questions

- The 1 fps timeline clock and composer live-metrics timers appear in the `## Active Sources` table (1.6 points). They are text clocks (seconds), not marks, and the lead's TASK-274 answer lists four active sources without them. Left as they are. Converting them to the 15 fps ticker would raise their frame rate.
- `gpui-kit` `test-support` as a dev-dependency does not resolve (`cocoa =0.26.0` against locked 0.26.1; the known MEMORY learning). Safe default taken: a dev-only `gpui` (`gpui-pre =0.3.9`, `test-support`) for the test context. `Cargo.lock` gained test-only packages. A test run of `-p bachuc-theme` builds `gpui-pre` with `test-support`; a non-test build does not.

## Lessons

- `gpui-kit`'s `test-support` cannot be a dev-dependency here; `gpui = { package = "gpui-pre", version = "=0.3.9", features = ["test-support"] }` gives `#[gpui::test]` and `TestAppContext`, and the same `gpui` types work with `gpui_kit` imports.
- `Window::draw(cx)` returns a value whose `clear(cx)` needs `cx` again; bind it first (`let draw = window.draw(cx); draw.clear(cx);`).
- Test the ticker staleness rule through a pure function with fake `Instant`s: the test executor clock moves timers but not `Instant::now`.
- The cargo dev profile was not cleaned; the lead cleans after the last cargo command of the run.

## Fix Round 1

Cause: the lead re-check found the dev-dependency `gpui` with `test-support` in `bachuc-theme`. It is the one `test-support` feature in the workspace, so a combined `cargo.sh test -p ...` builds gpui-pre twice (CLAUDE.md rule), and it added 224 lock lines outside the spec Files.

Changed:

- `packages/desktop-gpui/crates/theme/Cargo.toml`: restored to HEAD (no dev-dependency).
- `packages/desktop-gpui/Cargo.lock`: restored to HEAD; unchanged after the test and build runs below.
- `packages/desktop-gpui/crates/theme/src/motion.rs`: shipped behavior kept. The ticker rules moved into pure functions over plain data: `note_request` (one entry per view), `take_due_views` (fresh vs stale at `SLEEP_AFTER`), `keeps_ticking` (any view notified or mark painted), `ticks` (reduce-motion decision), `draw_time` (time 0 with reduce motion), `frame_draws`. `request_tick`, `Motion::tick`, `Motion::render`, and `tick_all` call them. The `#[gpui::test]` tests, `TestAppContext`, and the test-only `ticker_running` are removed. 7 plain `#[test]` tests use fake `Instant`s.

Unit-test Verify lines (all PASS, `cargo.sh test -p bachuc-theme`, 7 `motion::tests`):

- Two views requesting in one interval notified in one pass: PASS through `note_request` + `take_due_views` (`two_views_requesting_in_one_interval_are_notified_in_one_pass`).
- A view not painted for 200 ms is not notified: PASS through `take_due_views` (`a_view_not_painted_for_200_ms_is_not_notified`).
- The ticker task ends when no view is left: PASS through `keeps_ticking` (`the_ticker_ends_when_no_view_is_left`).
- Reduce motion: `Motion` draws time 0 and requests no tick: PASS through `draw_time` and `ticks` (`a_motion_draws_time_0_with_reduced_motion`, `reduced_motion_requests_no_tick`).
- A dropped view is not notified after `SLEEP_AFTER`, and the ticker restarts on the next request: PASS through `take_due_views` + `keeps_ticking` (`a_dropped_view_is_not_notified_and_the_next_request_wakes_the_ticker_again`).
- Also `frame_draws` (`a_frame_draws_when_due_and_changed`).

Read-checked only (no test): the GPUI wiring. `tick_all` calls `cx.notify(view)` for each due id and `Motion::tick` for each mark. `start` spawns the task only when `TICKER.task` is `None` and clears it when `tick_all` returns false. `request_tick` and `Motion::render` call `start`/`register`. A real notify and the real 66 ms timer are not exercised in a test.

Checks:

- `CARGO_BUILD_JOBS=4 cargo.sh test --no-fail-fast -p bachuc-theme -p bachuc-timeline -p bachuc-sidebar -p bachuc-composer -p bachuc-plugin-stop-orb` -> exit 0, every result `ok` (no Known Failure hit).
- `cargo.sh build` -> exit 0. `cargo.sh fmt --check -p bachuc-theme -p bachuc-timeline -p bachuc-sidebar -p bachuc-plugin-stop-orb` -> exit 0 (after `cargo.sh fmt -p bachuc-theme` fixed one line).
- `rg -n test-support packages/desktop-gpui --glob Cargo.toml --glob '!**/vendor/**' --glob '!**/target/**'` -> no match. (With the literal glob `!vendor/**` the vendored `gpui-base` `Cargo.toml` still matches; that is Hiep's vendored file, untouched.)
- `git diff --stat packages/desktop-gpui/Cargo.lock` -> empty, after test and build.
- Hiep's uncommitted files (`vendor/gpui-base/src/input/**`, `vendor/gpui-base/Cargo.toml`, `BACHUC-PATCH.md`, `crates/composer/tests/`) untouched.

DEFERRED (UI checker): unchanged. The CPU anti-goal stays DEFERRED. The dev profile was not cleaned; the lead cleans after the last cargo command of the run.

Lesson: no `test-support` dev-dependency in a leaf crate here. Put the rule in a pure function over plain data and test that.

## UI Checks

Checker: UI checker, Sonnet 5.5, run on 2026-10-20 (01:18 to 01:27 local), live `127.0.0.1:6768`, check bundle opened with `open -n`, kept in front with an `open <bundle>` loop every 3 s (owner override: front allowed). Window 1200x800 (2400x1600 px screenshots at 2x). Release daemon and release apps untouched.

### Binary proof

- Bundle `Bachuc GPUI Check.app` binary md5 `b0bed4e801d46cbe4a25b6db6c555cb3` = `target/release/bachuc-desktop` `b0bed4e801d46cbe4a25b6db6c555cb3`; BASE `/tmp/wt-base-app/Bachuc GPUI Base.app` `324b683285f877047c9030a797b607b8` (differs).
- `strings -a` of the bundle binary: `crates/theme/src/motion.rs` present (2 hits).
- Source: code commit `fff8e792d`; HEAD `49681601e` has no `packages/` diff against it. Build: `bundle.sh` 1m01s, `Finished release`. Hiep's uncommitted `vendor/gpui-base/src/input/**` changes were in the tree, as in TASK-274.

### Setup

Agent `8ea5c737` (Claude Haiku 5.5, `/tmp/wt-mock-275`), foreground `sleep 580` steps; default layout (sidebar open, one agent tab, composer, no board page); stop-orb enabled (running on live). The `Working` clock advanced during the reads (0:36, 4:05, 4:16) and `Background 1 running` showed.

### CPU reads (`measure.sh 60 r<n>=pids:19006`, during the running turn)

| Read | Start    | App / SHA                                    | md5 (bundle)  | Window   | Agent      | CPU %                                |
| ---- | -------- | -------------------------------------------- | ------------- | -------- | ---------- | ------------------------------------ |
| r1   | 01:20:14 | Check bundle, `fff8e792d`                    | `b0bed4e8...` | 1200x800 | `8ea5c737` | 14.9 (`sample` ran during this read) |
| r2   | 01:21:16 | same                                         | same          | same     | same       | 14.8                                 |
| r3   | 01:22:18 | same                                         | same          | same     | same       | 15.2                                 |
| idle | 01:24:36 | same, turn ended (interrupted), sidebar open | same          | same     | same       | 2.4                                  |

Median of the three running reads: 14.9% (TASK-274 CHECK `083620d10`: 31.3%; BASE: 42.4%). The 10% cap is NOT met.

### Sample (`sample 19006 10 -file /tmp/gpui-sample-275.txt`, during r1)

Main thread 5666 samples; `Window::draw` 1019 (18%; the rest is the idle event loop). Top 10 self frames under `Window::draw`: `_platform_memmove` 132, taffy `compute_flexbox_layout` 64, `<deduplicated_symbol>` 43, `_xzm_xzone_malloc_tiny` 39, taffy `compute_inner` (block) 26, `_xzm_free_main` 25, gpui `BoundsTree::insert` 23, gpui `Style::refine` 22, `Iterator::map` over taffy children 17, `TaffyView` tree fn 15. Same cost shape as TASK-274 (layout and copy per frame); there are fewer frames. Draw was 85% of main-thread samples on TASK-274 CHECK, now 18% of all, mostly idle wait.

### Screen checks

- Orb dots and sidebar ring in two screenshots about 200 ms apart (shots at 0 and 202 to 283 ms): PASS. The orb dot pattern differs; the sidebar ring arc moved from bottom-left to upper-left; the loader dots in the timeline also moved.
- One ring at 0 ms and about 450 ms: PASS (approximate). 10 crops of the ring, timestamps 54 to 737 ms. The arc was at the bottom at 54 ms and at about 1 o'clock at 448 ms (about 200 degrees apart, about half a turn); at 588 ms it was at 3 o'clock. Screenshot timing has a +/-50 ms error and the ring steps at 66 ms.
- After the turn: orb gone, ring a still dot: PASS. The turn was ended with `stop` (interrupt) at about 4m50s, not by the end of the 39 min script. The composer showed no stop orb, the row dot was a still green dot, the tab badge a still green dot.
- Idle read: 2.4 against the stated floor 1.3 = +1.1 points: FAIL by the literal rule. The like-for-like TASK-274 state (turn done, `stop-orb` on, sidebar open) read 2.7, so 2.4 is 0.3 below it (within 0.5). The 1.3 floor is the new-tab, sidebar-hidden state, with no agent tab and no 1 fps clock.

### Restore

Check app quit (pid 19006, no process left); the read loop ended; agent `8ea5c737` archived; stop-orb still `running` on live; zoom not changed; `git status` unchanged except this file; Hiep's uncommitted files untouched. The cargo dev profile was not touched by this check (only `--release` was built). The mock workspace `wt-mock-275` stays on live.

## CPU Close

CPU close (lead, 2026-10-20): the TASK-276 UI check read `measure.sh 60` on the `63321de20` check bundle (md5 `1b1d2715...`) during a running turn, 1200x800 in front: 6.9, 7.2, 7.3, median 7.2% (set 1: 7.4, 7.6, 6.9). The 10% cap of ADR-0039 is met. Caveats: no BASE read under the same procedure; BASE and NEW share a bundle id; the window froze for about 4 minutes in set 1 until a click (Q-179). Details: [TASK-276 outcome](TASK-276-outcome.md), `## UI Checks`.
