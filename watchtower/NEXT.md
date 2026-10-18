# NEXT

## Current Active Plan

- Title: GPUI zoom and follow-ups
- Slug: 20261018-gpui-zoom-and-follow-ups
- Status: ACTIVE
- Updated: 2026-10-18

## Tracker

One row per TASK. Group ties together items that ship as one transaction. Group T runs its four TASKs in order, because they share timeline files. Groups C, A, E, B, and P share no file with group T or with each other, so they can run in parallel.

| Order | TASK                                                | Group | Status  | Spec                                                                                                                   | Deps               | Context                                        | Notes                                                                                      |
| ----- | --------------------------------------------------- | ----- | ------- | ---------------------------------------------------------------------------------------------------------------------- | ------------------ | ---------------------------------------------- | ------------------------------------------------------------------------------------------ |
| 1     | TASK-243 Vendor list spacing option                 | T     | BLOCKED | [watchtower/tasks/TASK-243-vendor-list-spacing.md](watchtower/tasks/TASK-243-vendor-list-spacing.md)                   | -                  | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Q-144: top 4 px, bottom 16 px. Code committed; UI checks pending.                          |
| 2     | TASK-244 Hide plugin items with no renderer         | T     | BLOCKED | [watchtower/tasks/TASK-244-hide-unrendered-plugin-items.md](watchtower/tasks/TASK-244-hide-unrendered-plugin-items.md) | TASK-243           | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Q-148. Code committed; UI checks pending.                                                  |
| 3     | TASK-245 Corner pills keep clear of message text    | T     | BLOCKED | [watchtower/tasks/TASK-245-corner-pills-clear-text.md](watchtower/tasks/TASK-245-corner-pills-clear-text.md)           | TASK-244           | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Right gutter when the pane is narrow (1200 px window). Blocked: BUILT, UNVERIFIED (Q-161). |
| 4     | TASK-246 Zoom the timeline sizes                    | T     | BLOCKED | [watchtower/tasks/TASK-246-zoom-timeline.md](watchtower/tasks/TASK-246-zoom-timeline.md)                               | TASK-243, TASK-245 | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Q-151, timeline crate. Code committed; UI checks pending.                                  |
| 5     | TASK-247 Zoom composer, command, plugin host        | C     | BLOCKED | [watchtower/tasks/TASK-247-zoom-composer-command.md](watchtower/tasks/TASK-247-zoom-composer-command.md)               | -                  | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Q-151. Not `plugin_slots.rs` (TASK-245 owns it). Code committed; UI checks pending.        |
| 6     | TASK-248 Zoom app, workspace, sidebar               | A     | BLOCKED | [watchtower/tasks/TASK-248-zoom-app-workspace-sidebar.md](watchtower/tasks/TASK-248-zoom-app-workspace-sidebar.md)     | -                  | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Q-151. Not `panes.rs` (TASK-245 owns it). Code committed; UI checks pending.               |
| 7     | TASK-249 Zoom explorer, terminal, history, settings | E     | BLOCKED | [watchtower/tasks/TASK-249-zoom-explorer-settings.md](watchtower/tasks/TASK-249-zoom-explorer-settings.md)             | -                  | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Q-151. Code committed; UI checks pending.                                                  |
| 8     | TASK-250 Zoom board and watchtower-board            | B     | BLOCKED | [watchtower/tasks/TASK-250-zoom-board-plugins.md](watchtower/tasks/TASK-250-zoom-board-plugins.md)                     | -                  | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Q-151. Code committed; UI checks pending.                                                  |
| 9     | TASK-251 Zoom the other plugin crates               | P     | BLOCKED | [watchtower/tasks/TASK-251-zoom-other-plugins.md](watchtower/tasks/TASK-251-zoom-other-plugins.md)                     | -                  | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Q-151. Code committed; UI checks pending.                                                  |

TASK Status labels: TODO, IN PROGRESS, BLOCKED, DONE.

## Plan Verify

- `cd packages/desktop-gpui && ./cargo.sh build -p bachuc-app` -> builds with no new warnings in changed files.
- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-timeline -p bachuc-composer -p bachuc-command -p bachuc-plugin-host -p bachuc-app -p bachuc-workspace -p bachuc-sidebar -p bachuc-explorer -p bachuc-terminal -p bachuc-history -p bachuc-settings` plus every changed plugin crate, in one cargo call -> all pass.
- `cd packages/desktop-gpui && ./cargo.sh fmt --check` for each changed crate -> exit 0.
- `df -h /` between groups -> at least 20 GB free before the next group starts; otherwise run `./cargo.sh clean --profile dev` when no other cargo runs.
- UI checker: one check bundle, `BACHUC_WINDOW=1200x800` and `BACHUC_WINDOW=1600x900`, at zoom 100% and 150%. Screenshot the thread, composer, sidebar, explorer, settings, Board, and Watchtower board.
- `cd packages/desktop-gpui && ./cargo.sh clean --profile dev` after the last check, when no other cargo runs.

## Review Focus

- Zoom 200% in a 900 px window -> text wraps, no control clips its label; owner TASK-246.
- A run finishes while its thread shows, so the Remove pills appear -> the gutter appears on the next frame and stays still; owner TASK-245.
- The Board store goes offline, so the pills hide -> the gutter goes back to 0; owner TASK-245.
- A message that ends with a list, and a nested list -> the 16 px bottom gap shows only under the top-level list; owner TASK-243.
- A projected entry whose only parts are plugin items with no renderer -> the row draws nothing and adds no gap; owner TASK-244.

## Handoff

- Code for TASK-243 to TASK-251 is committed on `feat/gpui-zoom-and-follow-ups`, not pushed. Plan Verify code checks pass.
- Next action: Hiep runs the hand checks of Q-161, then `/watchtower verify` and `/watchtower archive`.

## Archive

- None.
