# TASK-267 Running pulse and shimmer in the GPUI timeline

Group: B (the `bachuc-timeline` crate; shares `lib.rs` and `motion.rs` with TASK-266)
Class: risky

## Brief

Goal: The running step and the running tool call move in GPUI as they do in Electron: the step number and its strip segment pulse, and the running tool row label shimmers.

Change: static running marks in the timeline -> the Electron pulse and shimmer, drawn through the TASK-266 `Motion` view.

Design: `usePulse` in [packages/app/src/tool-calls/detail-level/turn/view.tsx](packages/app/src/tool-calls/detail-level/turn/view.tsx) (lines 20-26 and 170-196), and the tool row shimmer in [packages/app/src/components/message.tsx](packages/app/src/components/message.tsx) (`ExpandableBadgeWebShimmerOverlay` at line 2388, the timing at lines 2600-2640, `SHIMMER_GRADIENT` and `buildShimmerTextStyle` at lines 2671-2695).

Boundaries:

- Pulse, from `usePulse`: opacity goes from 1 to 0.3 in 700 ms and back to 1 in 700 ms, then loops. `Animated.timing` uses its default ease in-out curve. Port it as a pure function of the motion clock.
- Strip segment (`lib.rs:2318-2341`): the running segment's fill pulses. A dimmed segment keeps its 0.3 opacity on the outer box, as Electron puts the pulse on the inner fill.
- Step number (`lib.rs:2455-2471`): the whole running number pulses (border, fill, and digit), as Electron puts the opacity on the number view.
- Shimmer: a running tool call (`is_loading`, `tool_view.rs:189`) shows a light band that moves across its label and summary. Read the full Electron path before porting. The duration in seconds is `max(1, min(2.3, 1.25 + chars * 0.008 - (chars <= 12 ? 0.25 : 0)))`, where `chars` counts the trimmed label and summary. The band width is `max(42, min(120, span * 0.22))` px. The band runs from `span start - band width` to `span end`, linear, and loops. Its colors follow `SHIMMER_GRADIENT`.
- Planner call: GPUI has no `background-clip: text`, so color the glyphs under the band through text runs (`StyledText::with_highlights`, as `tool_view.rs:390` uses). Take glyph x from a shaped line (`LineLayout::x_for_index`). Keep the current loading color (`LOADING_OPACITY`, `tool_view.rs:258-260`) outside the band.
- Planner call: the span is the label plus the summary (`tool_view.rs:263`, a separate div with `ml(SPACE_2)` and `truncate()`) at their shaped widths. When the summary is truncated, the band runs on into the clipped part, where it is not seen. Only the loop timing differs from Electron.
- Planner call: port `SHIMMER_GRADIENT` as is, a white band, in both themes, as Electron does.
- Use a 33 ms `Motion` interval for the pulse and the shimmer, which move smoothly.
- Use the `Motion` view of TASK-266 for every animated mark here, so a pulse redraws only its mark and not the timeline list. Cache one `Entity<Motion>` per mark by its row key, and drop it when the mark stops running.
- Planner call: all pulses and shimmers read the one shared motion clock, so they move in step. Electron starts each loop at mount; the difference is not visible at rest.
- Follows [ADR-0039](watchtower/decisions/ADR-0039-incremental-timeline-sync-over-throttle-or-budget.md): no row fingerprint changes per frame.

How:

- Add `pulse_opacity(ms: u128) -> f32` and the shimmer band math as pure functions in `motion.rs`.
- Draw the running strip segment fill and the running step number through `Motion`.
- Draw the running tool row label and summary through `Motion` with the shimmer runs.
- Add tests to [packages/desktop-gpui/crates/timeline/tests/motion.rs](packages/desktop-gpui/crates/timeline/tests/motion.rs).

Files:

- [packages/desktop-gpui/crates/timeline/src/motion.rs](packages/desktop-gpui/crates/timeline/src/motion.rs) (pulse and shimmer pure functions)
- [packages/desktop-gpui/crates/timeline/src/lib.rs](packages/desktop-gpui/crates/timeline/src/lib.rs) (strip segment and step number through `Motion`)
- [packages/desktop-gpui/crates/timeline/src/tool_view.rs](packages/desktop-gpui/crates/timeline/src/tool_view.rs) (running label and summary shimmer)
- [packages/desktop-gpui/crates/timeline/tests/motion.rs](packages/desktop-gpui/crates/timeline/tests/motion.rs) (new tests)

Interfaces:

- Consumes: `bachuc_timeline::motion::motion_ms() -> u128` (milliseconds on the one shared motion clock) from TASK-266
- Consumes: `bachuc_timeline::motion::Motion` view, built with `Motion::new(interval: Duration, draw: impl Fn(u128, &mut Window, &mut App) -> AnyElement + 'static, cx: &mut Context<Motion>) -> Motion`, where `draw` gets `motion_ms()`; it redraws every `interval` while painted in the last 200 ms, from TASK-266
- Consumes: `Motion::set_draw(&mut self, draw: impl Fn(u128, &mut Window, &mut App) -> AnyElement + 'static, cx: &mut Context<Motion>)` from TASK-266

Expected result:

- While a step runs, its number and its strip segment fade to 0.3 and back every 1.4 s.
- While a tool call runs, a light band moves across its label and summary.
- When the step or the tool call ends, the mark is still and drawn as today.
- Cases:
  - A dimmed running segment -> it pulses inside the dimmed box.
  - A short label of 12 characters or fewer -> the loop is 0.25 s shorter; 13 characters is not.
  - A label with no summary -> the band runs over the label alone.
  - A finished turn -> no `Motion` view stays alive for it.

Anti-goal: GPUI app CPU during a streaming live turn stays at or below 10% over 60 s; drift gauge; read with `packages/desktop-gpui/measure.sh 60 gpui=<check bundle binary>` while a live agent runs a turn with several tool calls, before changes and at completion. The 10% cap comes from [ADR-0039](watchtower/decisions/ADR-0039-incremental-timeline-sync-over-throttle-or-budget.md).

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-timeline` -> the new tests and every old test pass.
- `tests/motion.rs`: `pulse_opacity` -> 1 at 0 ms, 0.3 at 700 ms, 1 at 1400 ms, and strictly between 0.3 and 1 at 350 ms.
- `tests/motion.rs`: shimmer duration -> 10 chars gives 1.08 s, 12 chars gives 1.096 s, 13 chars gives 1.354 s, 100 chars gives 2.05 s, 200 chars gives 2.3 s.
- `tests/motion.rs`: band width -> a 100 px span gives 42 px, a 300 px span gives 66 px, a 1000 px span gives 120 px.
- `tests/motion.rs`: band position -> at phase 0 the band ends at the span start, and at phase 1 it starts at the span end.
- Read check: each pulse and shimmer draws through `Motion`, and a `Motion` entity is dropped when its mark stops running.
- UI checker on live (screen check): ask a live agent to run three `sleep 5` commands one by one. Raise the GPUI window and take three screenshots about 300 ms apart. Expect a changed opacity of the running step number and segment, and a moved band on the running tool label.
- Owner hand check: the pulse and the shimmer look like Electron. Mark it UNVERIFIED (owner) in the outcome until Hiep checks it.
- Anti-goal: `measure.sh 60` before and after, during a live turn with tool calls -> at most 10%; record both values.
