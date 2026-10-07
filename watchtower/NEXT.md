# NEXT

## Current Active Plan

- Title: Watchtower process overview screen
- Slug: 20261007-watchtower-process-overview
- Status: DONE
- Updated: 2026-10-07

## Tracker

One row per TASK. Group ties together items that write the same files.

| Order | TASK                                            | Group | Status | Spec                                                                                                   | Deps               | Context                                        | Notes                                                     |
| ----- | ----------------------------------------------- | ----- | ------ | ------------------------------------------------------------------------------------------------------ | ------------------ | ---------------------------------------------- | --------------------------------------------------------- |
| 1     | TASK-001 Overview plan data reader              | A     | DONE   | [watchtower/tasks/TASK-001-overview-plan-reader.md](watchtower/tasks/TASK-001-overview-plan-reader.md) | -                  | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | auto: no manual check. Also fixed the plugin lint script. |
| 2     | TASK-002 Overview run, questions, and history   | A     | DONE   | [watchtower/tasks/TASK-002-overview-run-history.md](watchtower/tasks/TASK-002-overview-run-history.md) | TASK-001           | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | auto: no manual check. Overview tasks drop `brief`.       |
| 3     | TASK-003 Lifecycle and needs-you model          | B     | DONE   | [watchtower/tasks/TASK-003-lifecycle-model.md](watchtower/tasks/TASK-003-lifecycle-model.md)           | TASK-002           | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | auto: no manual check.                                    |
| 4     | TASK-004 Branch timeline model                  | C     | DONE   | [watchtower/tasks/TASK-004-timeline-model.md](watchtower/tasks/TASK-004-timeline-model.md)             | TASK-002           | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | auto: no manual check.                                    |
| 5     | TASK-005 Branch timeline view                   | D     | DONE   | [watchtower/tasks/TASK-005-timeline-view.md](watchtower/tasks/TASK-005-timeline-view.md)               | TASK-004           | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | auto: manual checks pending (render on live).             |
| 6     | TASK-006 Overview cells                         | E     | DONE   | [watchtower/tasks/TASK-006-overview-cells.md](watchtower/tasks/TASK-006-overview-cells.md)             | TASK-003           | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | auto: manual checks pending (render on live).             |
| 7     | TASK-007 Overview screen                        | F     | DONE   | [watchtower/tasks/TASK-007-overview-screen.md](watchtower/tasks/TASK-007-overview-screen.md)           | TASK-005, TASK-006 | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | auto: manual checks pending (render on live).             |
| 8     | TASK-008 Full-screen surface shows the overview | G     | DONE   | [watchtower/tasks/TASK-008-surface-switch.md](watchtower/tasks/TASK-008-surface-switch.md)             | TASK-007           | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | auto: manual checks pending (live was down).              |
| 9     | TASK-009 Composer pill entry                    | H     | DONE   | [watchtower/tasks/TASK-009-composer-pill.md](watchtower/tasks/TASK-009-composer-pill.md)               | -                  | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | auto: manual checks pending (live was down).              |
| 10    | TASK-010 README for the overview and pill       | I     | DONE   | [watchtower/tasks/TASK-010-readme.md](watchtower/tasks/TASK-010-readme.md)                             | TASK-008, TASK-009 | -                                              | auto: no manual check.                                    |

TASK Status labels: TODO, IN PROGRESS, BLOCKED, DONE.
Plan-level Status header: ACTIVE while any row is open, DONE when all rows DONE, ARCHIVED after archive.

## Plan Verify

- In `hiep-plugins/plugins/watchtower-board`: `npm test` -> exit 0, and `tests/board.test.ts` still has 10 passing tests.
- In `hiep-plugins/plugins/watchtower-board`: `npm run typecheck` and `npm run lint` -> exit 0.
- From the repo root: `npm run format:check:files -- <changed plugin files>` -> exit 0.
- Live host check, when the live daemon on port 6768 is up: `npm run cli -- plugin reload watchtower-board` -> the plugin reloads without an error. When live is down, record it as `UNVERIFIED (autonomous run)`.
- Manual check, needs a human on live: click the Watchtower pill above an agent composer. The full-screen surface opens on that workspace and shows the lifecycle, the branch timeline, Needs you, the run log, manual checks, and decisions and history.

## Handoff

- Next action: All TASKs are DONE. The owner reviews draft PR #24 from `watchtower-overview` to `main`, after the manual checks below. PR #13 should merge first, because this branch starts from `t3code-port`.
- Plan Verify on 2026-10-07 13:16: `npm test` 49 pass, `tests/board.test.ts` 10 of 10, typecheck and lint exit 0, and the format check passes on all 22 changed plugin files. The live reload check is `UNVERIFIED (autonomous run)`, because the live daemon was stopped.
- Manual check pending: start live, reload `watchtower-board`, and look at an agent composer. The Watchtower pill shows the plan progress beside Tasks and Subagents (TASK-009).
- Manual check pending: on a wide window, click the pill. The page opens on that workspace and shows the lifecycle, the branch timeline, Needs you, the run log, manual checks, and decisions and history (TASK-005, TASK-006, TASK-007, TASK-008).
- Manual check pending: make the window narrower than 720 px. The Watchtower page shows the board, as before (TASK-008).
- Manual check pending: on New workspace, the pill reads `Watchtower` and opens the Watchtower page (TASK-009).

## Archive

- [watchtower/archive/20260930-active-machine-switch/](watchtower/archive/20260930-active-machine-switch/)
- [watchtower/archive/20260930-plugin-host-apis/](watchtower/archive/20260930-plugin-host-apis/)
- [watchtower/archive/20261006-t3code-orchestration-port/](watchtower/archive/20261006-t3code-orchestration-port/)
