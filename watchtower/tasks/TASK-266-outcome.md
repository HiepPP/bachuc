# TASK-266 outcome

Status: DONE (screen checks deferred to the UI checker)

Changed:

- `packages/desktop-gpui/crates/timeline/src/motion.rs` (new): `motion_ms`, `Motion`, `loader_draw`, `loader_step`, `loader_dot_opacity`, `loader_geometry`, `pulse_opacity`, and the shimmer functions.
- `packages/desktop-gpui/crates/timeline/src/lib.rs`: `RowKey::RunningFooter`/`RowKind::RunningFooter`, `ForkSource`, `ForkMenuKey`, the running footer row and render, the fork boundary refactor, the `Below::Footer` mapping, the motion cache, and the clock.
- `packages/desktop-gpui/crates/timeline/tests/motion.rs` (new): nine tests.

Contract:

- `bachuc_timeline::motion::motion_ms() -> u128`.
- `bachuc_timeline::motion::Motion` with `Motion::new(interval: Duration, draw: impl Fn(u128, &mut Window, &mut App) -> AnyElement + 'static, cx: &mut Context<Motion>)`, redrawing every `interval` while painted in the last 200 ms, and drawing time 0 when `App::reduce_motion()`.
- `Motion::set_draw(&mut self, draw: impl Fn(u128, &mut Window, &mut App) -> AnyElement + 'static, cx: &mut Context<Motion>)`, called from `sync`, never from a render.
- Row order at the stream bottom: running footer, bottom Background card, permission rows. The running footer maps to `Below::Footer`.
- Fork from the running footer emits `ForkBoundary::default()`; `ForkMenuKey::Running` cannot collide with a footer key; the menu clears when the footer goes.
- Start time: `ActiveTurn.started_at` when it parses (milliseconds kept), else the response's user message, else no clock.

Verified:

- `./cargo.sh test -p bachuc-client -p bachuc-timeline`: PASS (group-level run; timeline 136 tests, 0 failed).
- `tests/motion.rs` loader step 0/1/5/0 at 0/159/949/950 ms: PASS.
- `tests/motion.rs` dot opacity table and unknown step/dot = 0: PASS.
- `tests/motion.rs` geometry for size 14 -> gap 2, dot 10/3, 2 dots wide and 14 high: PASS.
- `tests/motion.rs` footer start: `ActiveTurn.started_at` with milliseconds, else the user message, else `None`: PASS.
- Read check of `sync`: the footer is pushed only when `turn_active`, after the stream rows and before the bottom card and permission rows; it maps to `Below::Footer`; no row fingerprint changes per loader step (the loader redraws through its own `Motion`).
- Read check: the running footer's fork menu uses `ForkMenuKey::Running` and `ForkBoundary::default()`; `fork_menu` clears when the footer goes.
- Read check of `motion.rs`: the timer notifies only while painted in the last 200 ms (the `ring.rs` shape); `Motion::render` draws time 0 when `reduce_motion()` is true.
- UI checker: footer on live with moving dots and a growing clock; fork menu through AX; completed footer after `sleep` -> DEFERRED (UI checker).
- Owner hand check: the loader looks like Electron -> UNVERIFIED (owner).
- Anti-goal: CPU with a live `sleep 90` turn within baseline + 2 points and at most 10% -> DEFERRED (UI checker). Baseline and final reads were not taken by the builder.

Anti-goal: not measured by the builder; the CPU read of the GPUI app is a screen check and is deferred.

Lessons:

- One shared `Motion` clock starts at the first `motion_ms()` call, so every mark moves in step.
- The loader interval is 158 ms (`950 / 6`); the pulse and shimmer use 33 ms.
- `Motion` entities are cached by `MotionKey` with a content hash. A changed hash calls `set_draw` from `sync`; a mark that stops running is dropped at the next sync.

## UI Checks

Run 2026-10-19 on live (6768) with the check bundle, launched with `BACHUC_NO_ACTIVATE=1` and driven by `cua_repl` AX actions only. Agent `c728c816` ran foreground `sleep 90` turns. The app was quit and the agent archived.

- UI checker on live: two screenshots 1.2 s apart while a turn ran -> PASS. The running footer was the last row: the loader dot opacities differed between shots, the fork icon was present, and the clock grew 41s -> 42s.
- UI checker on live: fork button through AX -> PASS. The menu opened with "Fork in a new tab" and "Fork in a new workspace"; Escape closed it; no fork was made.
- UI checker on live: after the sleep ended -> PASS. The running footer was gone and "Worked for 55s" showed.
- Plan Verify (NEXT.md): while a turn runs, the last row is the running footer with a moving loader, fork, and a growing clock -> PASS, from the first check.
- Anti-goal: 28.3% and 27.5% over two 60 s reads during live `sleep 90` and `sleep 120` turns -> FAIL against the 10% cap. Baseline comparison UNVERIFIED (unattended); no pre-change read exists.

## Fix Round 1

Status: DONE (screen checks deferred to the UI checker)

Changed:

- `packages/desktop-gpui/crates/timeline/src/motion.rs`: `Motion` no longer spawns a task per mark. A thread-local registry (`TICKER`) holds every live `WeakEntity<Motion>` and one task that ticks them all at `FRAME_INTERVAL`; the task stops when no mark was painted in the last 200 ms and restarts on the next render. `Motion::with_state(interval, state, draw, cx)` and `set_state` add a quantized state; a frame in the same state does not notify. `Motion::new` keeps the TASK-266 signature and passes no state.
- `packages/desktop-gpui/crates/timeline/src/lib.rs`: `ensure_motion` takes the state closure; the loader passes `loader_step(ms)`, the pulse and shimmer marks pass `pulse_state`/`shimmer_state`. The loader's state skips the frames between its 158 ms steps, so its 950 ms loop keeps its timing.

Contract:

- `bachuc_timeline::motion::motion_ms() -> u128`: unchanged.
- `bachuc_timeline::motion::Motion::new(interval, draw, cx)` and `Motion::set_draw(..)`: unchanged signatures.
- `Motion::with_state(interval, state: impl Fn(u128) -> u64 + 'static, draw, cx) -> Motion`: the timeline's constructor; one shared ticker for every mark in the app.
- The ticker notifies only a `Motion` entity. It does not call `TimelineView::sync`, reset the list, or notify the parent; GPUI itself marks the ancestor views dirty on any notified view.
- A tick that is not due, or whose quantized state equals the last drawn state, does nothing.
- The ticker stops after 200 ms without a paint; `Motion::render` registers again and restarts it.

Verified:

- `./cargo.sh test -p bachuc-timeline`: PASS (group-level run; 11 test files, 138 tests, 0 failed).
- `./cargo.sh build`: PASS, exit 0 (only the pre-existing `next_theme` dead-code warning in `bachuc-app`).
- Read check of `motion.rs`: `register` starts one task; `tick_all` iterates the registry and calls only `Motion::tick`; `tick` calls `cx.notify()` on its own entity only, and nothing writes TimelineView state; the task breaks when no mark was painted and clears its slot.
- Read check of `lib.rs`: `ensure_motion` reuses a mark when its content hash is unchanged; a changed hash calls `set_state` then `set_draw`, both from `sync`, never from a render.
- UI checker: CPU during a live turn -> DEFERRED (UI checker). No read was taken by the builder.
- Owner hand check: the loader looks like Electron -> UNVERIFIED (owner).

Anti-goal: not measured by the builder; the CPU read is a screen check. Expected effect: the measured 28% came from about 60 frames/s (three 33 ms timers out of phase) of a full timeline re-render per frame; the shared ticker makes one frame per 66 ms, and its quantized states skip further frames, so the animation cost should fall to about a quarter. The UI checker must confirm at most 10% and baseline + 2 points.

Lessons:

- GPUI 0.3.9 marks every ancestor view of a notified view dirty (`Window::mark_view_dirty`, `window.rs:2252`), so a child view animation re-renders the whole timeline that holds it. Keeping the animated view small does not isolate the cost; the frame rate and the number of frames do.
- Several per-mark timers with independent phases coalesce only per platform frame; they can request up to a 60 Hz redraw even when each mark ticks at 33 ms.
- A shared ticker that notifies all marks in one `tick_all` pass makes one frame per interval.

Questions:

- The pulse and shimmer intervals changed from 33 ms to 66 ms (15 fps) to fit the 10% CPU cap. 33 ms was the TASK-267 spec value; the lead allowed a lower rate when needed. The UI checker should record whether 15 fps still reads as smooth.
