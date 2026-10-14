# TASK-098 Outcome

## Status

DONE. The live check ran by hand on 2026-10-14 and found two text bugs, now fixed (see Live check). Running and waiting builder rows and the pace line stay UNVERIFIED.

## Changed

- `hiep-plugins/plugins/watchtower-board/client/overview.tsx`: a "Plan" cell with the pace line and the plan tree replaces the Branch timeline cell, Running now, and Up next. The page wires `useRunAgents` and `matchBuilders`.
- `hiep-plugins/plugins/watchtower-board/client/run-cells.tsx`: `RunningNowCell` and `UpNextCell` are removed. `clockAt` moved here unchanged from the deleted geometry file, because `AutorunStrip` and the overview still use it.
- Deleted with `git rm`: `client/timeline-view.tsx`, `client/timeline-geometry.ts`, `tests/timeline-geometry.test.ts`.
- `client/page.tsx` and `tests/overview.test.ts` did not change. The overview already has the host ID, the workspace ID, and navigation, and the test names no removed part.

## Contract

- Pace line (`PaceLine` in `overview.tsx`): four mono numbers, each with a muted label: `average task`, `estimated finish`, `builders at once`, and `slowest over average` (`TASK-089 +3m`). A number with no data is hidden. That is: the average needs a logged done TASK, the finish needs a start clock and open work, builders at once needs a value above 0, and slowest needs a running TASK over the average. A stopped run hides the finish estimate. The line shows only when RUN.md exists.
- With no autorun, the tree shows the Tracker waves, and the overview clears every row's time.
- `PlanTree` gets `builders.byTask`, `builders.lead`, `narrow`, `classes` (TASK ID to `taskClass`), `commits` (`commitShas(run.log)`), `planTitle` (plan title), and `runState`. `runState` is one of: `No autorun yet`, `Run stopped at HH:MM`, `Run finished`, `Autorun since HH:MM`, or `Autorun running`. The same text is the cell's aside.
- `onOpenAgent` is `navigation.openAgent({ agentId, serverId: host.id })`, the handler that Needs you already used. It is now one shared `openAgent` value.
- `onSelectTask` selects the TASK in the left rail through the path the opener already uses, `initialTaskId`. `TaskList` keeps its selection internal and reads `initialTaskId` only on mount. So a pick sets `{ id, count }`, and `TasksCard` remounts with key `id:count` and `initialTaskId=id`. The remount also opens the TASK's group when that group is collapsed. Picking the same TASK again reselects it.
- Layout: wide shows the tree, then Needs you beside Manual checks, Decisions, and Archived. Medium shows the tree, then two rows of two cells. Narrow stacks every cell.

## Verified

- `npm test` -> exit 0, 90 tests, 90 pass, 0 fail.
- `npm run typecheck` -> exit 0.
- `npm run lint` -> exit 2 from the shell's rtk ESLint wrapper (see the TASK-096 outcome). `rtk proxy npm run lint` -> exit 0, `Found 0 warnings and 0 errors`.
- `npm run format` -> exit 0. `git status` shows only my files (`client/overview.tsx`, `client/run-cells.tsx`, and the three deletions).
- `rg -n "BranchTimeline|TimelineStrip|RunningNowCell|UpNextCell|timeline-geometry" hiep-plugins/plugins/watchtower-board` -> no match. The README has no symbol name, only the prose "Running now" at `README.md:52`, which TASK-099 updates.
- GitNexus impact (index 27 commits behind): `RunningNowCell`, `UpNextCell`, `BranchTimeline`, `TimelineStrip`, `timeline-geometry.ts`, and `WatchtowerOverview` are LOW. `clockAt` is MEDIUM, and every caller is inside the plugin (overview, run-cells, timeline-view). The "Server" module hit is a graph community label, not a server file. No caller is outside the plugin.
- Live: `npm run cli -- plugin reload watchtower-board` -> `watchtower-board running`, no error. The visual check (the Watchtower page shows the tree and the pace line, with no hour axis) is UNVERIFIED (unattended, no browser session).

## Anti-goal

Tripwire: the four test files pass with the same or more cases.

| Checkpoint                | Before (2026-10-14T05:09:15Z) | After (2026-10-14T05:12:17Z) |
| ------------------------- | ----------------------------- | ---------------------------- |
| `tests/overview.test.ts`  | 19 pass                       | 19 pass                      |
| `tests/timeline.test.ts`  | 7 pass                        | 7 pass                       |
| `tests/lifecycle.test.ts` | 10 pass                       | 10 pass                      |
| `tests/actions.test.ts`   | 6 pass                        | 6 pass                       |

Source: `npx tsx --test tests/<file>.test.ts`, the `ℹ pass` line. `tests/timeline-geometry.test.ts` had 14 pass before. The spec removes it. Not crossed: final PASS.

## Live check

Run on 2026-10-14 from 05:25Z to 05:50Z. Live daemon 6768, `npm run dev:desktop` with CDP on port 9223, plugin reloaded with `npm run cli -- plugin reload watchtower-board`. Page: the Watchtower page (Cmd+G) of the `feat/watchtower-plan-tree` workspace, plan 20261014-watchtower-plan-tree (5 of 5 done). Test agents: two Claude Haiku agents in that workspace, a lead and a builder labelled `watchtower.task=TASK-098` with `paseo.parent-agent-id` set to the lead. Screenshots through Playwright over CDP.

- Plan cell -> PASS. It replaces the branch timeline; Running now and Up next are gone.
- Waves -> PASS after fix. Four waves (095 and 096, 097, 098, 099). Done waves start collapsed and open with Show.
- Lead row -> PASS. "Lead agent, implement", then "WT lead (test), idle, active 17s ago, no builder running", and Open thread. Mock-provider agents are skipped by design, so the first try with mock agents showed the Q-036 default row.
- Row press -> PASS. Pressing TASK-098 selected it in the rail: the Done group opened and its brief showed.
- Narrow layout (500 px) -> PASS. The stepper and cells stack, and the tree has no spine.
- Pace line -> UNVERIFIED. It shows only with `watchtower/RUN.md`, and `implement` writes no RUN.md, so this plan shows "No autorun yet".
- Running builder row and amber waiting row -> UNVERIFIED. Every TASK of this plan is done, and the test agents were idle.
- Bug 1, fixed: a later done wave said "ready to start". Now a wave whose rows are all done says "done". Test: "a finished wave says done instead of a start condition".
- Bug 2, fixed: a row showed the ID twice ("TASK-095 TASK-095 Plan tree model"), because the Tracker title starts with the ID. The model now uses `taskTitle` from `client/dashboard.ts`, like the task list. Test: "a row title drops the leading TASK ID, as the task list does".
- After the fixes: `npx tsx --test tests/plan-tree.test.ts` 17 pass; `npm test` 92 pass, 0 fail; `npm run typecheck` exit 0; `rtk proxy npm run lint` 0 warnings, 0 errors. The fixes are in `client/plan-tree.ts` and `tests/plan-tree.test.ts`, not committed.
- Console: no error from the plugin. Seen and not from this change: a refused `ws://localhost:6770/ws` (a stale host entry) and React unknown-prop warnings.

## Lessons

- `paseo run` without `--workspace` creates a new workspace for each run, so test agents land outside the page's workspace. Pass `--workspace <id>`.
- `clockAt` lived in the file that the spec deletes, and kept files still used it. Look for live helpers before you delete a module. Here it moved to `run-cells.tsx`, which is in Files.
- The task list has no outside select handler. A keyed remount through `initialTaskId` gives a rail selection without a change outside Files. A remount also resets the rail's collapsed groups. A controlled `selectedTaskId` on `TaskList` and `TasksCard` would avoid that, but it changes files outside this spec.
- In the stacked layout (below 720 px), the task list sits below the tree, so a pick selects a TASK that is off screen.
- `rtk proxy npm run lint` runs the real lint script past the rtk ESLint wrapper.
