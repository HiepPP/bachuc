# TASK-266 Running turn footer in the GPUI timeline

Group: B (the `bachuc-timeline` crate; shares [packages/desktop-gpui/crates/timeline/src/lib.rs](packages/desktop-gpui/crates/timeline/src/lib.rs) with TASK-265 and TASK-267)
Class: risky

## Brief

Goal: While an agent runs, the GPUI timeline shows the Electron running footer: an animated 6-dot loader, the fork button, and the elapsed time of the turn.

Change: no footer while a turn runs -> a running footer row at the stream bottom, drawn like `WorkingIndicator` in [packages/app/src/agent-stream/turn-footer.tsx](packages/app/src/agent-stream/turn-footer.tsx).

Design: [packages/app/src/agent-stream/turn-footer.tsx](packages/app/src/agent-stream/turn-footer.tsx), [packages/app/src/components/synced-loader.tsx](packages/app/src/components/synced-loader.tsx), [packages/app/src/components/synced-loader-state.ts](packages/app/src/components/synced-loader-state.ts).

Boundaries:

- Today `sync` pushes a footer only for a finished response (`lib.rs:910`, `push_footer` at `lib.rs:1170`). Keep that. Add a separate running footer row while the turn is active.
- Row order at the stream bottom, as Electron draws it: the running footer, then the bottom Background card of TASK-265, then the permission rows.
- Footer layout, from `turn-footer.tsx`: one row 24 px high, items centered, gap `SPACE_3`. Margin top `FOOTER_TOP` and padding bottom `SPACE_8`, as the completed footer (`render_footer`, `lib.rs:1920`). Children in order: the loader (14 px, `foreground`, margin left -2 px), the fork button, the elapsed text.
- Loader, from `synced-loader.tsx`: a 2 x 3 dot grid in a 14 px box. The gap is `max(1, round(size * 0.12))` and the dot is `max(2, (size - 2 * gap) / 3)`. Dots fill row by row, 2 per row. One loop is 950 ms in 6 steps. The step is `floor((now_ms % 950) * 6 / 950)`. Dot opacity comes from the 6 x 6 table `SYNCED_LOADER_OPACITY_STATES` in `synced-loader-state.ts`. All loaders use one shared clock, so they step together.
- Motion must not redraw the timeline list each frame. Follow the `RunningRing` pattern in [packages/desktop-gpui/crates/sidebar/src/ring.rs](packages/desktop-gpui/crates/sidebar/src/ring.rs): the animated mark is its own small view. It asks for a frame about every 33 ms while it was painted in the last 200 ms, and sleeps otherwise.
- Put that pattern in a new shared helper `motion.rs`, so TASK-267 reuses it. Keep one `Entity<Motion>` per animated mark, cached by key as `Sidebar::ring` does (`packages/desktop-gpui/crates/sidebar/src/sidebar.rs:794`).
- Fork, from `InFlightTurnForkHandler`: the running footer forks the whole chat, so it uses no boundary. Show it only when `Client::fork_supported` is true. On pick, emit `TimelineForkEvent::Fork { agent_id, target, boundary: ForkBoundary::default() }`. `ForkBoundary` with both fields `None` forks the whole chat (`packages/desktop-gpui/crates/client/src/model.rs:1090`); the shell passes it on unchanged (`shell.rs:1762`).
- Reuse the menu of the completed footer (`render_fork_menu`, `lib.rs:2137`), but it needs a change. `render_fork_menu` and `pick_fork` (`lib.rs:616-635`) take `start` and `end` and always recompute the boundary through `footer_fork_boundary`. Let them take the boundary source, so the running footer passes the whole-chat boundary. Give the running footer a `fork_menu` key that no entry key can take. Clear `fork_menu` when the running footer goes away.
- Elapsed: `format_elapsed(now - started)` (`lib.rs:3013`), size `METADATA_SIZE`, color `foreground_muted`, tabular digits when GPUI offers them. Electron uses the open turn's `startedAt`, and hides the clock when it has none.
- Planner call: take `started` from `Client::agent_active_turn(agent_id)` (`ActiveTurn.started_at`, an ISO string). Parse it with `chrono` `DateTime::parse_from_rfc3339`, which keeps milliseconds; `chrono` is already a timeline dependency. When it is missing or does not parse, use the running response's user message time. When that is also unknown, because the user message is on an older page (`lib.rs:887-889`), hide the clock, as Electron does.
- `update_clock` (`lib.rs:1367`) must also tick each second while the turn is active, even with no turn header. A turn can think for a while before its first tool call.
- Planner call: `Motion` draws the still state (time 0) when `App::reduce_motion()` is true. Nothing sets it from the OS today, so it is always false, but the check costs nothing.
- Follows [ADR-0039](watchtower/decisions/ADR-0039-incremental-timeline-sync-over-throttle-or-budget.md): no row fingerprint changes per frame. The row above the footer changes once when the footer comes or goes, because `sync` hashes `space_below` into it (`lib.rs:928-935`). Map the running footer to `Below::Footer` in `space_below`, as Electron uses the compact bottom while a turn runs (`packages/app/src/agent-stream/layout.ts:346`).
- `Motion` redraws at its own interval: the loader at its 158 ms step (950 / 6), so it does not redraw between steps. `set_draw` stores the closure and calls `cx.notify()`; call it from `sync`, never from a render.
- A `Motion` view is its own entity, but no view in this repo uses `.cached(`, so a redraw may still render its parent. The sidebar rings have the same shape. Only the anti-goal gauge proves the cost.

How:

- Add `pub mod motion;` with the shared clock, the `Motion` view, and the pure loader functions.
- Add a `RowKind` and `RowKey` variant for the running footer. Push it in `sync` when `turn_active`, after the stream rows and before the TASK-265 bottom card and the permission rows.
- Render the footer: the loader `Motion`, the fork button, and the elapsed text.
- Let the fork menu emit an empty `ForkBoundary` for the running footer.
- Extend `update_clock` to run while the turn is active.
- Add tests to [packages/desktop-gpui/crates/timeline/tests/motion.rs](packages/desktop-gpui/crates/timeline/tests/motion.rs).

Files:

- [packages/desktop-gpui/crates/timeline/src/motion.rs](packages/desktop-gpui/crates/timeline/src/motion.rs) (new: shared clock, `Motion` view, loader step, opacity, and geometry)
- [packages/desktop-gpui/crates/timeline/src/lib.rs](packages/desktop-gpui/crates/timeline/src/lib.rs) (module, row kind, footer render, fork boundary, clock)
- [packages/desktop-gpui/crates/timeline/tests/motion.rs](packages/desktop-gpui/crates/timeline/tests/motion.rs) (new tests)

Interfaces:

- Consumes: none. TASK-265 is a start order only, because both write `lib.rs`.
- Produces: `bachuc_timeline::motion::motion_ms() -> u128` (milliseconds on the one shared motion clock) for TASK-267
- Produces: `bachuc_timeline::motion::Motion` view, built with `Motion::new(interval: Duration, draw: impl Fn(u128, &mut Window, &mut App) -> AnyElement + 'static, cx: &mut Context<Motion>) -> Motion`, where `draw` gets `motion_ms()`; it redraws every `interval` while painted in the last 200 ms, for TASK-267
- Produces: `Motion::set_draw(&mut self, draw: impl Fn(u128, &mut Window, &mut App) -> AnyElement + 'static, cx: &mut Context<Motion>)` for TASK-267

Expected result:

- While an agent runs, the last timeline row is the running footer: a moving 6-dot loader, the fork button, and a clock that grows each second.
- When the turn ends, the running footer goes away and the completed footer ("Worked for ...") shows.
- Fork from the running footer opens a draft with the whole chat, the partial answer included.
- Cases:
  - Turn active, no tool call yet -> the footer shows, and the clock ticks.
  - Turn active, host without fork support -> no fork button.
  - Turn active, no `ActiveTurn.started_at` -> the clock counts from the user message.
  - Turn active, no start time and no loaded user message -> no clock; the loader and fork still show.
  - The turn ends while the fork menu is open -> the menu closes with the footer.
  - Turn not active -> no running footer.

Anti-goal: GPUI app CPU while one agent runs a quiet turn stays at or below the baseline plus 2 points, and at or below 10%; drift gauge; read with `packages/desktop-gpui/measure.sh 60 gpui=<check bundle binary>` while a live agent runs a foreground `sleep 90`, before changes and at completion. The 10% cap comes from [ADR-0039](watchtower/decisions/ADR-0039-incremental-timeline-sync-over-throttle-or-budget.md).

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-client -p bachuc-timeline` -> the new `motion` tests and every old test pass.
- `tests/motion.rs`: loader step -> 0 at 0 ms, 1 at 159 ms, 5 at 949 ms, 0 at 950 ms.
- `tests/motion.rs`: dot opacity -> step 0 gives `[1, 0, 0.78, 0, 0.56, 0.34]`; an unknown step or dot gives 0.
- `tests/motion.rs`: loader geometry for size 14 -> gap 2, dot 10/3 px, grid 2 dots wide and 14 px high.
- `tests/motion.rs` or a pure helper test: the footer start time -> `ActiveTurn.started_at` when it parses (milliseconds kept), else the user message time, else `None`.
- Read check of `sync` in `lib.rs`: the running footer is pushed only when `turn_active`, after the stream rows, and before the bottom card and the permission rows; it maps to `Below::Footer`; no fingerprint changes per loader step.
- Read check: the fork menu of the running footer emits `ForkBoundary::default()`, uses a key no entry can take, and `fork_menu` clears when the footer goes.
- Read check of `motion.rs`: the timer notifies only while painted in the last 200 ms, as `ring.rs` does, and draws time 0 when `reduce_motion()` is true.
- UI checker on live (screen check): ask a live agent to run `sleep 60` in the foreground. Raise the GPUI window, then take two screenshots 1 s apart. Expect the footer as the last row, with different dot opacities in the two shots, the fork icon, and a larger clock in the second shot.
- UI checker on live (screen check): press the fork button through AX -> the menu opens with "Fork in a new tab" and "Fork in a new workspace". Close it with Escape, and do not fork.
- UI checker on live (screen check): after the `sleep` ends -> the running footer is gone, and "Worked for ..." shows.
- Owner hand check: the loader looks like the Electron loader. Mark it UNVERIFIED (owner) in the outcome until Hiep checks it.
- Anti-goal: `measure.sh 60` before and after, with the same live `sleep 90` turn -> after is at most baseline + 2 points and at most 10%; record both values.
