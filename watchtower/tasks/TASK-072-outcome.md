# TASK-072 Outcome

## Outcome

Status: DONE (auto: manual checks pending)

Run: autorun iteration 1, 2026-10-12. Built by an Opus xhigh subagent, reviewed by one `reviewer` subagent (1 blocker, 1 should-fix, 2 nits, 4 test gaps), fixed by a second Opus subagent. The run re-ran every Verify line after the fixes. The Plan Verify manual checks on live wait for TASK-073, because no view passes `isNonCompact` yet.

Changed:

- `packages/app/src/types/stream.ts`: optional `sourceToolCall?: ToolCallItem` on `PluginTimelineStreamItem`. This is the tool call marker. It holds the replaced call, so the host can tell edits from other replaced calls and read the file path without a plugin ID check.
- `packages/app/src/plugins/timeline/projection.ts`: sets the marker when a transform replaces a `tool_call` item. A transform that keeps the source item does not set it.
- `packages/app/src/tool-calls/detail-level/grouping.ts`: turn mode (`TurnRun`, `GroupedTurnHistory`, `turnWorkCall`, `findTurnEnds`, `prepareTurnHistory`, `groupLiveTurns`). The shared live-group merge moved into `mergeLiveGroups`; overview behavior is unchanged.
- `packages/app/src/tool-calls/detail-level/turn/model.ts` (new): `buildTurnGroup` and the step model types.
- `packages/app/src/tool-calls/detail-level/projection.ts`: `ToolCallGroupingOptions { isNonCompact? }`; the overview level with `isNonCompact` uses turn mode. `ToolCallHistoryOptions` adds `isTurnActive?` for `prepareToolCallHistory`. New `turnGroupsByHostId` lookup on the projection.
- `packages/app/src/agent-stream/presentation.ts`: optional `isNonCompact` input, passed into the projection and part of the history cache key. In turn mode only, `isTurnActive` is part of the key too.
- Tests: `turn/model.test.ts` (new, 12 cases), 16 turn cases appended to `detail-level/projection.test.ts`, 1 marker case appended to `plugins/timeline/projection.test.ts`.

Review fixes:

- BLOCKER, answer at a non-user boundary: `findTurnEnds` is a backward pass. A boundary ends the turn when no note or tool work follows it before the next user message, and that message came or the turn is idle. `prepareTurnHistory` runs it over the tail with the new `isTurnActive` input; `groupLiveTurns` runs it over the head. So `[user, t1, n1, t2, final, panel]` on an idle agent keeps `final` outside the run. The presentation cache key holds `isTurnActive` only when `isNonCompact` and the level is `overview`, so compact and `detailed` keep the same key and output.
- SHOULD-FIX, first step too short: `TurnRun.startedAt` is the user message before the run, or the boundary that split its turn. Without either, for example at the start of the loaded window, it falls back to the first item. `GroupedTurnHistory.pendingStartedAt` carries it into the live head. `buildTurnGroup` starts step 1 there. The presentation has no active turn start, so that source is not used.
- NIT, stale running call: the trailing flush in `groupLiveTurns` uses `endsTurn: !isTurnActive`; `isSealed` is unchanged. `[user, t1 running, final]` on an idle agent keeps `final` outside and the run unsealed.
- Tests: answer before an unmarked plugin row or a notification (idle and active); answer and plugin row in the live head; a later user message; todo list and compaction as split and end boundaries; the stale running call; the 5-step turn through the live head from active to idle; the history regroup on the turn state change, with the compact cache kept; a non-edit step between two Edit steps; the first-step start from the turn start, a split boundary, and the window start. Two model cases now expect the user message time as the first step start.

Impact (GitNexus, upstream): `PluginTimelineStreamItem` CRITICAL (406 dependents, member of `StreamItem`); the change is one optional field. `prepareToolCallHistory`, `projectToolCallDetailLevel`, `createStreamPresentation`, `transformSourceItem`, `groupLiveToolCalls` LOW.

Verified:

- `cd packages/app && npx vitest run src/tool-calls/detail-level/turn/model.test.ts --bail=1` -> 12 passed: 5 steps with the answer outside, labels, Paseo edit rows as edits, end times and durations, first step from the turn start, Edit between two Edit steps, running step, summary, hidden thoughts with joined note blocks, presentation integration, 5 steps through the live head, history regroup on the turn state change.
- `cd packages/app && npx vitest run src/tool-calls/detail-level/projection.test.ts --bail=1` -> 35 passed (19 existing, 16 turn cases).
- `cd packages/app && npx vitest run src/agent-stream/presentation.test.ts --bail=1` -> 23 passed.
- `cd packages/app && npx vitest run src/plugins/timeline/projection.test.ts --bail=1` -> 9 passed, including the marker case.
- `npm run typecheck` -> exit 0.
- `npm run lint -- packages/app/src/tool-calls packages/app/src/agent-stream/presentation.ts packages/app/src/plugins/timeline/projection.ts packages/app/src/types/stream.ts` -> exit 0, 0 warnings, 0 errors.
- `npm run format:files` and `npm run format:check:files` on the 9 changed files -> exit 0.
- Extra: `cd packages/app && npx vitest run src/agent-stream/model.test.ts --bail=1` -> 10 passed.
- Review fixes RED: before the fix, 4 model cases and 7 projection cases failed for the expected reason; the rest passed.

Anti-goal:

- Before: 0 diff lines; both files 42 passed, 2026-10-12.
- After the grouping change: 0 removed lines; all 42 existing tests passed; only the 7 new turn cases failed, before the projection wiring.
- After the review fixes: `git diff -U0 turn-step-list` shows one hunk, `@@ -482,0 +483,291 @@` in `projection.test.ts`, 0 removed lines, and no change in `presentation.test.ts`; 35 + 23 passed.
- Result: PASS. 0 existing assertions edited or removed; existing tests pass.

Lessons:

- TASK-073 entry points: `presentStream({ ..., isNonCompact: !isMobile })` in `agent-stream/view.tsx` turns the mode on. Without the flag, output is the same as before, so nothing shows until TASK-073 passes it.
- Turn groups are in `presentation.turnGroupsByHostId`, not `groupsByHostId`, because `view.tsx` types `groupsByHostId` as overview groups. `historyGroupUpdatesByHostId` carries turn updates too.
- The host row is a `tool_call` item: `{ ...run.latest, id: run.id }`. `run.id` is the first run item's id, which can be a note block or a thought. So `renderToolCallItem` gets it, and `getStreamItemMessageId` returns the host id, not the note's message id.
- Model types are exported from `tool-calls/detail-level/turn/model.ts`: `TurnStepGroup { mode: "turn", run, steps, summary, runningStep, isLoading }`, `TurnStep { id, label, note, items, edits, counts, startedAt, endedAt, durationMs, isRunning }`, `TurnSummary { stepCount, changedFiles }`, `TurnStepItem`, `TurnStepLabel`. `TurnRun { id, items, latest, startedAt, isSealed }` is in `grouping.ts`.
- `step.note` is an array of Markdown blocks of one message. `step.items` holds tool calls and replaced plugin rows in order. `step.edits` holds edit/write rows: plugin rows, or Paseo calls when edit diffs are off. `step.counts` is the existing `OverviewSummary`.
- Runs are created in `prepareTurnHistory` (retained tail) and `groupLiveTurns` (live head). History places the trailing run as if the turn ended. Live takes back the answer rows through `openTail`, or moves the host to the head when the head brings the first tool call.
- Rules chosen where the spec was silent: a turn ends at a user message, or at a todo list, notification, compaction, or unmarked plugin row when no note or tool work follows before the next user message and the turn is idle. Then its last message is the answer, outside the run. A boundary where work follows splits the run and keeps its last note inside. A run needs at least one tool call; a turn of notes alone keeps today's rows. A non-edit step between two Edit steps is labeled `edit`. The first step starts at `run.startedAt`: the user message, the split boundary, or the first item.
- History decides a boundary in the last turn from the tail alone. If the agent shows idle while the head still streams after such a boundary, the note before it shows as the answer until the turn state or the tail changes.
- `npx vitest run --dir <path>` does not limit the run to that path: the reviewer ran the whole app unit suite once by mistake. Pass the test file path instead.
- Perf for TASK-073: every streamed token builds a new turn group, with new step objects (O(turn items)). The step list must reuse step objects whose items did not change, so memoized step rows do not render again on each token.
