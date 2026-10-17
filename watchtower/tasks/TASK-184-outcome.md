# TASK-184 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/plugins/edit-diffs` (new crate `bachuc-plugin-edit-diffs`):
  - `Cargo.toml`: `bachuc-client`, `bachuc-icons`, `bachuc-plugin-host`, `bachuc-theme`, `gpui-kit.workspace = true` (no `features` key), `log`, `serde`, `serde_json`. No view crate.
  - `src/lib.rs`: `pub fn plugin()`, the `edit-diff` transformer, the renderer, the `display` settings screen, and the `toggle-thread` command.
  - `src/diff.rs`: `line_diff`, `parse_unified_diff`, `build_hunks`, `side_text`, `needs_locating`, `apply_locations`, `number_rows`, `count_changes`, `change_bar`.
  - `src/edit_item.rs`: `to_edit_row` with the `errorText` rule, `edit_diffs_off`, `toggled_mode`, `EditRowData` (the `editRowSchema` decode), `display_path`, `header_label`.
  - `src/state.rs`: the expand rule, the settings screen states, the locate request and answer shapes, and `Locator` (300 ms settle, answers per status and request, stale answers dropped).
  - `src/highlight.rs`: `highlight_hunks`, with copies of the extension map, the capture-to-role map, and `split_lines` of `crates/explorer/src/syntax.rs`.
  - `src/row.rs`: the `EditDiffRow` view. `src/settings_view.rs`: the "Edit diffs" screen.
  - `tests/plugin.rs`: `plugin().id()`.
- `packages/desktop-gpui/crates/timeline/src/tool_view.rs`: removes `render_edit`, `change_bar`, `edit_status_label`, `display_path`, and the edit-only constants.
- `packages/desktop-gpui/crates/timeline/src/lib.rs`: removes the re-exports, the `Diff` cache and its build in `update_hashes`, `hidden_diffs`, and `toggle_diff`. A step draws a kept edit call with `render_tool_call` inside `BADGE_BLEED`. Adds the lazy `edit_stats` count for the turn header.
- `packages/desktop-gpui/crates/timeline/src/plugins.rs`: `StepPart::Edit` is now `StepPart::ToolCall`.
- `packages/desktop-gpui/crates/timeline/tests/tools.rs`: the edit-row asserts are gone; they moved to the plugin crate.
- `packages/desktop-gpui/crates/timeline/tests/plugins.rs`: `StepPart::ToolCall`, and a new step test.
- `packages/desktop-gpui/Cargo.lock` (generated): the `bachuc-plugin-edit-diffs` entry only. The `bachuc-app` lines in the file are group D's.

## Contract

- Produces `bachuc_plugin_edit_diffs::plugin() -> Box<dyn bachuc_plugin_host::NativePlugin>`, id `"edit-diffs"`, for TASK-191. It is the crate's only public item.
- Contributions:
  - Transformer `edit-diff` for `tool_call`. It returns `None` for `edit-diffs.mode == "off"` or a `detail.type` other than `edit` or `write`. Else one item, kind `edit-diff`, version 1, with the `toEditRow` data, and `source: None`.
  - Renderer `edit-diff` version 1. Data that does not decode draws "Plugin timeline item unavailable." in `foreground_muted`.
  - Settings screen `display`, "Edit diffs", `IconName::PencilOff`.
  - Command `toggle-thread`, context `Agent`, `IconName::PencilOff`. It sets only `edit-diffs.mode` with `set_agent_labels`. An `Err` shows with `show_message` at `MessageLevel::Error`.
- Consumes the TASK-170 host kinds, `PluginContext::{client, rpc_as, settings, show_message}`, `PluginSettings`, `ui::{section, card, switch_row, action_row, note, RowText}`, the TASK-168 `Client::{agent_labels, set_agent_labels, agents}`, the TASK-178 `step_parts` and `call_listed`, and the TASK-195 `IconName::PencilOff`, all with their exact names.
- Safe defaults and choices, for the lead:
  - The turn header still sums `+N -M` over a turn's edits, as Electron's `sumLineStats`. With the `Diff` cache gone, a stats-only cache, `edit_stats`, counts each edit once per `seqEnd` when a header draws. No sync builds a diff. The counts keep today's 400-line cut of `diff::entry_lines`.
  - The item `id` is the call's `callId`. The timeline builds a new view on each data change (TASK-178 `ensure_view`). So a per-call `RowMemory`, shared while any view of the call lives, holds the press override and the locate answers. This keeps "for the view's life" across a streaming call's status changes. Two panes of one agent share that memory.
  - Locate waits until the request has been unchanged for 300 ms, counted from when the view first sees it. React's `useSettled` starts settled, so Electron sends a stable request at mount. The spec text wins.
  - A locate error is logged and not sent again for that key. React Query retries a failed query 3 times.
  - The row has no `-13` bleed: Electron's row bleeds and the step's `pluginEdit` style cancels it. A GPUI step draws plugin items without that style.
  - The hunk body is a virtualized `uniform_list` with unconstrained width. It shows every line, as the TS row does; the old timeline row cut at 400 lines.
  - `serde` reads `null` optional strings as absent, where zod rejects them.
  - Missing shared helper: the extension map, role map, and `split_lines` are copied from `bachuc-explorer`, a view crate. A shared crate would let one copy serve both.

## Verified

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-plugin-edit-diffs` -> exit 0, 36 unit and 1 integration passed, 0 failed (`/tmp/wt-h184-v1.txt`).
  - Ports: 6 of `diff.test.ts`, 5 of `change-bar.test.ts`, 2 of `edit-item.test.ts`.
  - Spec cases: the label gate, the non-edit gate, the two-line write, the 600 by 600 fallback, and the header label (running, failed, completed). `changeBar(3, 1)` is 5 and 2, and `changeBar(100, 1)` is 6 and 1.
  - Spec cases: `displayPath` with `/private`; the expand rule with loading, error, invalid, and override; the locate gate (collapsed, located, expanded); the 300 ms settle and its restart.
  - Spec cases: the stale answer dropped and answers kept per status; the toggle label flip; the settings states (loading, error, invalid, a conflict that keeps the value); `plugin().id() == "edit-diffs"`; the 1,000 by 1,000 timing test.
  - The press handlers that call `reset` and `save` need a window. They are checked by reading only: UNVERIFIED (unattended): code tests only.
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-timeline` -> exit 0, 80 passed, 0 failed, as the baseline (`/tmp/wt-h184-v2.txt`). `incremental_rows_match_full_regroup` and `counts_paseo_edit_rows_as_edits_when_edit_diffs_are_off` pass with no test change. `an_edit_call_without_a_projection_draws_as_a_tool_call_in_its_step` passes.
- `rg -n "render_edit|hidden_diffs|toggle_diff" packages/desktop-gpui/crates/timeline/src` -> no match (exit 1).
- `rg -n "bachuc-(composer|sidebar|timeline|command|settings|workspace|explorer|app)" packages/desktop-gpui/plugins/edit-diffs/Cargo.toml` -> no match (exit 1).
- `rg -n "gpui-kit" packages/desktop-gpui/plugins/edit-diffs/Cargo.toml` -> one line, `13:gpui-kit.workspace = true`.
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-plugin-edit-diffs -p bachuc-timeline` -> exit 0.
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> exit 0, `Finished dev profile` (`/tmp/wt-h184-app.txt`). Its one warning, `next_theme` never used, is in `crates/app`, group D's crate.
- GitNexus impact: the index is 19 commits behind and does not find `step_parts`. `rg` confirms the removed and renamed symbols are used only in `crates/timeline` and its tests.
- The edit row, settings screen, and command on screen -> UNVERIFIED (unattended): code tests only. TASK-191 registers the crate and carries the screen check against Electron 6770.

## Anti-goal

- Checkpoint 0, before the first change: `bachuc-timeline` 80 passed; the tripwire `rg` had 9 matches. Source: `/tmp/wt-h184-base.txt`, 2026-10-16 23:26.
- Checkpoint 1, after the logic port: a 1,000 by 1,000 line edit built its hunks and numbered rows in 205.6 us in a debug test, under 50 ms. The tripwire `rg` still had 9 matches, because the timeline was not changed yet. Source: `test -p bachuc-plugin-edit-diffs --lib 1000_by_1000 -- --nocapture`, 23:34.
- Checkpoint 2, at completion: 670.9 us by the same command, 23:39. The tripwire `rg` had no match, 23:38.
- Tripwire: not tripped.

## Lessons

- The gpui glob import `use gpui_kit::*` brings its own `test` attribute. A unit test module under such a file must not `use super::*`, or `#[test]` hits the recursion limit.
- The timeline builds a new plugin view whenever the item data changes. A plugin that must keep view state across a streaming item keys it by a stable item id outside the view.
