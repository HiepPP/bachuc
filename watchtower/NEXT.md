# NEXT

## Current Active Plan

- Title: Plan tree on the Watchtower page
- Slug: 20261014-watchtower-plan-tree
- Status: DONE
- Updated: 2026-10-14

## Tracker

One row per TASK. Group ties together items that write the same files.

| Order | TASK                                 | Group | Status | Spec                                                                                                         | Deps               | Context                                        | Notes                                                                              |
| ----- | ------------------------------------ | ----- | ------ | ------------------------------------------------------------------------------------------------------------ | ------------------ | ---------------------------------------------- | ---------------------------------------------------------------------------------- |
| 1     | TASK-095 Plan tree model             | A     | DONE   | [watchtower/tasks/TASK-095-plan-tree-model.md](watchtower/tasks/TASK-095-plan-tree-model.md)                 | -                  | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Pure function. Waves from depIds.                                                  |
| 2     | TASK-096 Builder matching            | B     | DONE   | [watchtower/tasks/TASK-096-builder-matching.md](watchtower/tasks/TASK-096-builder-matching.md)               | -                  | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Agents to TASKs by label, lead by parent.                                          |
| 3     | TASK-097 Plan tree view              | C     | DONE   | [watchtower/tasks/TASK-097-plan-tree-view.md](watchtower/tasks/TASK-097-plan-tree-view.md)                   | TASK-095, TASK-096 | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | New component per mock D v2.                                                       |
| 4     | TASK-098 Replace the branch timeline | D     | DONE   | [watchtower/tasks/TASK-098-replace-branch-timeline.md](watchtower/tasks/TASK-098-replace-branch-timeline.md) | TASK-097           | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Wires the tree, deletes the Gantt and two cells. implement: manual checks pending. |
| 5     | TASK-099 README and ADR              | E     | DONE   | [watchtower/tasks/TASK-099-readme-and-adr.md](watchtower/tasks/TASK-099-readme-and-adr.md)                   | TASK-098           | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Docs only.                                                                         |

TASK Status labels: TODO, IN PROGRESS, BLOCKED, DONE.
Plan-level Status header: ACTIVE while any row is open, DONE when all rows DONE, ARCHIVED after archive.

## Plan Verify

- `cd hiep-plugins/plugins/watchtower-board && npm run typecheck` -> exit 0.
- `cd hiep-plugins/plugins/watchtower-board && npm run lint` -> exit 0.
- `cd hiep-plugins/plugins/watchtower-board && npm test` -> every test file passes.
- `rg -n "BranchTimeline|TimelineStrip|RunningNowCell|UpNextCell" hiep-plugins/plugins/watchtower-board/client` -> no match.
- Needs a human or a browser session: on live, with the plugin reloaded (`npm run cli -- plugin reload watchtower-board`), open the Watchtower page of a workspace with an active plan. The tree shows waves, TASK rows, and the lead; a builder that waits for permission shows an amber row with Open thread.

## Handoff

- Next action: on live, open the Watchtower page of a workspace with an active plan and check the tree (Plan Verify, last line). The plugin was reloaded on live on 2026-10-14. Then review `feat/watchtower-plan-tree`, merge it, and run `archive`.
- Plan Verify, 2026-10-14: `npm test` 90 pass, `npm run typecheck` exit 0, oxlint 0 warnings and 0 errors, removed names have no match. `npm run lint` exits 2 only through the shell's rtk ESLint wrapper; `rtk proxy npm run lint` exits 0.

## Archive

- [watchtower/archive/20261013-fleet-map/](watchtower/archive/20261013-fleet-map/)
- [watchtower/archive/20261013-turn-step-list-follow-ups/](watchtower/archive/20261013-turn-step-list-follow-ups/)
- [watchtower/archive/20261013-composer-pill-wheel-select/](watchtower/archive/20261013-composer-pill-wheel-select/)
