# NEXT

## Current Active Plan

- Title: GPUI polish and fixes
- Slug: 20261018-gpui-polish-and-fixes
- Status: ACTIVE
- Updated: 2026-10-18

## Tracker

One row per TASK. Group ties together items that ship as one transaction. Group O runs first because it may touch any view crate. Group H runs last because it touches files of groups S, C, W, and T.

| Order | TASK                                               | Group | Status      | Spec                                                                                                                   | Deps                                   | Context                                        | Notes                                                     |
| ----- | -------------------------------------------------- | ----- | ----------- | ---------------------------------------------------------------------------------------------------------------------- | -------------------------------------- | ---------------------------------------------- | --------------------------------------------------------- |
| 1     | TASK-232 Text overflow on narrow windows           | O     | IN PROGRESS | [watchtower/tasks/TASK-232-text-overflow-narrow-windows.md](watchtower/tasks/TASK-232-text-overflow-narrow-windows.md) | -                                      | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Horizontal and vertical overflow; scan and fix all views. |
| 2     | TASK-233 Thread spacing like Electron              | T     | IN PROGRESS | [watchtower/tasks/TASK-233-thread-spacing-like-electron.md](watchtower/tasks/TASK-233-thread-spacing-like-electron.md) | TASK-232                               | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Measure Electron, then apply.                             |
| 3     | TASK-234 Whole-UI zoom with Cmd+=, Cmd-, Cmd+0     | S     | IN PROGRESS | [watchtower/tasks/TASK-234-whole-ui-zoom.md](watchtower/tasks/TASK-234-whole-ui-zoom.md)                               | TASK-232                               | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Saved in appearance.json.                                 |
| 4     | TASK-235 Type anywhere to focus the composer       | S     | IN PROGRESS | [watchtower/tasks/TASK-235-type-to-focus-composer.md](watchtower/tasks/TASK-235-type-to-focus-composer.md)             | TASK-234                               | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Shares shell.rs with TASK-234.                            |
| 5     | TASK-236 Tab close button on the left              | S     | IN PROGRESS | [watchtower/tasks/TASK-236-tab-close-button-left.md](watchtower/tasks/TASK-236-tab-close-button-left.md)               | TASK-235                               | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Setting Left or Right, default Left.                      |
| 6     | TASK-237 Clickable menu buttons and real app icons | S     | IN PROGRESS | [watchtower/tasks/TASK-237-menu-buttons-and-app-icons.md](watchtower/tasks/TASK-237-menu-buttons-and-app-icons.md)     | TASK-236                               | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Follows ADR-0046.                                         |
| 7     | TASK-238 Slash picker with skills like Electron    | C     | IN PROGRESS | [watchtower/tasks/TASK-238-slash-picker-skills.md](watchtower/tasks/TASK-238-slash-picker-skills.md)                   | TASK-232                               | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Missing skills, picker not opening, look.                 |
| 8     | TASK-239 Sidebar space swipe follows the finger    | W     | IN PROGRESS | [watchtower/tasks/TASK-239-sidebar-swipe-follows-finger.md](watchtower/tasks/TASK-239-sidebar-swipe-follows-finger.md) | TASK-232                               | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Owner hand test.                                          |
| 9     | TASK-240 Wider watchtower board                    | B     | IN PROGRESS | [watchtower/tasks/TASK-240-wider-watchtower-board.md](watchtower/tasks/TASK-240-wider-watchtower-board.md)             | TASK-232                               | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Overlay and TASK dialog.                                  |
| 10    | TASK-241 Turn diff recap shows on each response    | D     | IN PROGRESS | [watchtower/tasks/TASK-241-turn-diff-recap-shows.md](watchtower/tasks/TASK-241-turn-diff-recap-shows.md)               | TASK-232                               | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Find why the card does not show.                          |
| 11    | TASK-242 Dropdowns attach to their button          | H     | IN PROGRESS | [watchtower/tasks/TASK-242-dropdowns-attach-to-button.md](watchtower/tasks/TASK-242-dropdowns-attach-to-button.md)     | TASK-233, TASK-237, TASK-238, TASK-239 | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Runs last.                                                |

TASK Status labels: TODO, IN PROGRESS, BLOCKED, DONE.

## Plan Verify

- `cd packages/desktop-gpui && ./cargo.sh build -p bachuc-app` -> builds with no new warnings in changed files.
- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-timeline -p bachuc-workspace -p bachuc-sidebar -p bachuc-composer -p bachuc-settings -p bachuc-plugin-thread-branch -p bachuc-plugin-workspace-spaces` -> all pass. Run in one cargo call so gpui-kit builds once.
- UI checker: build a check bundle under its own name, open it with `open -n -g --env BACHUC_NO_ACTIVATE=1`, and screenshot each TASK screen at 900 px and 1600 px window width.
- Owner hand test at the end: swipe (TASK-239), zoom (TASK-234), type-to-focus (TASK-235).
- `cd packages/desktop-gpui && ./cargo.sh clean --profile dev` after the last check, when no other cargo runs.

## Review Focus

- Window width under 700 px with a long unbreakable token, such as a path or URL, in a message and in an AskUserQuestion option -> it wraps or breaks inside its card; owner TASK-232.
- Zoom at the limits (50% and 200%), then app restart -> the level stays and stops at the limit; owner TASK-234.
- Typing while the terminal, a settings field, or a search box has focus -> the key stays there; owner TASK-235.
- A swipe that starts while a space save is busy, and a short drag that does not pass half the width -> the swipe runs after the save, and a short drag snaps back; owner TASK-239.
- An editor app that is not installed, such as Zed missing -> the menu row shows a fallback icon and no crash; owner TASK-237.

## Handoff

- Next action: run `/watchtower implement`.

## Archive

- None.
