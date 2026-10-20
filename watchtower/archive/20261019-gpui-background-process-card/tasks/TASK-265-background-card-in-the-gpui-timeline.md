# TASK-265 Background card in the GPUI timeline

Group: B (the `bachuc-timeline` crate; shares `lib.rs` with TASK-266 and TASK-267; follows TASK-264 by start order only)
Class: risky

## Brief

Goal: The GPUI timeline shows the Electron Background card: running Bash and monitor processes with a live clock and Stop, finished rows with their result, and the moved note.

Change: no card in GPUI -> one new timeline row per card, placed by the Electron rule, drawn like [packages/app/src/agent-stream/background-process-card.tsx](packages/app/src/agent-stream/background-process-card.tsx).

Design: [packages/app/src/agent-stream/background-process-card.tsx](packages/app/src/agent-stream/background-process-card.tsx), [packages/app/src/agent-stream/background-process-placement.ts](packages/app/src/agent-stream/background-process-placement.ts), and its test [packages/app/src/agent-stream/background-process-placement.test.ts](packages/app/src/agent-stream/background-process-placement.test.ts).

Boundaries:

- Placement rule, ported as is: a running process shows under the step list of the newest turn. A finished process goes back to the turn whose tool call started it (`toolCallId` against `entry["item"]["callId"]` of a `tool_call` entry). A finished process whose turn is not loaded, or has no step list, is not shown. A user message resets the newest turn. A running process with no newest turn goes to the stream bottom. A running process shown in a later turn adds one to `movedCount` of its start turn.
- Row order, as Electron draws it: a turn card sits after the turn's last step row, and the rule under a finished turn moves under the card. The bottom card sits after the response footer and before the permission rows ([packages/app/src/agent-stream/view.tsx](packages/app/src/agent-stream/view.tsx), `renderPendingPermissionsNode`).
- Running card: a header with a running dot, "Background", and "N running". Each row has a dot, the kind label (`BASH` or `MONITOR`), the command (or the description when `command` is null) on one line, the live elapsed time, a Stop button, and the last `tail` line or "no output yet".
- Finished card: no header. Each row has a result dot, the kind label, the command, the result text, and the duration. Result, as `finishedResult` in the Electron card: `stopped` gives "stopped" with a muted dot and text. Else the process failed when `status` is `failed` or `exitCode` is not 0. Then "exit N" when `exitCode` is set, else "failed" or "done". The tone is danger when it failed, else success. So `failed` with exit 0 shows "exit 0" in danger.
- The moved note has a running dot and reads "Background processes started here: N · shown in the latest turn".
- A click on a row's main part toggles its log: every `tail` line (or "no output yet") and then `outputFile`. Keep the expanded set across syncs, keyed by process ID.
- Stop shows only when `Client::supports_background_stop` is true. It calls `Client::stop_background_process`.
- Planner call: no read-only gate on Stop. Electron also hides Stop in a read-only view, but the GPUI timeline has no read-only mode.
- Planner call: a `SharedRow::Turn` row counts as a step list. Electron places by the turn groups in `groupsByHostId`, and GPUI builds one turn row per group.
- Times: running elapsed uses `format_elapsed` ("6m 4s", "2m"; as `formatDuration` in [packages/app/src/utils/time.ts](packages/app/src/utils/time.ts), which `LiveElapsed` uses). Finished duration uses `format_duration` ("2m 04s", zero padded; as `formatTurnDuration`). Both exist in [packages/desktop-gpui/crates/timeline/src/lib.rs](packages/desktop-gpui/crates/timeline/src/lib.rs).
- Font: the card styles name `theme.fontFamily.mono`, but the Electron web build draws all text in the UI font unless a parent carries `data-pmono` (`packages/app/src/appearance/apply-root-font.web.ts:16`; code surfaces use `CODE_SURFACE_DATASET` in `packages/app/src/styles/code-surface.ts`). The card has no such mark, so draw the kind label, command, times, tail, log lines, and path in the UI font, as Electron shows them.
- Sizes and colors come from the Electron styles: 7 px dots, 58 px kind column, 12.5 px command, a 1 px `border` card with 8 px radius on `surface0`, and a 1 px `border` log box with 6 px radius on `surface1`. Use `dot_running`, `dot_success`, `dot_danger`, `foreground_muted`, `status_success`, and `status_danger` from [packages/desktop-gpui/crates/theme/src/tokens.rs](packages/desktop-gpui/crates/theme/src/tokens.rs), per [ADR-0043](watchtower/decisions/ADR-0043-electron-tokens-and-lucide-icons-over-gpui-kit-defaults.md).
- Follows [ADR-0039](watchtower/decisions/ADR-0039-incremental-timeline-sync-over-throttle-or-budget.md): a process change must not rebuild turn or step rows. Only card rows and the row whose rule moves change their fingerprint.
- Processes arrive in `agent_update`, not in the timeline, so `TimelineState::version` does not change with them. The early return in `sync` (`lib.rs:805`) must also compare a hash of the processes.
- Row reuse trap: `sync` copies an unchanged turn as the slice `self.rows[header..=header + group.steps.len()]` (`lib.rs:903`). Push the turn card after that copy or after `push_turn_rows`, never inside the slice, or a reused turn loses its card.
- The rule under a finished turn is drawn by the last step: its fingerprint at `lib.rs:1150` and its render at `lib.rs:2423` (`is_last_of_finished`) and `lib.rs:2610`. When a card follows, the last step must skip the rule and include "card follows" in its fingerprint, so the reused slice is rebuilt when a card comes or goes.
- Planner call: the timeline clock (`update_clock`, `lib.rs:1367`) also ticks while any process is running, even after the turn ends. A dev server can run for hours, so the anti-goal measures this tick.
- Keep each card one list row, per the render rules in [docs/gpui.md](docs/gpui.md). Never toggle `display` from hover. Give the Stop button and each row an `AccessibleAction::Click` handler.

How:

- Add `pub mod background;` with a pure `place(...)` that returns the processes and `moved_count` per turn key, and the bottom list. Port the 3 cases of the Electron placement test to [packages/desktop-gpui/crates/timeline/tests/background.rs](packages/desktop-gpui/crates/timeline/tests/background.rs), plus the result-text rule.
- In `sync`, read `Client::background_processes` and add the processes hash to the early-return check.
- Add a `RowKind` and `RowKey` variant for a card. Push a turn card after the turn's step rows, and the bottom card before the permission rows.
- Move the finished-turn rule from the last step row to the card row when a card follows.
- Render the card in `lib.rs` or a new `background_view.rs`, from the Electron styles above.
- Extend `update_clock` to run while any shown process is running. Today it checks only a loading `TurnHeader` (`lib.rs:1371`).
- Drop expanded process IDs that no longer exist at each sync.

Files:

- [packages/desktop-gpui/crates/timeline/src/background.rs](packages/desktop-gpui/crates/timeline/src/background.rs) (new: placement and result text, pure)
- [packages/desktop-gpui/crates/timeline/src/lib.rs](packages/desktop-gpui/crates/timeline/src/lib.rs) (module, row kinds, sync, rule move, clock, render or render call)
- [packages/desktop-gpui/crates/timeline/src/background_view.rs](packages/desktop-gpui/crates/timeline/src/background_view.rs) (new, optional: card render when `lib.rs` would grow too much)
- [packages/desktop-gpui/crates/timeline/tests/background.rs](packages/desktop-gpui/crates/timeline/tests/background.rs) (new tests)

Interfaces:

- Consumes: `bachuc_client::BackgroundProcess { id: String, kind: String, description: String, command: Option<String>, status: String, tool_call_id: Option<String>, started_at_ms: i64, ended_at_ms: Option<i64>, exit_code: Option<i64>, output_file: Option<String>, tail: Vec<String> }` from TASK-264
- Consumes: `Client::background_processes(&self, agent_id: &str) -> Vec<BackgroundProcess>` from TASK-264
- Consumes: `Client::supports_background_stop(&self) -> bool` from TASK-264
- Consumes: `Client::stop_background_process(&mut self, agent_id: &str, process_id: &str, cx: &mut Context<Client>)` from TASK-264
- Produces: no name for TASK-266. TASK-266 follows by the shared `lib.rs` and puts the running footer above this TASK's bottom card.

Expected result:

- A running background command shows the running card under the newest turn's step list, with a clock that ticks each second.
- Stop ends the process; the row then shows in the finished card of its start turn with "stopped".
- Cases:
  - Running process, newest turn has a step list -> card under that step list.
  - Running process started in an older turn -> card in the newest turn, and the moved note in the older turn.
  - Running process, newest turn has no step list -> card at the stream bottom, above permission cards.
  - Finished process, start turn loaded -> finished row in that turn.
  - Finished process, start turn not loaded -> nothing shown.
  - No processes, or a host without the feature -> no card; Stop hidden when the feature is off.

Anti-goal: GPUI app CPU with one idle agent that runs a background `sleep 300` stays at or below 2% over 60 s; drift gauge; read with `packages/desktop-gpui/measure.sh 60 gpui=<check bundle binary>` before changes (no card, same agent idle) and at completion.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-client -p bachuc-timeline` -> the new `background` tests and every old test pass.
- `tests/background.rs`: a running process from an older turn -> placed in the newest turn, and the older turn has `moved_count` 1.
- `tests/background.rs`: a finished process -> placed in its start turn; a finished process with an unknown `tool_call_id` -> not placed.
- `tests/background.rs`: a running process after a user message with no later step list -> in the bottom list.
- `tests/background.rs`: result text -> `stopped` gives "stopped" muted; `completed` with `exit_code` 0 gives "exit 0" success; `exit_code` 2 gives "exit 2" danger; `failed` with `exit_code` 0 gives "exit 0" danger; `failed` with no code gives "failed" danger; `completed` with no code gives "done" success.
- Read check of `sync` in `lib.rs`: a process change with no timeline change passes the early return and changes only card rows and the rule row.
- Read check of `sync` in `lib.rs`: a reused turn (the `old_turns` slice) keeps its card, and its last step stops drawing the rule when a card follows.
- Read check: `update_clock` stops when no turn loads and no shown process runs; Stop is not drawn when `supports_background_stop` is false; expanded IDs of gone processes are dropped.
- UI checker on live (screen check): ask a live agent to run `sleep 300` in the background. Raise the GPUI window and screenshot. Expect the running card under the newest turn, with `BASH`, the command, a clock that grows between two shots, and Stop.
- UI checker on live (screen check): press Stop through AX -> the running card goes away, and a finished row with "stopped" shows in the start turn.
- UI checker on live (screen check): press a finished row through AX -> its log opens with the tail lines or "no output yet". Press again -> it closes.
- Owner hand check: hover on rows and the Stop button looks like Electron. Mark it UNVERIFIED (owner) in the outcome until Hiep checks it.
- Anti-goal: `measure.sh 60` on the check bundle with the idle agent and a running `sleep 300` -> CPU at or below 2%; record the baseline and final values.
