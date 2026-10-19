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

## UI Checks

Run 2026-10-19 on live (6768) with the check bundle, launched with `BACHUC_NO_ACTIVATE=1` and driven by `cua_repl` AX actions only. Agent `bf4a43a8` ran `sleep 25` three times, then a `sleep 90` turn whose running step was selected to show its Shell badge. The app was quit and the agent archived.

- UI checker on live: three screenshots about 300 ms apart during a running Bash call -> FAIL. The step number and strip segment pulse: the number region mean fell 250.0 -> 249.2 -> 247.7, and the strip bar RGB went (186,216,242) -> (135,189,234) -> (42,137,220); five shots over 2 s gave five different bar colors. The running tool badge ("Shell sleep 90") did not shimmer: its label and summary pixels were byte-identical across the five shots over 2 s (mean diff 0.0). Likely cause, read from source: `ensure_motions` reads `entry["status"]`, but entries store status at `entry["item"]["status"]`, so no `MotionKey::Badge` motion is created (`packages/desktop-gpui/crates/timeline/src/lib.rs:1707-1711`); `render_badge` then draws its static label and summary. Pulse PASS; shimmer FAIL.
- Plan Verify (NEXT.md): the running step pulses and the running tool label shimmers -> FAIL. The pulse passes; the shimmer does not draw, from the check above.
- Anti-goal: 28.2% over 60 s during a live turn with several tool calls -> FAIL against the 10% cap. Baseline comparison UNVERIFIED (unattended); no pre-change read exists.

## Fix Round 1

Status: DONE (screen checks deferred to the UI checker)

Changed:

- `packages/desktop-gpui/crates/timeline/src/lib.rs`: `ensure_motions` reads the tool status through `motion::tool_entry_running(entry)`, which reads `entry["item"]["status"]` instead of `entry["status"]`. It also builds the badge's name and summary from `entry["item"]`, the level `render_badge` and `tools::presentation` read. The running badge now gets a `MotionKey::Badge` motion.
- `packages/desktop-gpui/crates/timeline/src/motion.rs`: `tool_entry_running`, `pulse_state`, `shimmer_state`, `frame_draws`. The pulse and shimmer marks carry a quantized state; the shared ticker skips a frame whose state did not change.
- `packages/desktop-gpui/crates/timeline/tests/motion.rs`: a running `tool_call` entry in the daemon shape gets its shimmer gate and a moving state; an unchanged quantized frame and a not-due frame do not draw.

Contract:

- `motion::tool_entry_running(entry: &serde_json::Value) -> bool`: true when `entry["item"]["status"]` is `running` or `executing`; false when the status sits on the wrapper.
- `motion::pulse_state(ms) -> u64`: opacity in 24 levels. `motion::shimmer_state(ms, duration) -> u64`: band position in 24 steps.
- `motion::frame_draws(elapsed, interval, changed) -> bool`: a frame draws only when due and changed.
- Pulse, shimmer, and band math are unchanged; only the sampling rate and quantization changed.

Verified:

- `./cargo.sh test -p bachuc-timeline`: PASS (group-level run; 138 tests, 0 failed; `tests/motion.rs` 11 passed).
- `tests/motion.rs` `a_running_tool_call_gets_its_shimmer_motion`: PASS (running and executing true, completed false, wrapper-level status false, shimmer state moves).
- `tests/motion.rs` `an_unchanged_quantized_frame_is_skipped`: PASS (not due and unchanged do not draw; due and changed draws).
- Read check: the gate in `ensure_motions` is `item_type(entry) == "tool_call" && motion::tool_entry_running(entry)`; `render_badge` looks up `MotionKey::Badge(entry_key)`; both read the same key and the item level.
- UI checker: three screenshots about 300 ms apart during a running Bash call -> DEFERRED (UI checker). The shimmer could not be measured by the builder.
- Owner hand check: the pulse and shimmer look like Electron -> UNVERIFIED (owner).

Anti-goal: not measured by the builder; the CPU read is a screen check (see TASK-266 Fix Round 1 for the timer change).

Lessons:

- A daemon timeline entry is `{seqStart, seqEnd, item, ...}`; the tool fields are under `entry["item"]`. Reading `entry["status"]` silently yields `""`, so no badge motion was created and the static label drew.
- The animation interval is now 66 ms (15 fps), raised from 33 ms to fit the 10% CPU cap; the loader keeps its 158 ms step. The pulse fades in 24 levels and the shimmer band steps about 10 px per frame. Electron animates at 60 fps, so the GPUI motion is coarser; the UI checker must judge whether it still reads as motion.
- A mark notifies its own entity; GPUI then marks the timeline that holds it dirty and re-renders it, so one frame is the unit of cost, not one mark.

Questions:

- Is 15 fps acceptable for the pulse and the shimmer, or should the next round trade CPU budget for a higher rate?
