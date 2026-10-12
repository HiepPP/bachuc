# NEXT

## Current Active Plan

- Title: Turn step list follow-ups
- Slug: 20261013-turn-step-list-follow-ups
- Status: ACTIVE
- Updated: 2026-10-13

## Tracker

One row per TASK. Group ties together items that ship as one transaction.

| Order | TASK                                    | Group | Status | Spec                                                                                                                       | Deps | Context                                        | Notes                                    |
| ----- | --------------------------------------- | ----- | ------ | -------------------------------------------------------------------------------------------------------------------------- | ---- | ---------------------------------------------- | ---------------------------------------- |
| 1     | TASK-076 Note opens its step            | A     | TODO   | [watchtower/tasks/TASK-076-note-opens-its-step.md](watchtower/tasks/TASK-076-note-opens-its-step.md)                       | -    | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Model change; risky.                     |
| 2     | TASK-077 Edit row spacing outside steps | A     | TODO   | [watchtower/tasks/TASK-077-edit-row-spacing-outside-steps.md](watchtower/tasks/TASK-077-edit-row-spacing-outside-steps.md) | -    | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Shares the step list view with TASK-078. |
| 3     | TASK-078 Scheme label colors            | A     | TODO   | [watchtower/tasks/TASK-078-scheme-label-colors.md](watchtower/tasks/TASK-078-scheme-label-colors.md)                       | -    | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Design rule exception per ADR-0020.      |

TASK Status labels: TODO, IN PROGRESS, BLOCKED, DONE.
Plan-level Status header: ACTIVE while any row is open, DONE when all rows DONE, ARCHIVED after archive.

## Plan Verify

- `npm run typecheck` -> exit 0 after all TASKs.
- `cd packages/app && npx vitest run src/tool-calls/detail-level/turn/model.test.ts --bail=1`, then the same for `turn/view.test.tsx` and `projection.test.ts` -> pass.
- `cd hiep-plugins/plugins/edit-diffs && npm test` -> pass.
- Manual on live (needs a screen; reload the window first if Metro showed an error): the sample turn in the turn-step-check thread at the Summary level -> each note sits above the work it announced, and the label colors read well in the light and dark themes.
- Manual on live: the same thread at Full detail -> two edit rows in a row have a gap between them, as before PR #60.

## Handoff

- Next action: run `/watchtower implement` or `/watchtower autorun`, starting with TASK-076. All three owner choices are made; see [watchtower/CONTEXT.md](watchtower/CONTEXT.md).

## Archive

- [watchtower/archive/20260930-active-machine-switch](watchtower/archive/20260930-active-machine-switch)
- [watchtower/archive/20260930-plugin-host-apis](watchtower/archive/20260930-plugin-host-apis)
- [watchtower/archive/20261006-t3code-orchestration-port](watchtower/archive/20261006-t3code-orchestration-port)
- [watchtower/archive/20261007-bachuc-rebrand](watchtower/archive/20261007-bachuc-rebrand)
- [watchtower/archive/20261007-watchtower-process-overview](watchtower/archive/20261007-watchtower-process-overview)
- [watchtower/archive/20261012-repo-wide-ids-and-board-lanes](watchtower/archive/20261012-repo-wide-ids-and-board-lanes)
- [watchtower/archive/20261012-turn-step-list](watchtower/archive/20261012-turn-step-list)
