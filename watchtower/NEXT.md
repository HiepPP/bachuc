# NEXT

## Current Active Plan

- Title: GPUI text, shortcuts, sidebar preferences, and prompt footer
- Slug: 20261019-gpui-text-shortcuts-sidebar-footer
- Status: ACTIVE
- Updated: 2026-10-19

## Tracker

| Order | TASK                                               | Group | Status      | Spec                                                                                                                     | Deps               | Context                                        | Notes                                             |
| ----- | -------------------------------------------------- | ----- | ----------- | ------------------------------------------------------------------------------------------------------------------------ | ------------------ | ---------------------------------------------- | ------------------------------------------------- |
| 1     | TASK-256 Text drawn like Electron                  | A     | IN PROGRESS | [watchtower/tasks/TASK-256-text-like-electron.md](watchtower/tasks/TASK-256-text-like-electron.md)                       | -                  | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Turns off gpui glyph dilation.                    |
| 2     | TASK-257 Native macOS menu bar                     | A     | IN PROGRESS | [watchtower/tasks/TASK-257-native-menu-bar.md](watchtower/tasks/TASK-257-native-menu-bar.md)                             | TASK-256           | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Shares main.rs with TASK-256.                     |
| 3     | TASK-258 Sidebar collapse state persists           | B     | IN PROGRESS | [watchtower/tasks/TASK-258-sidebar-collapse-persists.md](watchtower/tasks/TASK-258-sidebar-collapse-persists.md)         | -                  | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Adds sidebar.json.                                |
| 4     | TASK-259 Client parses workspace display data      | C     | IN PROGRESS | [watchtower/tasks/TASK-259-client-workspace-display-data.md](watchtower/tasks/TASK-259-client-workspace-display-data.md) | -                  | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | PR, checks, labels, statusEnteredAt.              |
| 5     | TASK-260 Client rewind call and capabilities       | C     | IN PROGRESS | [watchtower/tasks/TASK-260-client-rewind.md](watchtower/tasks/TASK-260-client-rewind.md)                                 | TASK-259           | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Shares core.rs with TASK-259.                     |
| 6     | TASK-261 Display preferences menu                  | B     | IN PROGRESS | [watchtower/tasks/TASK-261-display-preferences-menu.md](watchtower/tasks/TASK-261-display-preferences-menu.md)           | TASK-258, TASK-259 | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Grouping, Title, Show, filters.                   |
| 7     | TASK-262 Sidebar row meta items                    | B     | IN PROGRESS | [watchtower/tasks/TASK-262-sidebar-row-meta-items.md](watchtower/tasks/TASK-262-sidebar-row-meta-items.md)               | TASK-261           | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Draws each Show item.                             |
| 8     | TASK-263 Prompt footer with time, rewind, and copy | B     | IN PROGRESS | [watchtower/tasks/TASK-263-prompt-footer.md](watchtower/tasks/TASK-263-prompt-footer.md)                                 | TASK-260           | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Runs after TASK-262 in group B (shared shell.rs). |

## Plan Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-sidebar -p bachuc-client` -> all tests pass.
- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-timeline -p bachuc-app` -> all tests pass.
- `cd packages/desktop-gpui && ./cargo.sh clippy --workspace` -> no new warnings in changed crates.
- Owner hand check on a check bundle against the live host: text, menu bar, collapse after relaunch, display preferences menu, and prompt footer match Electron.
- After the checks, quit the check app and run `./cargo.sh clean --profile dev` per [CLAUDE.md](CLAUDE.md).

## Review Focus

- `sidebar.json` is missing, empty, or holds bad JSON -> the sidebar opens with defaults and no crash; owner TASK-258.
- A collapsed project is deleted, then the app restarts -> the stale key is ignored and does not grow the file forever; owner TASK-258.
- An old host without `features.rewind`, or an agent with no rewind flags -> no rewind button shows; owner TASK-263.
- The rewind call fails -> a toast shows the error and the composer text does not change; owner TASK-263.
- A workspace label that is missing from the host label catalog -> it is dropped, then shows after a catalog update; owner TASK-259.

## Handoff

- Next action: run `/watchtower implement`.

## Archive

- None.
