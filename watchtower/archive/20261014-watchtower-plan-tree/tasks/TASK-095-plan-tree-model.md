# TASK-095 Plan tree model

Group: A (writes only the model file and its test)
Class: code

## Brief

Goal: Turn the overview tasks and the timeline into waves of TASK rows that the tree view can draw without more logic.

Change: The page computes bars on an hour axis -> `buildPlanTree` returns waves, each with ordered rows and a short time text.

Design: see Shared Context in [watchtower/CONTEXT.md](watchtower/CONTEXT.md).

Boundaries:

- Pure function. No React, no agent list (TASK-096 adds builders).
- A wave is the dependency level: a TASK with no in-plan deps is wave 1; otherwise 1 + the highest wave of its in-plan deps. A dep that is not in the plan does not count.
- A cycle must not hang or throw. Put every TASK left in a cycle in one last wave and mark it `cyclic`.
- Inside a wave, keep Tracker order.
- Time text per row, built from [client/timeline.ts](hiep-plugins/plugins/watchtower-board/client/timeline.ts): done `4m`; running `12m / 9m` with an `over` flag when it passes the average; planned `~10:24` from the expected start; blocked or no data: no time.
- A wave is `done` when every row is done. The view collapses done waves.
- The wave header text: wave 1 with N groups says "N groups in parallel"; later waves say "starts when TASK-X and TASK-Y finish", naming the in-plan deps that are not done.
- Pace line: average task minutes, estimated finish clock, max parallel builders, and the slowest running TASK with its minutes over average. Take each from `timeline.stats` and `finishMinute`.

How:

- Write `tests/plan-tree.test.ts` first.
- Write `buildPlanTree({ tasks, run, questions }, started)` in `client/plan-tree.ts`, returning `{ waves, pace }`.

Files:

- [hiep-plugins/plugins/watchtower-board/client/plan-tree.ts](hiep-plugins/plugins/watchtower-board/client/plan-tree.ts) (new)
- [hiep-plugins/plugins/watchtower-board/tests/plan-tree.test.ts](hiep-plugins/plugins/watchtower-board/tests/plan-tree.test.ts) (new)

Expected result:

- The Fleet map plan (6 TASKs, deps 090 on 086..089, 091 on 090) gives three waves: 4, 1, and 1 rows.
- A dep outside the plan is ignored.
- A cycle ends in one last wave marked `cyclic`, with no throw.
- Done, running, planned, and blocked rows get the time text above.
- The pace line matches `timeline.stats` for the same input.

Anti-goal: The model stays fast for big plans. Tripwire: 200 TASKs with chained deps build in under 20 ms in the test; read at the end of the TASK.

## Verify

- `cd hiep-plugins/plugins/watchtower-board && npx tsx --test tests/plan-tree.test.ts` -> all cases pass.
- `cd hiep-plugins/plugins/watchtower-board && npm run typecheck` -> exit 0.
- `cd hiep-plugins/plugins/watchtower-board && npm run lint` -> exit 0.
- Anti-goal: the 200-TASK case asserts under 20 ms and passes.
