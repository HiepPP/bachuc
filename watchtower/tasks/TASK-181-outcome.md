# TASK-181 Outcome

## Status

DONE. Six crates under `packages/desktop-gpui/plugins/<id>`: claude-look, stop-orb, idle-runtime-closer, plugins-nav, thread-context-attach, assistant-cite. Every `## Verify` command passes. The two `UI check:` lines are DEFERRED (UI checker).

## Changed

- New crates `bachuc-plugin-<id>`, each with `Cargo.toml`, `src`, and `tests/plugin.rs` (the id test). Rule tests sit next to the code, as in `bachuc-plugin-edit-diffs`.
- `packages/desktop-gpui/Cargo.lock` (generated). It also holds the lines of the other builder's `watchtower-board` crate.
- No file outside the six crates, the lock file, and this sidecar.

## Contract

Each crate exports only `pub fn plugin() -> Box<dyn bachuc_plugin_host::NativePlugin>`; `id()` is the plugin id.

- claude-look: one `ThemeContribution` `claude-light` / `Claude Light` / `Light`. `CLAUDE_LIGHT` holds the eight hex values of `theme.ts`; the test reads `theme.ts` and compares.
- stop-orb: `StopButton` `solving-orb`. `orb.rs` ports `frameRubik`, `makeProj`, `solveCycle`, `applyMoves`, `makeMoves`, `finalizeFrame`, `scaleCounts`, `scaleRadii`, `resolvePreset`, `inkColor`, and `solvingOrb` as `f64` pure functions. `face.rs` is one `OrbFace` view per agent id, kept in a map the closure owns. It paints dots with `canvas` and `paint_quad`, asks `request_animation_frame` only while not cancelling, uses `STOP_RED` (named constant, cites `stop-orb.tsx:8`), picks dark ink from `bachuc_theme::scheme`, and swaps only the `surface2` background and the clock speed on hover.
- idle-runtime-closer: `ComposerPill` `close-runtime` (Track, not on draft). `state` returns `None` while `SHOW_PILL` is `false`. The press path is `refresh_agent`, then `check_closable(status, pending)`, then `close_agent_runtime`. A refusal or failure calls `show_message(.., MessageLevel::Error, ..)` with the Electron texts.
- plugins-nav: `SidebarItem` `plugins` (`Plugins`, `Blocks`, surface `plugins`, press calls `open_plugins_page`, `requires_workspace` false) and an empty `Surface` `plugins`.
- thread-context-attach: `AttachmentSource` `thread` (`Thread`, `MessagesSquare`) with the picker title, placeholder, and `thread-context.search`. No `on_open`.
- assistant-cite: chip-only `AttachmentSource` `quote` (`Quote`, no picker, `on_open` reveals the passage only when the chip `server` equals `Client::server_info().server_id`) and `SelectionAction` `cite` (builds the item, calls `add_composer_attachment` with `commentable: true`). `format.rs` and `item.rs` port `shared/format.ts` and `shared/item.ts` with UTF-16 length, the JS whitespace set, and FNV-1a over code units in base 36.

## Verified

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-plugin-claude-look` -> pass, 3 tests incl. `palette_matches_theme_ts`, `contributes_one_light_theme`.
- `... test -p bachuc-plugin-stop-orb` -> pass, 10 tests incl. `solving_orb_picks_nearest_preset`, `frame_dots_sorted_by_depth`, `frame_radius_floor`, `ink_ramp_dark_and_light`, `hover_speed_multiplier`, `cancelling_freezes_frame`, `one_frame_under_budget` (1,000 frames in 0.02 s), and `frame_matches_the_library` (30 golden dots from `MODE_FRAMES.rubik(20, 0.6, ..)` run in Node).
- `... test -p bachuc-plugin-idle-runtime-closer` -> pass, 6 tests incl. `pill_stays_hidden`, `close_refused_for_busy_agents`.
- `... test -p bachuc-plugin-plugins-nav` -> pass, 2 tests incl. `sidebar_item_and_surface_ids`.
- `... test -p bachuc-plugin-thread-context-attach` -> pass, 3 tests incl. `picker_uses_search_method`, `search_payload_decodes`.
- `... test -p bachuc-plugin-assistant-cite` -> pass, 28 tests: one per case of `format.test.ts` and `item.test.ts`, plus `item_id_and_url_match_js` (strings from Node: `msg-1:6n05hy`, `paseo-cite://quote?server=host-1&agent=agent-1&message=msg-1`), `length_counts_utf16_units`, `other_host_chip_reveals_nothing`.
- `rg -n "bachuc-(composer|sidebar|timeline|command|settings|workspace|explorer|app)" .../Cargo.toml` -> no match (exit 1).
- `rg -n "gpui-kit" .../Cargo.toml` -> one line per crate, `gpui-kit.workspace = true`.
- `packages/desktop-gpui/cargo.sh fmt --check -p <six crates>` -> exit 0.
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> exit 0, run twice.
- UI check, stop-orb in the composer: DEFERRED (UI checker).
- UI check, `Plugins` sidebar row: DEFERRED (UI checker).
- Live checks: UNVERIFIED (unattended): live daemon down.
- Press handlers (`open_plugins_page`, `close_runtime`, `cite`, `open_quote`) need `App` and `PluginContext`; the host has no headless context. Read-checked only; their rules (`check_closable`, `reveal_target`, `cite_attachment`) are tested.

## Anti-goal

Count of `ls packages/desktop-gpui/target/debug/deps/libgpui_kit-*.rlib | wc -l`:

| Checkpoint                                | Value | Source   | Time             |
| ----------------------------------------- | ----- | -------- | ---------------- |
| before the first crate                    | 6     | ls count | 2026-10-16 23:43 |
| after the first crate (claude-look) built | 6     | ls count | 2026-10-16 23:43 |
| after six crate tests, first app build    | 6     | ls count | 2026-10-17 00:00 |
| after the second app build                | 6     | ls count | 2026-10-17 00:01 |

The count stays 6. The app does not depend on these crates yet (TASK-191), so the app builds do not rebuild gpui-kit.

## UI Checks

Checked 2026-10-17 by the UI checker with `open -g`, AX reads, AX presses, and window screenshots (no pointer, no foreground). Electron 6770 shows every plugin as `failed` (their directories point to `~/Projects/paseo`), so it draws no plugin UI; for placement the stock Electron rows are the reference. Measures in points (screenshot px / 2) from `/tmp/wt-ui/gpui-initial.jpg` and `/tmp/wt-ui/electron-initial.jpg`.

- UI check (stop-orb: red solving orb in the stop button's place, with an agent running) -> UNVERIFIED (unattended): needs pointer. No running agent view can open (AX press on sidebar rows does nothing: `/tmp/wt-ui/gpui-t144.jpg`), a prompt to start one is not allowed, and the reference has no plugin.
- UI check (`Plugins` sidebar row with the blocks icon, placed as in Electron 6770) -> FAIL on placement. The row exists with the blocks icon, after `Search` and `Board` (`/tmp/wt-ui/gpui-initial.jpg`). Row centres are y 91, 123.5, 156.75, a pitch of about 33 pt. The Electron nav rows (`New workspace`, `History`, `Search`) sit at y 81, 110, 140, a pitch of 30 pt, and plugin rows use the same rows in the source (`sidebar-nav-rows.tsx`). The GPUI nav rows are about 3 pt taller. The Electron build itself shows no `Plugins` row (plugins-nav `failed` there).

## Lessons

- `use gpui_kit::*` also imports GPUI's `test` macro, so `#[test]` fails with "recursion limit reached" in any module that does `use super::*` over it. Import gpui-kit items by name in files that hold tests.
- `Contributions` fields are `pub(crate)`, but `ContributionIndex::build([(id, &contributions)])` has public getters. A plugin crate can test the shape of a contribution by building it with a pure function and a no-op handler.
- Port JS string rules with JS semantics: `length` is UTF-16 units, `trim` and `\s` add U+FEFF and drop U+0085 against Rust, `split(/\r?\n/)` leaves a lone `\r`, and a one-pass `replace` differs from chained `str::replace`. Run the real JS in Node to pin golden strings.
- The `thinking-orbs` internals are exported by short names (`M` = `MODE_FRAMES`, `r` = `resolvePreset`, `a` = `scaleRadii`), so Node can print golden frames for a port test.
- Defaults taken (no question needed): the faces map keeps one entity per agent id until the plugin deactivates; the frame time jumps when hover starts or stops, as the library's effect restart does; a title cut that splits a surrogate pair drops the half pair, where JS keeps a lone surrogate; the spec cites `STOP_RED` at `composer.rs:43`, which is line 80 now, so the constant cites only the Electron source.

## Fix Round 1

2026-10-17, code tests only. The lead re-runs the screen check.

- Cause of the 33 pt pitch: Search and the plugin nav rows (drawn by `crates/sidebar`) used GPUI's default line height 1.618, so each 28 px `minHeight` row grew to 22.7 + 8 = 30.7 px. Electron `sidebar-header-row.tsx` (compact: `minHeight` 28, `paddingVertical` 4) draws the label at about 1.2, so its rows stay 28 px; with the 2 px group gap the pitch is 30.
- Change: both rows use `NAV_ROW_MIN` 28 and `line_height(LABEL_LINE)` 20 from the new `crates/sidebar/src/metrics.rs`, so each row is 28 px and the pitch is 30 (Electron 30). The text centre in the row does not move.
- AX press: the `Search`, `Board`, and `Plugins` rows now call their handler from `AccessibleAction::Click` too; see TASK-175 Fix Round 1 for the full list and reason.
- Tests: `CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-sidebar` -> all pass, new `metrics::tests::nav_rows_are_30_px_apart`; `./cargo.sh fmt --check -p bachuc-sidebar` -> exit 0.
- UI check (`Plugins` row placement, nav pitch 30) -> DEFERRED (UI checker). UI check (stop-orb) -> DEFERRED (UI checker): it needs an agent view, which the AX press fix may now open.

## UI Checks, round 2

Checked 2026-10-17 by the UI checker with `open -g`, AX reads, AX presses, and window screenshots (no pointer, no foreground). Electron reference: the round-1 screenshot `/tmp/wt-ui/electron-initial.jpg` (no plugin rows there). Measures in points from `/tmp/wt-ui2/initial.jpg` (pixel scan).

- UI check (`Plugins` sidebar row with the blocks icon, placed as in Electron 6770) -> PASS. Icon centres are y 90, 120, and 150 (`Search`, `Board`, `Plugins`): a pitch of 30 pt, as in Electron (81, 110, 140). The `Plugins` row fill spans y 136-164, 28 pt tall (round 1: pitch 33). The first GPUI row is 9 pt lower than Electron's because the GPUI sidebar has a host row above it. Evidence: `/tmp/wt-ui2/initial.jpg`.
- UI check (stop-orb: red solving orb in the stop button's place, with an agent running) -> UNVERIFIED (unattended): no agent can run. The only mock agent that opens (`wt-long-thread`) has a working directory that does not exist, so a send fails with "Working directory does not exist: /tmp/wt-long-thread" (`/tmp/wt-ui2/run-agent-2.jpg`). The reference has no plugin. The test draft text was cleared.
