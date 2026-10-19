# TASK-267 outcome

Status: DONE (screen checks deferred to the UI checker)

Changed:

- `packages/desktop-gpui/crates/timeline/src/motion.rs`: `pulse_opacity`, `shimmer_chars`, `shimmer_duration`, `shimmer_band_width`, `shimmer_band_left`, `shimmer_alpha`, `shimmer_phase`.
- `packages/desktop-gpui/crates/timeline/src/lib.rs`: the running strip segment fill and the running step number draw through `Motion`, and their draw closures.
- `packages/desktop-gpui/crates/timeline/src/tool_view.rs`: `shimmer_draw`, `shimmer_runs`, and the running badge label and summary drawn through the shimmer.
- `packages/desktop-gpui/crates/timeline/tests/motion.rs`: pulse, shimmer duration, band width, and band position tests.

Contract:

- `motion::pulse_opacity(ms) -> f32`: 1 at 0 ms, 0.3 at 700 ms, 1 at 1400 ms, on a cubic ease in-out.
- `motion::shimmer_chars`, `shimmer_duration`, `shimmer_band_width`, `shimmer_band_left`, `shimmer_alpha`, `shimmer_phase` as the Electron timing and band.
- Every pulse and shimmer draws through a cached `Motion` entity (`MotionKey::Segment`, `StepNumber`, `Badge`) and is dropped when its mark stops running.

Verified:

- `./cargo.sh test -p bachuc-timeline`: PASS (group-level run; 136 tests, 0 failed).
- `tests/motion.rs` pulse 1 / 0.3 / 1 at 0 / 700 / 1400 ms and strictly between 0.3 and 1 at 350 ms: PASS.
- `tests/motion.rs` shimmer duration 1.08 / 1.096 / 1.354 / 2.05 / 2.3 s for 10 / 12 / 13 / 100 / 200 characters: PASS.
- `tests/motion.rs` band width 42 / 66 / 120 px for 100 / 300 / 1000 px spans: PASS.
- `tests/motion.rs` at phase 0 the band ends at the span start, and at phase 1 it starts at the span end: PASS.
- Read check: the strip segment pulses inside the dimmed outer box; the whole running number (border, fill, digit) pulses; the running badge label and summary draw through `shimmer_draw`; `ensure_motions` prunes keys that stopped running.
- UI checker: three screenshots about 300 ms apart during a live turn -> DEFERRED (UI checker).
- Owner hand check: the pulse and shimmer look like Electron -> UNVERIFIED (owner).
- Anti-goal: CPU at or below 10% over 60 s during a live streaming turn -> DEFERRED (UI checker). Baseline and final reads were not taken by the builder.

Anti-goal: not measured by the builder; the CPU read of the GPUI app is a screen check and is deferred.

Lessons:

- GPUI has no `background-clip: text`, so the shimmer colors the glyphs under the band white at the gradient alpha and keeps the loading color outside it. The band is shaped twice per line per frame (geometry, then colored runs).
- The shimmer span is the label plus `SPACE_2` plus the summary at their shaped widths; a truncated summary runs into the clipped part, as Electron clips it.
- `LineLayout::x_for_index` gives byte-position x positions from a shaped line.
