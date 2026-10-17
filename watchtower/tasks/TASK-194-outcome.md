# TASK-194 Outcome

## Status

DONE

## Changed

- `src/model/overview_view.rs` (new): every derived-data and text rule of the overview, with 15 tests: `layout_for` and `Layout`, `run_state_text`, `needs_you_rows`, `pace_items`, `reject_step`, `actions_disabled`, `send_disabled`, `agent_note`, `action_subject`, `row_state`, `secondary_line`, `lead_text`, `active_builders`, `decision_rows`, `archived_meta`, `autorun_strip`, `segments`, `live_counts`, `without_times`, `derive_overview` (`Derived`), `DerivedKey` and `DerivedCache`, and `body_items` (`BodyItem`, `CellKind`, `Grid`).
- `src/model/id_map.rs` (new): `IdMap` and `IdSet`, a sorted `Vec` with binary search, with 1 test. It replaces the `HashMap`s of the timeline and the plan tree (see Anti-goal).
- `src/model/plan_tree.rs`: `build_plan_tree_and_timeline` (one timeline when TASK IDs are unique; `build_plan_tree` keeps its signature and calls it), `assign_waves` over task positions, the wave-1 group set without string copies, and `commit_shas` split by hand instead of by regex. Same results; every TASK-190 test unchanged.
- `src/model/timeline.rs`: one `by_id` index built as bars are pushed replaces the `ends` map and the two `by_id` maps; `eq_ignore_ascii_case("DONE")` replaces `to_uppercase`. Same results; every TASK-190 test unchanged.
- `src/model/mod.rs`: the new modules and exports.
- `src/page.rs`: replaced the overlay body. The page measures its width with a `canvas`, keeps the rail list (360 wide, `surface1`, right border, padding 16) and the main list beside it from 720 px, or one stacked list below it (header, progress, waiting, body, tasks, autorun log). It keeps TASK-190's workspace choice and placeholders, the Needs you send flow, the decision modal state, and a 60 s clock.
- `src/overview.rs` (new): draws each `BodyItem`.
- `src/cells.rs` (new): `lifecycle_stepper`, `manual_checks_cell`, `decisions_cell`, `archived_plans_cell`, `pace_line`, `error_box`, `skeleton`, `no_plan`, `warning_line`, `outline_button`.
- `src/run_cells.rs` (new): `autorun_strip`.
- `src/plan_tree_view.rs` (new): `plan_top` (title, run state, pace line, lead row), `wave_header`, `task_row` (spine 2 px at inset 8, gap 14, 16 px ticks, rows 7 by 8 with radius 8, the rotating `LoaderCircle`).
- `src/needs_you.rs` (new): `NeedsYouState`, `NeedsAction`, the agent note, the rows, the buttons, and the answer form.
- `src/decision_modal.rs` (new): `DecisionModal` and `decision_overlay`.
- `src/cell.rs`: `overview_cell` grows to fill a grid column (`flexGrow: 1`).
- `src/store.rs`, `src/task_list.rs`, `src/lib.rs`: removed the `dead_code` allows that this TASK made stale; added the module lines.
- `packages/desktop-gpui/Cargo.lock`: unchanged (no new dependency).

## Contract

- Produces: `plugin()` stays the only public item. No new contribution and no new request path; every read and send goes through `WatchtowerStore`.
- Consumes as specified: `WatchtowerStore::{board, overview, refresh, decision, builders, agent_target, send, initial_task}`, `Loaded<T>`, the TASK-190 model types and functions, `crate::cell::{overview_cell, show_more}`, the rail cards, `TaskList::select_from_outside`, `PluginContext::{show_message, open_agent}`, `MessageLevel::{Info, Error}`, and the TASK-195 icons.
- Initial task: the page passes `store.initial_task()` to the rail's task list, which selects it once the board loads; an unknown id selects none.
- Load rules: the page loads once per workspace (`ensure`); the header Refresh and the error Retry call `store.refresh`, which reloads the board and the overview. The decision modal loads only while open; Retry is the only reload.
- Lists: the main column and the stacked page are each one GPUI `list`; the plan top, each wave header, each task row, the plan bottom, and, in the narrow grid, each Needs you row are their own items. Cells side by side in a grid row are one item per row group.

## Verified

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-plugin-watchtower-board` -> exit 0; 133 lib tests and 1 integration test pass (`/tmp/wt194/test.txt`), 2026-10-17 00:46. New tests: `run_state_text_follows_the_electron_order`, `layout_for_keeps_the_electron_breakpoints`, `needs_you_rows_append_the_answer_by_clock_unless_the_run_stopped`, `pace_items_leave_out_the_average_and_a_stopped_finish`, `reject_step_confirms_then_sends_and_cancel_goes_back`, `action_buttons_are_disabled_without_an_idle_agent_or_while_sending`, `secondary_line_covers_each_row_state`, `the_lead_row_shows_the_plan_and_run_state_without_a_lead`, `decision_rows_sort_by_id_descending_and_only_adr_ids_open`, `archived_meta_and_strip_and_segments_match_electron`, `live_counts_match_the_dashboard_summary_of_the_live_tasks`, `tree_times_are_none_when_the_overview_has_no_run`, `body_items_follow_the_three_grids_and_the_load_states`, `the_derived_cache_rebuilds_only_when_its_key_changes`, `one_thousand_derives_of_a_40_task_plan_take_under_200_ms` (153 ms), and `a_later_insert_replaces_the_value_and_lookups_borrow`. The 117 TASK-190 tests pass with no test line changed (`git diff` of the test modules shows no change).
- `rg -n "bachuc-(composer|...|plugin-[a-z-]+)\b" packages/desktop-gpui/plugins/watchtower-board/Cargo.toml | rg -v plugin-host` -> UNSATISFIABLE, as in TASK-190: it matches line 2, the crate's own name. Replacement `... | rg -v 'plugin-host|^2:name = "bachuc-plugin-watchtower-board"$'` -> no match (exit 1).
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-plugin-watchtower-board` -> exit 0.
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> exit 0 (`/tmp/wt194/build-app.txt`). `bachuc-app` does not link this crate until TASK-191; its one warning (`next_theme` never used) is in `crates/app`.
- `git diff --stat -- hiep-plugins/plugins/watchtower-board` -> empty.
- `cargo.sh clippy -p bachuc-plugin-watchtower-board --tests` -> no warning in the new code; two warnings remain in TASK-190 code (`model/actions.rs:294`, `model/page.rs:66`), not changed here.
- UI check (overlay at 1,300 and 700 against Electron 6770) -> DEFERRED (UI checker).
- The view wiring (layout switch, list slices, spinner frames, Needs you sends with "Sent to <name>", the modal, Escape) needs a window: UNVERIFIED (unattended): code tests only. Live daemon down: no live check.
- GitNexus impact on `overview_cell` and `build_plan_tree`: `risk: UNKNOWN` (the crate is not in the index). `rg` found the only callers: the rail's `WaitingCard`, and tests plus `derive_overview`.

## Anti-goal

- Checkpoint 1, when the derived-data rules landed: tripped. `one_thousand_derives_of_a_40_task_plan_take_under_200_ms` took 643 ms; source: the first crate test run; 2026-10-17 00:31. Profile per 1,000 calls: timeline 147 ms, plan tree 318 ms (it built a second timeline), commit regex 20 ms. Fixes: one timeline per derive, sorted-vec `IdMap` instead of debug-build `HashMap`s (40 inserts and lookups: 18 ms with SipHash or FNV, 3 ms sorted), `assign_waves` over positions, `commit_shas` without regex, counts without copying the tasks, and `classes` in an `IdMap`.
- Checkpoint 2, at completion: 153 ms for the best of 3 runs of 1,000 derives, in the full crate suite; source `/tmp/wt194/test.txt`; 2026-10-17 00:46. The test takes the best of 3, because other tests and the other builder's cargo share the machine and only add time; one run alone measured 154 ms.
- Recompute rule: `DerivedCache` rebuilds only when `DerivedKey { revision, live, minute }` changes; `the_derived_cache_rebuilds_only_when_its_key_changes` shows 100 same-key updates build once. The page calls it from `sync` (store change, `Connected`, `AgentsChanged`, the 60 s clock), never from render.

## UI Checks

Checked 2026-10-17 by the UI checker with `open -g`, AX reads, AX presses, and window screenshots (no pointer, no foreground).

- UI check (Watchtower overlay at width 1,300 and 700 against Electron 6770) -> UNVERIFIED (unattended): needs pointer. AX press on the `Open Watchtower` pill changed nothing (`/tmp/wt-ui/gpui-watchtower-overlay.jpg`), so no overlay drew, and the window cannot be resized without the pointer. The reference lacks the plugin (`watchtower-board` is `failed` on 6770).

## Lessons

- Safe defaults taken (see questions): Escape closes the modal through the dialog's focus, but the shell's capture listener (`crates/app/src/shell.rs:1537`) closes the whole surface on Escape first, so Escape closes the page with the modal. The error Retry reloads the board too, because the store has no overview-only refresh. Reduced motion is not read; the spinner always turns. `DECISIONS.md` IDs sort by byte compare, which agrees with `localeCompare` on ADR IDs.
- In a debug build, `HashMap` work dominated the overview rules: a 40-entry map costs about 18 us per build-and-lookup pass. A sorted `Vec` with binary search costs 3 us. Use one when a debug timing test covers a hot pure function.
- `ListState::splice` over the whole range resets the scroll position; `changed_range` plus `remeasure()` keeps it while heights change.
- A backdrop that does not `occlude()` lets a click reach the list rows under it.

## UI Checks, round 2

Checked 2026-10-17 by the UI checker with `open -g`, AX reads, AX presses, and window screenshots (no pointer, no foreground). Electron 6770 shows `watchtower-board` as `failed`.

- UI check (Watchtower overlay at width 1,300 and 700 against Electron 6770) -> UNVERIFIED (unattended): needs pointer. AX press on `Open Watchtower` in the agent composer now opens the overlay at the window width of 1,200 pt, with the header, a left column (`0%`, `0 of 0 done`, `Tasks`) and a right column (`Watchtower`, `No plan yet`, and the cards `Needs you`, `Decisions`, `Archived plans`) (`/tmp/wt-ui2/watchtower-overlay.jpg`). The workspace `/tmp/wt-long-thread` has no plan, the window cannot be resized without the pointer, and the reference has no plugin. The header, plan tree, and stacked order are not compared.
