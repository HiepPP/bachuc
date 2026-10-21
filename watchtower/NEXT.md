# NEXT

## Current Active Plan

- Title: GPUI branch menu issues and pull requests rows
- Slug: 20261021-gpui-branch-menu-issues-prs
- Status: DONE
- Updated: 2026-10-21

## Tracker

| Order | TASK                                                    | Group | Status | Spec                                                                                                                 | Deps | Context | Notes                                                  |
| ----- | ------------------------------------------------------- | ----- | ------ | -------------------------------------------------------------------------------------------------------------------- | ---- | ------- | ------------------------------------------------------ |
| 1     | TASK-281 GPUI branch menu issues and pull requests rows | A     | DONE   | [watchtower/tasks/TASK-281-gpui-branch-menu-issues-prs.md](watchtower/tasks/TASK-281-gpui-branch-menu-issues-prs.md) | -    | -       | GitHub remotes only; implement: manual checks pending. |

## Plan Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-plugin-thread-branch` -> all tests pass.

## Review Focus

- A GitLab remote -> the two rows stay hidden; owner TASK-281
- No remote -> the two rows stay hidden; owner TASK-281
- A GitHub remote with an upstream -> the new rows sit between the Open row and the divider; owner TASK-281
- A GitHub remote with no upstream -> the rows show and the divider sits before Refresh; owner TASK-281

## Handoff

- Next action: Hiep checks the branch pill menu in a GPUI build, then merges `feat/gpui-branch-menu-issues-prs` and runs `/watchtower archive`.

## Archive

- None.
