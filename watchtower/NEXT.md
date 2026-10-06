# NEXT

## Current Active Plan

- Title: Port T3 Code orchestration and reply features
- Slug: 20261006-t3code-orchestration-port
- Status: ACTIVE
- Updated: 2026-10-06

## Tracker

One row per TASK. Group ties together items that write the same files.

| Order | TASK                                                     | Group | Status | Spec                                                                                                             | Deps               | Context                                        | Notes                                                                                 |
| ----- | -------------------------------------------------------- | ----- | ------ | ---------------------------------------------------------------------------------------------------------------- | ------------------ | ---------------------------------------------- | ------------------------------------------------------------------------------------- |
| 1     | TASK-001 Cascade Stop to managed subagents               | A     | DONE   | [watchtower/tasks/TASK-001-cascade-stop.md](watchtower/tasks/TASK-001-cascade-stop.md)                           | -                  | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | A2. auto: manual checks pending. Adds the disarm hook that TASK-002 and TASK-003 use. |
| 2     | TASK-002 Coalesce child finish notices                   | A     | TODO   | [watchtower/tasks/TASK-002-coalesce-finish-notices.md](watchtower/tasks/TASK-002-coalesce-finish-notices.md)     | TASK-001           | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | A1. Q-001.                                                                            |
| 3     | TASK-003 Daemon-side message queue                       | A     | TODO   | [watchtower/tasks/TASK-003-daemon-message-queue.md](watchtower/tasks/TASK-003-daemon-message-queue.md)           | TASK-001           | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | A3 daemon part. Protocol change. Q-002, Q-003, Q-004.                                 |
| 4     | TASK-004 App uses the daemon queue                       | B     | TODO   | [watchtower/tasks/TASK-004-app-daemon-queue.md](watchtower/tasks/TASK-004-app-daemon-queue.md)                   | TASK-003           | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | A3 app part.                                                                          |
| 5     | TASK-005 PR watch plugin                                 | C     | TODO   | [watchtower/tasks/TASK-005-pr-watch-plugin.md](watchtower/tasks/TASK-005-pr-watch-plugin.md)                     | -                  | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | A7. Plugin only. Q-005, Q-006.                                                        |
| 6     | TASK-006 Host API for selection actions and plugin chips | B     | TODO   | [watchtower/tasks/TASK-006-selection-action-host-api.md](watchtower/tasks/TASK-006-selection-action-host-api.md) | -                  | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | B3 host part 1. Q-007.                                                                |
| 7     | TASK-010 Host API to reveal a quoted passage             | B     | TODO   | [watchtower/tasks/TASK-010-reveal-passage-host-api.md](watchtower/tasks/TASK-010-reveal-passage-host-api.md)     | TASK-006           | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | B3 host part 2: jump back to the source. Q-007.                                       |
| 8     | TASK-007 Assistant cite plugin                           | D     | TODO   | [watchtower/tasks/TASK-007-assistant-cite-plugin.md](watchtower/tasks/TASK-007-assistant-cite-plugin.md)         | TASK-006, TASK-010 | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | B3 plugin part. Q-007.                                                                |
| 9     | TASK-008 Host API for a sandboxed HTML frame             | B     | TODO   | [watchtower/tasks/TASK-008-html-frame-host-api.md](watchtower/tasks/TASK-008-html-frame-host-api.md)             | -                  | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | B4 host part. Q-008, ADR-0002.                                                        |
| 10    | TASK-009 HTML reply plugin                               | E     | TODO   | [watchtower/tasks/TASK-009-html-reply-plugin.md](watchtower/tasks/TASK-009-html-reply-plugin.md)                 | TASK-008           | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | B4 plugin part. Q-008, ADR-0002.                                                      |

TASK Status labels: TODO, IN PROGRESS, BLOCKED, DONE.
Plan-level Status header: ACTIVE while any row is open, DONE when all rows DONE, ARCHIVED after archive.

## Plan Verify

- `npm run typecheck` -> exit 0.
- `npm run lint` on every changed host file -> exit 0.
- `npm run format:check:files -- <changed host files>` -> exit 0.
- In each new plugin folder: `npm test`, `npm run typecheck`, `npm run lint` -> exit 0.
- Live host check (needs the live daemon on port 6768 and a human): a parent agent with two subagents shows one combined finish notice, and Stop on the parent stops both.

## Handoff

- Next action: The autorun loop builds TASK-002 next. It must keep the in-flight delivery cancel from TASK-001 (see [watchtower/tasks/TASK-001-outcome.md](watchtower/tasks/TASK-001-outcome.md)).
- Manual check pending: Stop on a live parent with a running subagent (TASK-001).

## Archive

- [watchtower/archive/20260930-active-machine-switch/](watchtower/archive/20260930-active-machine-switch/)
- [watchtower/archive/20260930-plugin-host-apis/](watchtower/archive/20260930-plugin-host-apis/)
