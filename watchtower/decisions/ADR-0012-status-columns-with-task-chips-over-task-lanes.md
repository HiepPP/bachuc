# ADR-0012 Status columns with TASK chips over task lanes on Board

Status: superseded by ADR-0014
Date: 2026-10-12
Supersedes: ADR-0010
Scope: `hiep-plugins/plugins/board/**`

## Context

- ADR-0010 put each project in a grid of TASK rows across Todo, Running, Finished, and Verified.
- The live check on 2026-10-12 (outcome of the task-lanes TASK, since dropped) showed bachuc as 8 rows where 7 held only the hint "Done" and no card; the grid took about 560 px. A project without a plan was one Ad-hoc row with two shaded, unused columns, and its 7 cards stacked in one cell.
- The Watchtower page already shows TASK status, deps, blockers, progress, and a timeline per plan, one click away from Board.
- Board's own job is triage across every project and host: what needs the owner, what runs, what just finished, what to remove.

## Decision

- Board goes back to the two status columns, Running and Just finished, per host lane.
- It keeps what Watchtower does not have: a TASK chip on each card and subagent row, the Needs you column, search, keys, and one plan line per project that opens Watchtower.
- The owner chose this on 2026-10-12, after reviewing the live lanes.

## Consequences

- `laneModel`, `client/lanes.tsx`, and the TASK rows go away; the server's `taskId` and `plans` stay.
- Search and keys walk the two columns instead of the grids.
- Plan depth lives only on the Watchtower page.

## Options Considered

- Task lanes, ADR-0010: rejected. Measured on live, most cells were empty and the grid repeated the Watchtower page.
- Task lanes without empty DONE rows and shaded columns: rejected. It still repeats Watchtower's task list and splits cards of one project into many cells.
- A Gantt as Board's main view: rejected. Bars drop star, Remove, model, and subagents, and 50 threads over days give a sparse axis; a Timeline view beside Recaps covers looking back.

## Revisit If

- A plan regularly runs 3 or more TASKs at once in one project, so the Watchtower page and Board together no longer show what each thread is doing.
- The Watchtower page is removed or stops showing TASK status.
