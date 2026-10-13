# NEXT

## Current Active Plan

- Title: Mouse wheel selects a value on composer pills
- Slug: 20261013-composer-pill-wheel-select
- Status: ACTIVE
- Updated: 2026-10-13

## Tracker

| Order | TASK                                | Group | Status | Spec                                                                                                               | Deps               | Context                                        | Notes                                                                        |
| ----- | ----------------------------------- | ----- | ------ | ------------------------------------------------------------------------------------------------------------------ | ------------------ | ---------------------------------------------- | ---------------------------------------------------------------------------- |
| 1     | TASK-079 Wheel step helper and hook | A     | DONE   | [watchtower/tasks/TASK-079-wheel-step-helper-and-hook.md](watchtower/tasks/TASK-079-wheel-step-helper-and-hook.md) | -                  | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | auto: manual checks pending. New files only.                                 |
| 2     | TASK-080 Wire the toolbar pills     | B     | DONE   | [watchtower/tasks/TASK-080-wire-toolbar-pills.md](watchtower/tasks/TASK-080-wire-toolbar-pills.md)                 | TASK-079           | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | auto: manual checks pending. Mode, thinking, provider, select feature pills. |
| 3     | TASK-081 Wire the model pill        | C     | DONE   | [watchtower/tasks/TASK-081-wire-model-pill.md](watchtower/tasks/TASK-081-wire-model-pill.md)                       | TASK-079           | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | auto: manual checks pending. Composer toolbar branch only, per Q-033.        |
| 4     | TASK-082 Document wheel select      | D     | TODO   | [watchtower/tasks/TASK-082-document-wheel-select.md](watchtower/tasks/TASK-082-document-wheel-select.md)           | TASK-080, TASK-081 | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Docs only. Runs after the shape is final.                                    |

TASK Status labels: TODO, IN PROGRESS, BLOCKED, DONE.
Plan-level Status header: ACTIVE while any row is open, DONE when all rows DONE, ARCHIVED after archive.

## Plan Verify

- `npm run typecheck` -> passes.
- `npm run lint -- packages/app/src/composer/agent-controls packages/app/src/components/combined-model-selector.tsx` -> passes.
- `npm run format:check:files -- <every file this plan changed>` -> passes.
- `npx vitest run packages/app/src/composer/agent-controls --bail=1 > /tmp/wheel-select-tests.txt 2>&1` -> all files pass; read the output file after the run.
- Live host check, needs a human and a mouse: start live, hover each composer pill, and turn the wheel one tick. The value steps by one option, the list stays closed, the step stops at the first and last option, and the agent stream behind the composer does not scroll.

## Handoff

- Next action: implement TASK-082. TASK-079 to TASK-081 are merged. The model pill wheel runs only on the composer `toolbar` branch (Q-033) and ignores an off-list model. Q-030 to Q-033 are DEFAULTED.

## Archive

- None.
