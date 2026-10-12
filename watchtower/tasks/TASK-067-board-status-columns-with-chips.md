# TASK-067 Board status columns with TASK chips

Group: B3 (`hiep-plugins/plugins/board/client/**`, `hiep-plugins/plugins/board/shared/lanes.ts`, Board tests and README; shared with TASK-068)
Class: code

## Brief

Goal: Board shows Running and Just finished again, each card carries its TASK chip, and a project with an active plan gets one plan line that opens Watchtower.

Change: task-lane grids per project -> two status columns per host lane, beside the Needs you column.

Design: Claude Design project "Watchtower unified UI options", file `Board Watchtower Options.dc.html`, option 1a plus the Needs you column of option 2a.

Boundaries:

- Follows [ADR-0012](watchtower/decisions/ADR-0012-status-columns-with-task-chips-over-task-lanes.md), which supersedes ADR-0010.
- Keep the server side of TASK-063 as is: `taskId`, `planRoot`, `workspaceId`, and `plans`.
- Keep Needs you (TASK-064), `openWatchtower` (TASK-066), and the compact `stretch` fix.
- Restore the two-column layout from before TASK-064 (`RunColumn` in `git show HEAD:hiep-plugins/plugins/board/client/page.tsx`); do not rewrite cards.
- The plan line sits once per project with an active plan, above that project's first card group in Running, or in Just finished when nothing runs: `<project> · <plan title> · <done>/<total> · autorun running`. Pressing it opens Watchtower on the plan's workspace.
- A TASK chip sits in the card's model and effort row and in each subagent row that has a TASK. Pressing it opens Watchtower at that TASK. A chip whose TASK is not in the active plan still shows and opens Watchtower without a TASK.
- "N new" returns to the Just finished header.

How:

- Restore `RunColumn` and the two columns in `client/page.tsx`; mount Needs you on the left as now.
- Add the chip to `RunCard` and the subagent row, through a small `TaskChip` in a new `client/task-chip.tsx`.
- Add `client/plan-line.tsx` and place one per plan project in the columns.
- Delete `client/lanes.tsx` and the lane parts of `shared/lanes.ts` (`laneModel`, `treeTask`, `LANE_*`); keep `needsYouItems`.
- Update `tests/host-lanes.test.ts` for the two columns; replace `tests/lanes.test.ts` with chip and plan-line placement tests.
- Rewrite the README Task lanes section as "Watchtower on Board", and link ADR-0012.

Files:

- [hiep-plugins/plugins/board/client/page.tsx](hiep-plugins/plugins/board/client/page.tsx) (two columns, chips, plan lines)
- [hiep-plugins/plugins/board/client/task-chip.tsx](hiep-plugins/plugins/board/client/task-chip.tsx) (new)
- [hiep-plugins/plugins/board/client/plan-line.tsx](hiep-plugins/plugins/board/client/plan-line.tsx) (new)
- [hiep-plugins/plugins/board/client/lanes.tsx](hiep-plugins/plugins/board/client/lanes.tsx) (delete)
- [hiep-plugins/plugins/board/shared/lanes.ts](hiep-plugins/plugins/board/shared/lanes.ts) (keep Needs you only)
- [hiep-plugins/plugins/board/tests/host-lanes.test.ts](hiep-plugins/plugins/board/tests/host-lanes.test.ts) and [hiep-plugins/plugins/board/tests/lanes.test.ts](hiep-plugins/plugins/board/tests/lanes.test.ts)
- [hiep-plugins/plugins/board/README.md](hiep-plugins/plugins/board/README.md)

Expected result:

- A host lane shows Needs you, then Running and Just finished with their counts and "N new".
- A card of a labelled thread shows its TASK chip; pressing it opens Watchtower at that TASK on the plan's workspace.
- bachuc shows one plan line, not 8 rows; a project without a plan shows no plan line.
- Cases: a chip for a TASK of an archived plan opens Watchtower without a TASK; a plan with no workspace shows its line without a link.

Anti-goal: the bachuc block on a 1440 px Board with the 3 live test threads stays under 250 px tall, against about 560 px with lanes; drift gauge; read from a live screenshot before the change (`/tmp/lanes-1.png`), after restoring the columns, and at completion.

## Verify

- `cd hiep-plugins/plugins/board && npm run typecheck && npm run-script lint && npm test` -> all pass.
- `rg -n "laneModel|LANE_COLUMNS|TaskLanes" hiep-plugins/plugins/board` -> no match.
- Human check on live (Expo web on daemon 6768, light theme): two columns, Needs you, one bachuc plan line, chips on labelled cards; a chip opens Watchtower with that TASK selected.
- Measure the bachuc block height on a 1440 px screenshot -> under 250 px.
