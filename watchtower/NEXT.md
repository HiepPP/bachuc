# NEXT

## Current Active Plan

- Title: Turn step list for agent responses
- Slug: 20261012-turn-step-list
- Status: ACTIVE
- Updated: 2026-10-12

## Tracker

One row per TASK. Group ties together items that ship as one transaction.

| Order | TASK                                   | Group | Status | Spec                                                                                                                   | Deps     | Context                                        | Notes                                           |
| ----- | -------------------------------------- | ----- | ------ | ---------------------------------------------------------------------------------------------------------------------- | -------- | ---------------------------------------------- | ----------------------------------------------- |
| 1     | TASK-072 Turn grouping and step model  | A     | DONE   | [watchtower/tasks/TASK-072-turn-grouping-and-step-model.md](watchtower/tasks/TASK-072-turn-grouping-and-step-model.md) | -        | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | auto: manual checks pending.                    |
| 2     | TASK-073 Turn step list view           | B     | TODO   | [watchtower/tasks/TASK-073-turn-step-list-view.md](watchtower/tasks/TASK-073-turn-step-list-view.md)                   | TASK-072 | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Draws the final design; Q-025 default applies.  |
| 3     | TASK-074 Edit diffs row inside a step  | C     | TODO   | [watchtower/tasks/TASK-074-edit-diffs-step-row.md](watchtower/tasks/TASK-074-edit-diffs-step-row.md)                   | -        | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Plugin only; can run in parallel with TASK-072. |
| 4     | TASK-075 Overview as the default level | D     | TODO   | [watchtower/tasks/TASK-075-overview-as-default-level.md](watchtower/tasks/TASK-075-overview-as-default-level.md)       | TASK-073 | -                                              | Q-027 default; ship after the view exists.      |

TASK Status labels: TODO, IN PROGRESS, BLOCKED, DONE.
Plan-level Status header: ACTIVE while any row is open, DONE when all rows DONE, ARCHIVED after archive.

## Plan Verify

- `npm run typecheck` -> exit 0 after all TASKs.
- Manual on live (needs a screen): set Settings to the Overview level, run an agent turn with notes, commands, and an edit on desktop -> one step list per turn matches the final design, and the answer sits below it.
- Manual on live: the same turn at the `detailed` level and at the compact width -> today's views, unchanged.

## Handoff

- Next action: the autorun builds TASK-073 next, then TASK-074 and TASK-075. Q-025, Q-026, and Q-027 are answered in [watchtower/QUESTIONS.md](watchtower/QUESTIONS.md).
- TASK-073 starts from the Lessons in [watchtower/tasks/TASK-072-outcome.md](watchtower/tasks/TASK-072-outcome.md): pass `isNonCompact` in `view.tsx`, read `turnGroupsByHostId`, and reuse unchanged step objects on each streamed token.

## Archive

- [watchtower/archive/20260930-active-machine-switch](watchtower/archive/20260930-active-machine-switch)
- [watchtower/archive/20260930-plugin-host-apis](watchtower/archive/20260930-plugin-host-apis)
- [watchtower/archive/20261006-t3code-orchestration-port](watchtower/archive/20261006-t3code-orchestration-port)
- [watchtower/archive/20261007-bachuc-rebrand](watchtower/archive/20261007-bachuc-rebrand)
- [watchtower/archive/20261007-watchtower-process-overview](watchtower/archive/20261007-watchtower-process-overview)
- [watchtower/archive/20261012-repo-wide-ids-and-board-lanes](watchtower/archive/20261012-repo-wide-ids-and-board-lanes)
