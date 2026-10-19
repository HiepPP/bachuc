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
