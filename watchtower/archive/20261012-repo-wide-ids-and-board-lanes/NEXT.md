# NEXT

## Current Active Plan

- Title: Repo-wide Watchtower IDs and Board task lanes
- Slug: 20261012-repo-wide-ids-and-board-lanes
- Status: ARCHIVED
- Updated: 2026-10-12

## Tracker

One row per TASK. Group ties together items that ship as one transaction.

| Order | TASK                                                                   | Group | Status | Spec                                                                                                                   | Deps               | Context                                        | Notes                                               |
| ----- | ---------------------------------------------------------------------- | ----- | ------ | ---------------------------------------------------------------------------------------------------------------------- | ------------------ | ---------------------------------------------- | --------------------------------------------------- |
| 1     | TASK-059 Repo-wide ID rule in the skill                                | S     | DONE   | [watchtower/tasks/TASK-059-repo-wide-id-rule.md](watchtower/tasks/TASK-059-repo-wide-id-rule.md)                       | -                  | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Skill text in the ~/.claude repo; not committed.    |
| 2     | TASK-060 ID migration script                                           | M     | DONE   | [watchtower/tasks/TASK-060-id-migration-script.md](watchtower/tasks/TASK-060-id-migration-script.md)                   | -                  | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | 4 tests pass.                                       |
| 3     | TASK-061 Migrate this repo's archive                                   | R     | DONE   | [watchtower/tasks/TASK-061-migrate-bachuc-archive.md](watchtower/tasks/TASK-061-migrate-bachuc-archive.md)             | TASK-059, TASK-060 | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Applied after owner review; ADR-0003 fixed by hand. |
| 4     | TASK-062 Surface params in openSurface                                 | H     | DONE   | [watchtower/tasks/TASK-062-open-surface-params.md](watchtower/tasks/TASK-062-open-surface-params.md)                   | -                  | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Q-023. Release needs a new build.                   |
| 5     | TASK-063 Watchtower Needs you: full text, target link, no mock targets | W2    | DONE   | [watchtower/tasks/TASK-063-watchtower-needs-you-clarity.md](watchtower/tasks/TASK-063-watchtower-needs-you-clarity.md) | -                  | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | ADR-0013. watchtower-board only.                    |
| 6     | TASK-064 Board Timeline tab                                            | B4    | DONE   | [watchtower/tasks/TASK-064-board-timeline-tab.md](watchtower/tasks/TASK-064-board-timeline-tab.md)                     | -                  | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Live checked. Q-024 defaulted.                      |

TASK Status labels: TODO, IN PROGRESS, BLOCKED, DONE.

## Plan Verify

- `node ~/.claude/skills/watchtower/scripts/migrate-ids.mjs --repo .` -> prints "nothing to migrate" after TASK-061.
- `npm run typecheck` -> exit 0.
- In `hiep-plugins/plugins/board`: `npm run typecheck && npm run lint && npm test` -> all pass.
- In `hiep-plugins/plugins/watchtower-board`: `npm run typecheck && npm test` -> all pass except the pre-existing proposed-ADR assertion in `tests/overview.test.ts`.

## Handoff

- Next action: archive the plan. Every TASK is DONE.
- Dropped on 2026-10-12: the Board Watchtower work (plan reading, task lanes, keyboard and search, plan line, Board Needs you). Its code was never committed; the owner chose not to rebuild it. The TASKs were removed and the rest renumbered: old TASK-071 is TASK-063, old TASK-069 is TASK-064.
- Board keeps the TASK chip from the `watchtower.task` label; the chip opens Watchtower on the thread's workspace.
- Release: reload Board and watchtower-board on release only after a release build that includes TASK-062; older release hosts ignore the workspace param.
- Pre-existing: `hiep-plugins/plugins/watchtower-board/tests/overview.test.ts` expects a `proposed` ADR that no longer exists since commit 5357c1c.

## Archive

- Archived: 2026-10-12 -> watchtower/archive/20261012-repo-wide-ids-and-board-lanes/
