# NEXT

## Current Active Plan

- Title: Repo-wide Watchtower IDs and Board task lanes
- Slug: 20261012-repo-wide-ids-and-board-lanes
- Status: ACTIVE
- Updated: 2026-10-12

## Tracker

One row per TASK. Group ties together items that ship as one transaction.

| Order | TASK                                                                   | Group | Status | Spec                                                                                                                         | Deps               | Context                                        | Notes                                               |
| ----- | ---------------------------------------------------------------------- | ----- | ------ | ---------------------------------------------------------------------------------------------------------------------------- | ------------------ | ---------------------------------------------- | --------------------------------------------------- |
| 1     | TASK-059 Repo-wide ID rule in the skill                                | S     | DONE   | [watchtower/tasks/TASK-059-repo-wide-id-rule.md](watchtower/tasks/TASK-059-repo-wide-id-rule.md)                             | -                  | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Skill text in the ~/.claude repo; not committed.    |
| 2     | TASK-060 ID migration script                                           | M     | DONE   | [watchtower/tasks/TASK-060-id-migration-script.md](watchtower/tasks/TASK-060-id-migration-script.md)                         | -                  | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | 4 tests pass.                                       |
| 3     | TASK-061 Migrate this repo's archive                                   | R     | DONE   | [watchtower/tasks/TASK-061-migrate-bachuc-archive.md](watchtower/tasks/TASK-061-migrate-bachuc-archive.md)                   | TASK-059, TASK-060 | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Applied after owner review; ADR-0003 fixed by hand. |
| 4     | TASK-062 Surface params in openSurface                                 | H     | DONE   | [watchtower/tasks/TASK-062-open-surface-params.md](watchtower/tasks/TASK-062-open-surface-params.md)                         | -                  | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Q-023. Release needs a new build.                   |
| 5     | TASK-063 Board server reads Watchtower                                 | B1    | DONE   | [watchtower/tasks/TASK-063-board-watchtower-state.md](watchtower/tasks/TASK-063-board-watchtower-state.md)                   | -                  | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | 10 ms for 50 runs.                                  |
| 6     | TASK-064 Board task lanes UI                                           | B2    | DONE   | [watchtower/tasks/TASK-064-board-task-lanes.md](watchtower/tasks/TASK-064-board-task-lanes.md)                               | TASK-063           | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Full cards in cells; manual check pending.          |
| 7     | TASK-065 Board keyboard and search                                     | B2    | DONE   | [watchtower/tasks/TASK-065-board-keyboard-search.md](watchtower/tasks/TASK-065-board-keyboard-search.md)                     | TASK-064           | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Manual check pending.                               |
| 8     | TASK-066 Open Watchtower at a workspace                                | B2    | DONE   | [watchtower/tasks/TASK-066-open-watchtower-link.md](watchtower/tasks/TASK-066-open-watchtower-link.md)                       | TASK-062, TASK-064 | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Manual check pending.                               |
| 9     | TASK-067 Board status columns with TASK chips                          | B3    | DONE   | [watchtower/tasks/TASK-067-board-status-columns-with-chips.md](watchtower/tasks/TASK-067-board-status-columns-with-chips.md) | TASK-066           | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Live checked; bachuc block 106 px.                  |
| 10    | TASK-068 Board search and keys on the two columns                      | B3    | DONE   | [watchtower/tasks/TASK-068-board-search-keys-on-columns.md](watchtower/tasks/TASK-068-board-search-keys-on-columns.md)       | TASK-067           | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Shares the Board client files with TASK-067.        |
| 11    | TASK-070 Board Needs you holds thread input only                       | B4    | DONE   | [watchtower/tasks/TASK-070-board-needs-you-input-only.md](watchtower/tasks/TASK-070-board-needs-you-input-only.md)           | TASK-068           | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | ADR-0013. Counts move to the plan line.             |
| 12    | TASK-071 Watchtower Needs you: full text, target link, no mock targets | W2    | DONE   | [watchtower/tasks/TASK-071-watchtower-needs-you-clarity.md](watchtower/tasks/TASK-071-watchtower-needs-you-clarity.md)       | -                  | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | ADR-0013. watchtower-board only.                    |
| 13    | TASK-069 Board Timeline tab                                            | B4    | TODO   | [watchtower/tasks/TASK-069-board-timeline-tab.md](watchtower/tasks/TASK-069-board-timeline-tab.md)                           | TASK-070           | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Design 1a with the 1c load strip. Q-024             |

TASK Status labels: TODO, IN PROGRESS, BLOCKED, DONE.

## Plan Verify

- `node ~/.claude/skills/watchtower/scripts/migrate-ids.mjs --repo .` -> prints "nothing to migrate" after TASK-061.
- `npm run typecheck` -> exit 0.
- In `hiep-plugins/plugins/board`: `npm run typecheck && npm run lint && npm test` -> all pass.
- In `hiep-plugins/plugins/watchtower-board`: `npm run typecheck && npm test` -> all pass except the pre-existing proposed-ADR assertion in `tests/overview.test.ts`.
- Human check on live: Board shows Needs you, Running, and Just finished, one bachuc plan line, and TASK chips on labelled cards; a chip opens Watchtower on the plan's workspace with that TASK selected.

## Handoff

- Next action: commit the done work, then run `implement` for TASK-069 (Board Timeline tab).
- TASK-070 and TASK-071 passed the live check on 2026-10-12 (Expo web on daemon 6768).
- All live test agents of this plan are archived. Their extra local bachuc workspaces (each `npm run cli -- run` made one) stay on live.
- TASK-067 and TASK-068 passed the live check on 2026-10-12 (Expo web on daemon 6768), including a compact-height fix.
- TASK-069 adds the Board Timeline tab (design 1a with the 1c load strip, Claude Design file `Board Timeline.dc.html`); Q-024 is open with a safe default.
- The three live manual checks passed on 2026-10-12 in the Expo web build on the live daemon, after three fixes: compact width (TASK-064), keys acting on a hidden Board page (TASK-065), and the collapsed Done group hiding the opener's TASK (TASK-066). See the outcomes.
- The `npm run dev:desktop` Electron window was not checked on screen: it stayed on an old frame and did not poll.
- Release: reload Board and watchtower-board on release only after a release build that includes TASK-062; older release hosts ignore the workspace param.
- Pre-existing: `hiep-plugins/plugins/watchtower-board/tests/overview.test.ts` expects a `proposed` ADR that no longer exists since commit 5357c1c.

## Archive

- None.
