# NEXT

## Current Active Plan

- Title: Plan tree on the Watchtower page
- Slug: 20261014-watchtower-plan-tree
- Status: ARCHIVED
- Updated: 2026-10-14

## Tracker

One row per TASK. Group ties together items that write the same files.

| Order | TASK                                              | Group | Status | Spec                                                                                                                       | Deps                         | Context                                        | Notes                                                                              |
| ----- | ------------------------------------------------- | ----- | ------ | -------------------------------------------------------------------------------------------------------------------------- | ---------------------------- | ---------------------------------------------- | ---------------------------------------------------------------------------------- |
| 1     | TASK-108 Plan tree model                          | A     | DONE   | [watchtower/tasks/TASK-108-plan-tree-model.md](watchtower/tasks/TASK-108-plan-tree-model.md)                               | -                            | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Pure function. Waves from depIds.                                                  |
| 2     | TASK-109 Builder matching                         | B     | DONE   | [watchtower/tasks/TASK-109-builder-matching.md](watchtower/tasks/TASK-109-builder-matching.md)                             | -                            | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Agents to TASKs by label, lead by parent.                                          |
| 3     | TASK-110 Plan tree view                           | C     | DONE   | [watchtower/tasks/TASK-110-plan-tree-view.md](watchtower/tasks/TASK-110-plan-tree-view.md)                                 | TASK-108, TASK-109           | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | New component per mock D v2.                                                       |
| 4     | TASK-111 Replace the branch timeline              | D     | DONE   | [watchtower/tasks/TASK-111-replace-branch-timeline.md](watchtower/tasks/TASK-111-replace-branch-timeline.md)               | TASK-110                     | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Wires the tree, deletes the Gantt and two cells. implement: manual checks pending. |
| 5     | TASK-112 README and ADR                           | E     | DONE   | [watchtower/tasks/TASK-112-readme-and-adr.md](watchtower/tasks/TASK-112-readme-and-adr.md)                                 | TASK-111                     | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Docs only.                                                                         |
| 6     | TASK-113 Run timing from the git log              | F     | DONE   | [watchtower/tasks/TASK-113-run-from-git-log.md](watchtower/tasks/TASK-113-run-from-git-log.md)                             | -                            | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Pace and row times without RUN.md.                                                 |
| 7     | TASK-114 Polish the page toward mock D v2         | G     | DONE   | [watchtower/tasks/TASK-114-polish-to-d-v2.md](watchtower/tasks/TASK-114-polish-to-d-v2.md)                                 | -                            | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Segmented progress, link actions, commit line.                                     |
| 8     | TASK-115 Select a TASK without resetting the rail | G     | DONE   | [watchtower/tasks/TASK-115-keep-rail-state-on-select.md](watchtower/tasks/TASK-115-keep-rail-state-on-select.md)           | TASK-114                     | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Q-045. Same file as TASK-114.                                                      |
| 9     | TASK-116 Live check of running and waiting rows   | H     | DONE   | [watchtower/tasks/TASK-116-live-check-running-and-waiting.md](watchtower/tasks/TASK-116-live-check-running-and-waiting.md) | TASK-113, TASK-114, TASK-115 | -                                              | Lead runs it on live.                                                              |

TASK Status labels: TODO, IN PROGRESS, BLOCKED, DONE.
Plan-level Status header: ACTIVE while any row is open, DONE when all rows DONE, ARCHIVED after archive.

## Plan Verify

- `cd hiep-plugins/plugins/watchtower-board && npm run typecheck` -> exit 0.
- `cd hiep-plugins/plugins/watchtower-board && npm run lint` -> exit 0.
- `cd hiep-plugins/plugins/watchtower-board && npm test` -> every test file passes.
- `rg -n "BranchTimeline|TimelineStrip|RunningNowCell|UpNextCell" hiep-plugins/plugins/watchtower-board/client` -> no match.
- Needs a human or a browser session: on live, with the plugin reloaded (`npm run cli -- plugin reload watchtower-board`), open the Watchtower page of a workspace with an active plan. The tree shows waves, TASK rows, and the lead; a builder that waits for permission shows an amber row with Open thread.

## Handoff

- Next action: review `feat/watchtower-plan-tree`, merge it, and run `archive`. Every TASK is done; Q-048 was fixed in 985b868a8 and the TASK-116 re-run passed on live.
- Plan Verify, 2026-10-14: `npm test` 90 pass, `npm run typecheck` exit 0, oxlint 0 warnings and 0 errors, removed names have no match. `npm run lint` exits 2 only through the shell's rtk ESLint wrapper; `rtk proxy npm run lint` exits 0.

## Archive

- [watchtower/archive/20261013-fleet-map/](watchtower/archive/20261013-fleet-map/)
- [watchtower/archive/20261013-turn-step-list-follow-ups/](watchtower/archive/20261013-turn-step-list-follow-ups/)
- [watchtower/archive/20261013-composer-pill-wheel-select/](watchtower/archive/20261013-composer-pill-wheel-select/)
- Archived: 2026-10-14 -> watchtower/archive/20261014-watchtower-plan-tree/
