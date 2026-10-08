# TASK-004 Outcome

## Outcome

Status: DONE

Changed:

- Added [hiep-plugins/plugins/watchtower-board/client/timeline.ts](hiep-plugins/plugins/watchtower-board/client/timeline.ts) with `buildTimeline`, `barEnd`, `DEFAULT_TASK_MINUTES`, and the timeline types.
- Added [hiep-plugins/plugins/watchtower-board/tests/timeline.test.ts](hiep-plugins/plugins/watchtower-board/tests/timeline.test.ts) with 7 tests, built on the mid-run state of the design mock.
- Spec refinement 1: a running bar holds its lane, and planned bars wait, until the later of `now` and its expected end. A task already over the average cannot end in the past.
- Spec refinement 2: planned bars never start before `now`.
- Spec refinement 3: when every open task is running, `finishMinute` is the latest running expected end, not `null`. `null` stays for a plan with every task `DONE`.

Contract:

- `buildTimeline({ tasks, run, questions })` returns `bars`, `edges`, `laneCount`, `axis`, `stats`, `finishMinute`, and `deadlines`.
- Bar kinds: `done` (from the latest `DONE` log row of a `DONE` task), `running`, `planned`, and `blocked`.
- The average is the rounded mean of done bar lengths, or 20 minutes.
- Lanes pack by start time. A task takes its first dep's lane when that lane is free.
- Ticks fall on whole clock hours from `Started:`. With no start time, ticks read `+1h`, `+2h`, and so on.
- A deadline is the planned start of the earliest planned or blocked task that an `OPEN` question blocks.

Verified:

- `npx tsx --test tests/timeline.test.ts` -> 7 pass, 0 fail. The mock case gives 2 lanes, a finish at minute 194 (02:06), and a Q-008 deadline at minute 176 (01:48).
- `npm test` in the plugin folder -> 41 pass, 0 fail.
- `npm run typecheck` -> exit 0. `npm run lint` -> exit 0, 0 warnings.
- `rg -n "react-native|from \"react\"" client/timeline.ts` -> no match.

Anti-goal:

- Before lane packing: 0.1713 ms average over 50 runs, 200 tasks and 200 log rows; the timing test with lane packing switched off, 2026-10-07 12:49.
- After lane packing: 0.2230 ms; same test, 2026-10-07 12:49.
- Final: 0.2230 ms; same test after formatting, 2026-10-07 12:50.
- Result: PASS against 10 ms.

Lessons:

- The timeline works in minutes from `Started:`. A view converts a minute to a clock time with the run start; see `hourTicks` in [hiep-plugins/plugins/watchtower-board/client/timeline.ts](hiep-plugins/plugins/watchtower-board/client/timeline.ts).
- Use `barEnd(bar)` for where a bar stops taking room. A running bar's `end` is `now`, but its lane stays busy until its expected end.
