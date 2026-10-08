# TASK-004 Branch timeline model

Group: C (standalone: `client/timeline.ts` and `tests/timeline.test.ts`)
Class: code

## Brief

Goal: Add a pure model that places every task on a time axis, packs the bars into lanes, links each task to its deps, and estimates the finish time.

Change: no timeline model -> `buildTimeline(overview)` with tests.

Design: the branch timeline cell of option 3f, and option 2d of round 2, both in the mock named in [watchtower/CONTEXT.md](watchtower/CONTEXT.md).

Boundaries:

- Follow the branch timeline rules in [watchtower/CONTEXT.md](watchtower/CONTEXT.md) `## Decisions`.
- Pure functions in a `.ts` file. No React and no `react-native` import.
- Use minute offsets from TASK-002 (`startMinute`, `endMinute`, `nowMinute`). Do not parse clock text again.

How:

- `buildTimeline(overview)` returns `bars`, `edges`, `laneCount`, `axis`, `stats`, `finishMinute`, and `deadlines`.
- Bars: one per task. `kind` is `done`, `running`, `planned`, or `blocked`. A task with no log row and status `DONE` gets no bar.
- Done bars use the latest log row of the task with result `DONE`.
- Running bars (`IN PROGRESS`) start at the end of the latest log row, or at minute 0 when the log is empty. They end at `nowMinute`. They also get `expectedEnd`: start plus the average duration.
- Planned and blocked bars run one at a time in Tracker order. Each starts at the latest of: the previous planned bar end, the expected end of every running bar, and the end of each dep.
- The average duration is the mean of done bar lengths, rounded to whole minutes, or 20 when there are none.
- Lanes: sort bars by start. Put a task in the lane of its first dep when that lane is free at its start. Otherwise use the lowest free lane.
- Edges: one per dep that has a bar, from the dep bar end to the task bar start, with both lanes.
- Axis: the range from 0 to the max of `nowMinute` and the last bar end. Ticks fall on whole clock hours; use `Started:` to place them.
- Stats: average duration, the longest done bar, the running bar furthest over the average, and the max number of bars that overlap in time.
- `finishMinute`: the end of the last planned or blocked bar, or `null` when every task is done.
- `deadlines`: for each blocking question, the planned start of the earliest task in its `blocks`. Answering by then keeps `finishMinute`.

Files:

- [hiep-plugins/plugins/watchtower-board/client/timeline.ts](hiep-plugins/plugins/watchtower-board/client/timeline.ts) (new)
- [hiep-plugins/plugins/watchtower-board/tests/timeline.test.ts](hiep-plugins/plugins/watchtower-board/tests/timeline.test.ts) (new)

Expected result:

- A done log with no overlap packs into lane 0, even when two tasks share a dep.
- Two running tasks, such as 006 and 008 in the mock, give two lanes that overlap in time. The edge from 008 to its planned child 009 changes lanes only when lane 1 is not free.
- A task with two deps starts after the later dep ends.
- A blocking question on a planned task gets a deadline equal to that task's planned start.
- No done bars gives an average of 20 minutes.
- Every task `DONE` gives `finishMinute: null`.

Anti-goal: the model stays cheap: `buildTimeline` on a 200-task overview with 200 log rows finishes within 10 ms on average over 50 runs; drift gauge; read in a test before the lane packing, after it, and at completion.

## Verify

- In the plugin folder: `npx tsx --test tests/timeline.test.ts` -> every case above passes.
- In the plugin folder: `npm run typecheck` and `npm run lint` -> exit 0.
- `rg -n "react-native|from \"react\"" hiep-plugins/plugins/watchtower-board/client/timeline.ts` -> no match.
- The timing test reports the average -> 10 ms or less.
