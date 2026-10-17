# TASK-193 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/plugins/board` (crate `bachuc-plugin-board`, no new dependency):
  - `src/lib.rs`: `contribute` reuses the one `BoardStore` and adds the settings screen `thinking-orb` "Thinking orb" (`Sparkles`), the header subtitle `project`, and three corner pills. `plugin()` is still the only public item.
  - `src/orb/mod.rs`, `src/orb/modes.rs`: the engine of the seven offered states (`rubik`, `orbits`, `globe`, `wave`, `braid`, `ribbon`, `web`) with `lerp`, `frac`, `vnoise`, `hashD`, `fibDir`, `angleDelta`, `makeProj`, `radiusScale`, `scaleCounts`, `scaleRadii`, `finalizeFrame`, `solveCycle`, `makeMoves`, `applyMoves`, `BASE_PROFILES`, `PRESETS`, `resolvePreset`, and the monochrome `inkColor`. Each mode is `(size, t, opts) -> Frame` (sorted dots plus the lines of `web`). `ring` and `morph` are not ported.
  - `src/orb_view.rs`: `orb_avatar` (avatar at 0.9 of the size, round, at `avatarOpacity / 100`, and a grayscale copy under a 0.6 color copy for `saturate(0.6)`), with the orb on a GPUI `canvas` at `sphere_canvas(avatar, state)`. Only the canvas paint asks for the next frame, and only when its bounds meet the content mask.
  - `src/card.rs`: `running_avatar` draws `orb_avatar` when `orb.enabled`, else the spinner.
  - `src/model/timeline.rs`: `decode_timeline` (one thread at a time), `timeline_model`, `duration_text`, `clock_text` (local time), `timeline_notes`, `timeline_kind`, bar text, label, colors, footer, host meta and notes.
  - `src/model/recaps.rs`: `decode_recaps` (one day, project, and entry at a time), `recap_day_markdown`, `recap_meta`, `recap_items`, `CopyState`, `recaps_body`.
  - `src/model/view_poll.rs`: `ViewPoll` (only while shown and online, one request in flight, one per interval), `minute_of`, `until_next_minute`.
  - `src/timeline.rs`: the Timeline view. Legend, load strip, axis, host header, notes, and footer are fixed; one `list` item per thread row and per subagent row. The model rebuilds on new data, a new minute, a new chart width, or a new scale.
  - `src/recaps.rs`: the Recaps view under the lane rail ("Last 7 days"), one `list` item per day header, project header, and entry, and Copy per day.
  - `src/orb_settings.rs`, `src/header_chip.rs`, `src/pills.rs`: the settings screen, the project chip, and the pills.
  - `src/model/settings.rs`: `OrbState::ALL`, `OrbState::id`, `sphere_canvas`. `src/store.rs`: `context`, `open_watchtower` (`open_task` calls it). `src/avatar.rs`: `avatar_image`. `src/runs.rs`: `rail` takes a `meta` text and `palette_notice` is shared. `src/page.rs`: the Timeline and Recaps arms.
- `packages/desktop-gpui/Cargo.lock`: unchanged.
- No file in `hiep-plugins/plugins/board` changed.

## Contract

- Consumes every TASK-189, TASK-170, and TASK-195 name of the spec as written; none was missing.
- Produces no public item; `plugin()` stays the only one, for TASK-191.
- Safe defaults taken (reported as questions):
  - A timeline TASK chip calls the new `BoardStore::open_watchtower(workspace_id, task)`. `open_task` takes a `Run`, and a timeline bar has no `Run`; `open_task` now calls the same method.
  - "Store error" for the pills is the store connection `Error` or `Offline`. Electron hides the pills on any failed `board.snapshot`, and an offline host fails it.
  - `cx.write_to_clipboard` returns nothing, so Copy reads the clipboard back: the same text shows "Copied", else "Copy failed".
  - GPUI has no saturate filter: a grayscale copy of the avatar sits under the color copy at 0.6.
  - The Timeline uses the 230 px label column only; the compact 112 px column is not ported, as the Runs view keeps no compact layout.
  - Project order inside the Timeline sorts `projectKey` by byte order, not `localeCompare`.

## Verified

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-plugin-board` -> exit 0: 91 unit tests and 1 integration test pass. New tests cover each expected case: the Electron `timeline.test.ts` client cases (6 h 40 m break, 90 min gap, running thread and subagents, host and project order, load peak, empty chart), `duration_text(90_000)` "2m" and `(3_600_000)` "1h", `clock_text` in local time, `timeline_notes` (none, `olderDropped`, one and two missing starts), `recap_day_markdown` with did, branch, commit and with none, `orb_preset(16)` -> (20, 0.8), `sphere_canvas(32, Weaving)` -> 32 / 0.76, the ported `orb.test.ts` cases, each of the seven modes (dots sorted by depth, radius at least 0.3, equal frames at equal times, lines only for `web`), the five pill cases, and the decode of real `board.timeline` and `board.recaps` payloads with one bad row each. Every TASK-189 test is unchanged (no removed test line in the diff).
- 1,000-frame timing per mode at the 32 px preset (debug): Solving 38 ms, Working 24 ms, Searching 21 ms, Listening 12 ms, Weaving 13 ms, Composing 65 ms, Connecting 7 ms; limit 500 ms.
- `rg -n "bachuc-(composer|...|plugin-[a-z-]+)\b" packages/desktop-gpui/plugins/board/Cargo.toml | rg -v plugin-host` -> one match, line 2 `name = "bachuc-plugin-board"`. UNSATISFIABLE: the pattern matches the crate's own required name (TASK-189 lesson). Replacement: the same grep over the `[dependencies]` section only -> no match (exit 1).
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-plugin-board` -> exit 0.
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> exit 0. `bachuc-app` does not link the crate until TASK-191.
- `git diff --stat -- hiep-plugins/plugins/board` -> empty.
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh clippy -p bachuc-plugin-board --tests` -> no warning in this crate after two fixes.
- UI check (Timeline, running orb, and corner pills against Electron 6770) -> DEFERRED (UI checker). It also needs TASK-191 to link the crate.
- Live daemon checks -> UNVERIFIED (unattended): live 6768 is down.

## Anti-goal

- Checkpoint 1, when the engine landed: name 1,000-frame timing; value 7 to 64 ms per mode, all under 500 ms; source `orb::tests::a_thousand_frames_of_each_mode_stay_under_500_ms`; time 2026-10-17 00:48.
- Checkpoint 2, at completion: name frame requests; value `rg -n request_animation_frame src` finds one call, `src/orb_view.rs:119`, in the orb canvas paint after the `in_view` check, so no frame is asked while no orb shows (`only_an_orb_inside_the_visible_area_asks_for_frames` passes); source `packages/desktop-gpui/plugins/board/src`; time 2026-10-17 00:50.
- Checkpoint 3, at completion: name orb timing rerun; value same range, 91 of 91 pass; source `cargo.sh test -p bachuc-plugin-board -- --nocapture`; time 2026-10-17 00:50.
- Tripwire not hit.

## UI Checks

Checked 2026-10-17 by the UI checker with `open -g`, AX reads, AX presses, and window screenshots (no pointer, no foreground).

- UI check (Timeline, running orb, and corner pills against Electron 6770) -> UNVERIFIED (unattended): needs pointer. AX press on the sidebar `Board` button changed nothing (`/tmp/wt-ui/gpui-board.jpg`), so the Board view cannot open. The line also needs a running thread, and the reference lacks the plugin (`board` is `failed` on 6770).

## Lessons

- A test module that imports `super::*` from a file with `use gpui_kit::*` gets GPUI's own `test` attribute, and the build fails with "recursion limit reached while expanding `#[test]`". Import the tested names one by one.
- GPUI `Styled::flex_shrink` takes a factor; use `flex_shrink(1.)`.
- A GPUI `list` paints its overdraw too, so an animation that asks for frames from paint must check `window.content_mask().bounds` first.
- `git diff` in this checkout runs through a wrapper that indents lines; use `git --no-pager diff --no-ext-diff` and grep `^  -` for removed lines.
