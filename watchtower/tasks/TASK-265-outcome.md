# TASK-265 outcome

Status: DONE (screen checks deferred to the UI checker)

Changed:

- `packages/desktop-gpui/crates/timeline/src/background.rs` (new): `place`, `PlacementRow`, `TurnSlot`, `Placement`, `finished_result`, `ResultTone`, `kind_label`, `process_label`, `processes_hash`, `hash_process`.
- `packages/desktop-gpui/crates/timeline/src/background_view.rs` (new): the card row, its running and finished rows, the log box, Stop, and the moved note.
- `packages/desktop-gpui/crates/timeline/src/lib.rs`: module, `RowKey::TurnCard`/`BottomCard`, `RowKind::TurnCard`/`BottomCard`, the processes read and hash in `sync`, the placement pre-pass, the card rows, the rule move, `update_clock`, `expanded_processes`, and the toggle and stop handlers.
- `packages/desktop-gpui/crates/timeline/tests/background.rs` (new): six tests.

Contract:

- No name for TASK-266. TASK-266 puts the running footer above this TASK's bottom card.
- Placement: a running process shows in the newest turn, else at the bottom; a finished process shows in the turn whose `toolCallId` matches a `tool_call` `callId`, else nowhere; a user message resets the newest turn; a running process shown in a later turn adds one to its start turn's `moved_count`.
- Result: `stopped` muted; else failed when `status == "failed"` or `exitCode != 0`; `exit N` or `failed`/`done` with the failed tone.
- A turn card sits after the turn's step rows and draws the finished-turn rule; the last step skips it when a card follows. The bottom card sits after the running footer and before permission rows.
- `sync` reads `Client::background_processes` and compares `processes_hash` in the early return.

Verified:

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-client -p bachuc-timeline`: PASS (group-level run for the group; client 135 tests + 5 ignored, timeline 136 tests, 0 failed).
- `tests/background.rs` running process from an older turn -> newest turn, start turn has `moved_count` 1: PASS.
- `tests/background.rs` finished process -> start turn; unknown `tool_call_id` -> not placed: PASS.
- `tests/background.rs` running process after a user message -> bottom list: PASS.
- `tests/background.rs` result text for stopped, exit 0, exit 2, failed+exit 0, failed, done: PASS.
- Read check of `sync`: the processes hash joins the early return; a process change rebuilds only the card row and the rule row (the last step's fingerprint includes `card_follows`; other step fingerprints are unchanged).
- Read check of `sync`: a reused turn (`old_turns` slice) gets its card pushed after the slice, and `card_changed` forces the step rows to rebuild when a card comes or goes.
- Read check: `update_clock` runs while `turn_active`, a loading header, or any running process; Stop draws only when `supports_background_stop`; `expanded_processes` drops ids of gone processes at each sync.
- UI checker: running card on live, Stop through AX, expand/collapse through AX -> DEFERRED (UI checker).
- Owner hand check: hover on rows and Stop -> UNVERIFIED (owner).
- Anti-goal: CPU with one idle agent and a background `sleep 300` at or below 2% over 60 s -> DEFERRED (UI checker). Baseline and final reads were not taken by the builder.

Anti-goal: not measured by the builder; the CPU read of the GPUI app is a screen check and is deferred.

Lessons:

- A process change must not rebuild turn or step rows. The card fingerprints cover only card content; `card_changed` exists only to move the finished-turn rule.
- `BackgroundProcess.kind` and `status` stay plain strings (TASK-264); the card maps `monitor` to `MONITOR` and every other kind to `BASH`.
- The running elapsed uses `format_elapsed` and the finished duration uses `format_duration`, both private to the timeline crate root and reachable from the child module.

## UI Checks

Run 2026-10-19 on live (6768) with the check bundle, launched with `BACHUC_NO_ACTIVATE=1` and driven by `cua_repl` AX actions only. Agent `7a749a8d` ran `sleep 300` with `run_in_background`. The app was quit and the agent archived.

- UI checker on live: running card -> PASS. The card is under the newest turn: header "Background 1 running"; row BASH, `sleep 300`, elapsed 20s -> 30s between two shots, Stop, and "no output yet".
- UI checker on live: Stop through AX -> PASS. The running card went away; a finished row "BASH sleep 300 stopped 1m 41s" shows in the start turn.
- UI checker on live: press the finished row through AX -> PASS. Its log opened with the tail line `[killed]` and the output file path; a second press closed it.
- Plan Verify (NEXT.md): background `sleep 300` shows the running card, Stop ends it, and the row moves to the finished card with "stopped" -> PASS, from the three checks above.
- Anti-goal: 1.4% over 60 s with an idle agent and a running `sleep 300` -> PASS against the 2% cap. Baseline comparison UNVERIFIED (unattended); no pre-change read exists.
