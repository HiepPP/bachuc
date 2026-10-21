# NEXT

## Current Active Plan

- Title: GPUI composer prompt history and project menu actions
- Slug: 20261021-gpui-prompt-history-project-menu
- Status: ACTIVE
- Updated: 2026-10-21

## Tracker

| Order | TASK                                               | Group | Status      | Spec                                                                                                         | Deps | Context | Notes                                            |
| ----- | -------------------------------------------------- | ----- | ----------- | ------------------------------------------------------------------------------------------------------------ | ---- | ------- | ------------------------------------------------ |
| 1     | TASK-286 Composer prompt history with Up and Down  | A     | IN PROGRESS | [watchtower/tasks/TASK-286-composer-prompt-history.md](watchtower/tasks/TASK-286-composer-prompt-history.md) | -    | -       | GPUI-local history per repo, as Claude Code CLI. |
| 2     | TASK-287 Project menu settings, Finder, and Remove | B     | IN PROGRESS | [watchtower/tasks/TASK-287-project-menu-actions.md](watchtower/tasks/TASK-287-project-menu-actions.md)       | -    | -       | Same logic as the Electron project menu.         |

TASK Status labels: TODO, IN PROGRESS, BLOCKED, DONE.

## Plan Verify

- `cd packages/desktop-gpui && ./cargo.sh build -p bachuc-desktop` -> builds with no error.
- `cd packages/desktop-gpui && ./cargo.sh clean --profile dev` after the checks -> `target/debug` is gone; skip it when another cargo or rustc process runs.

## Review Focus

- A multi-line prompt with the cursor in a middle line -> Up and Down move the cursor, not the history; owner TASK-286.
- The slash or mention menu is open -> Up and Down move the menu selection, not the history; owner TASK-286.
- A repo with no saved prompt, or a missing or bad history file -> Up does nothing and nothing fails; owner TASK-286.
- Remove project when the daemon answers `accepted: false` -> a failure notification shows and the row stays; owner TASK-287.
- Remove project picked twice before the answer, or an answer that never comes -> only one request goes out, and the guard clears after a timeout; owner TASK-287.

## Handoff

- Next action: run `/watchtower implement`.

## Archive

- None.
