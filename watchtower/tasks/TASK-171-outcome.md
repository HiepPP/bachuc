# TASK-171 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/composer/Cargo.toml`: adds `anyhow`, `bachuc-sidebar`, and `chrono` (std only, as `bachuc-timeline` uses it).
- `packages/desktop-gpui/Cargo.lock` (generated).
- `packages/desktop-gpui/crates/composer/src/lib.rs`: the new modules; exports `composer_dock`.
- `packages/desktop-gpui/crates/composer/src/composer.rs`: the root view. `Watched` grows `queue_held`, `status`, `workspace_id`, `active_turn`, `last_metrics`, `usage`, `tasks`, and `subagents`; it is read on `AgentsChanged`, `Connected`, and `TimelineChanged` for this agent, and a change alone redraws. The live tick, the send through `send_message` and `enqueue_message` with the send error, the queue actions, the density measure, and the strip assembly.
- New files, one per Electron component: `input.rs` (`MessageInput`, `round_button`), `dock.rs`, `pill_styles.rs`, `tracks.rs`, `agent_tracks.rs`, `task_list.rs`, `subagents_track.rs`, `turn_metrics_pill.rs`, `diff_stat_pill.rs`, `context_window_meter.rs`, `queue.rs`, `attachment_tray.rs`, `agent_controls/mod.rs`, `agent_controls/control.rs`, `agent_controls/layout.rs`, `agent_controls/features.rs`.
- `packages/desktop-gpui/crates/composer/src/features.rs`: deleted; moved to `agent_controls/features.rs`. The diff helpers and their tests moved to `diff_stat_pill.rs`.

## Contract

- `ComposerEvent` keeps `Sent { text, images }` and `ReplaceWithDraft`, and adds `OpenChanges { workspace_id }` (diff stat pill) and `OpenAgent { agent_id }` (Paseo subagent row). `Composer::new`, `agent_id`, `focus`, `attachment_count`, `Attachment`, `can_send`, and `mime_for_path` are unchanged.
- `bachuc_composer::composer_dock(centered: bool, content: AnyElement, composer: AnyElement) -> AnyElement`.
- Crate-internal: `tracks::TrackStrip::new(leading: Vec<AnyElement>, trailing: Vec<AnyElement>)`, `agent_tracks::has_agent_tracks(subagents, tasks, archiving, plugin_pills) -> bool`, `input::MessageInput` with `toolbar(..)` after the attach button and `primary(..)`, `agent_controls::layout::Density` (`Full`, `Condensed`, `Tight`) and `resolve_density`, and the `pill_styles` constants.
- Consumes the TASK-169 names as specified: `TurnMetrics`, `ActiveTurn`, `AgentUsage`, `TaskEntry`, `TaskStatus`, `SubagentRow`, `SubagentKind`, `OutgoingMessage`, and the `Client` accessors and queue requests.
- The live estimate runs only on the 1 s tick of an active turn: it scans back from the end of `Client::timeline` and stops at the active turn's first entry. The snapshot reads `agent_last_turn_metrics` only while no turn runs.
- Subagent marks use `bachuc_sidebar::model::bucket_for_agent` with 0 permissions, no attention reason, and the row's `requires_attention`.
- Amber 500 and red 500 are the named constants `AMBER_500` (`context_window_meter.rs`) and `SEND_ERROR_RED` (`composer.rs`).

## Verified

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-composer` -> exit 0; 71 passed at TASK-171 end (`/tmp/wt-c-171-test1.txt`), 91 passed after TASK-172 (`/tmp/wt-c-final-test.txt`). All 37 baseline tests are still present by name (`comm` of the two test lists is empty). Added tests cover: strip hidden with no source; `has_agent_tracks` for each source alone; the task label with `completed` and with status `Completed`; subagent segments in bucket order and description over title; each metric format and tone threshold; waiting "TTFT 3s" with U+2026; the live estimate before and after the 1 s window and with an earlier first output kept; the tick off without an active turn; a finished turn without metrics; meter hidden without context data, percent clamp, 91 -> destructive, 70 -> amber, both cost formats; the three densities with hysteresis both ways; queue move targets and first/last row moves; `Err("agent busy")` text and the kept input; the redraw test with tasks, subagents, metrics, usage, and a held queue.
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-composer` -> exit 0.
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> exit 0 (`/tmp/wt-c-171-app.txt`, again `/tmp/wt-c-final-app.txt`).
- UI check (strip height, divider, pill gaps, trailing order against Electron 6770) -> DEFERRED (UI checker).
- GitNexus `impact Composer` and `impact ComposerEvent` -> `UNKNOWN` (index 7 commits behind); `rg` found the callers `crates/app/src/panes.rs:79` (`Composer::new`), `crates/app/src/shell.rs:1095` (`==` on `ReplaceWithDraft`), and `examples/composer.rs`; all build.

## Anti-goal

- Checkpoint 1, before the split: `test -p bachuc-composer` 37 passed, including `an_unchanged_snapshot_causes_no_redraw`; source `/tmp/wt-c-base-test.txt`; 2026-10-16 19:13.
- Checkpoint 2, after the snapshot grew: 71 passed, including `an_unchanged_snapshot_causes_no_redraw` (full snapshot), `each_strip_field_counts_for_the_redraw`, and `the_tick_is_off_without_an_active_turn`; source `/tmp/wt-c-171-test1.txt`; 2026-10-16 22:52.
- Checkpoint 3, at completion: 91 passed with the same tripwire tests; source `/tmp/wt-c-final-test.txt`; 2026-10-16 23:03.

## Lessons

- In this gpui, `flex_grow` and `flex_shrink` take a value, and `Icon::with_size` needs `Sizable` in scope.
- Every test module in a file with `use gpui_kit::*` needs named imports; `use super::*` hits the gpui `test` macro recursion.
- Defaults taken (see questions): the diff pill label is the Electron string "Open Changes tab"; the running mark in the strip is a static ring, because the animated `RunningRing` is private to `bachuc-sidebar`; the bulk archive progress is tracked locally from the rows that leave the list, because `Client::archive_agent` returns no result, so no failure count shows; the meter keeps a track-only ring while the agent starts or runs without usage, as Electron does.
