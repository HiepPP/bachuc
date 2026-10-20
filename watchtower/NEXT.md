# NEXT

## Current Active Plan

- Title: GPUI sidebar saved order like Electron
- Slug: 20261020-gpui-sidebar-saved-order
- Status: ACTIVE (built; owner checks pending)
- Updated: 2026-10-20

## Tracker

One row per TASK. Group ties together items that ship as one transaction.

| Order | TASK                                                        | Group | Status  | Spec                                                                                                                               | Deps     | Context                                        | Notes                                                                                                         |
| ----- | ----------------------------------------------------------- | ----- | ------- | ---------------------------------------------------------------------------------------------------------------------------------- | -------- | ---------------------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| 1     | TASK-278 Saved sidebar order with Electron's rules          | A     | DONE    | [watchtower/tasks/TASK-278-gpui-sidebar-saved-order.md](watchtower/tasks/TASK-278-gpui-sidebar-saved-order.md)                     | -        | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Name sort, then the saved order; new threads first, new projects last.                                        |
| 2     | TASK-279 One-time import of Electron's saved order          | A     | DONE    | [watchtower/tasks/TASK-279-gpui-import-electron-sidebar-order.md](watchtower/tasks/TASK-279-gpui-import-electron-sidebar-order.md) | TASK-278 | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Reads a temp copy of Electron's LevelDB store with `rusty-leveldb`. implement: manual checks pending (Q-182). |
| 3     | TASK-280 Drag to reorder projects, threads, and pinned rows | A     | BLOCKED | [watchtower/tasks/TASK-280-gpui-sidebar-drag-reorder.md](watchtower/tasks/TASK-280-gpui-sidebar-drag-reorder.md)                   | TASK-278 | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Blocked: BUILT, UNVERIFIED (Q-180).                                                                           |

TASK Status labels: TODO, IN PROGRESS, BLOCKED, DONE.

## Plan Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-sidebar` -> all sidebar tests pass.
- `cd packages/desktop-gpui && ./cargo.sh clippy -p bachuc-sidebar -- -D warnings` -> no warnings.
- Owner check on a check bundle against release: the sidebar order matches Electron's, a new thread shows first in its project, and a drag survives a restart.

## Review Focus

- An empty saved order -> the rows stay the same as before this plan; owner TASK-278.
- No server id yet, before `server_info` arrives -> threads keep the name order and are not seeded; owner TASK-278.
- Electron's folder is missing, or Electron holds its LOCK file -> the import still sets its flag, or reads from the copy; owner TASK-279.
- A drag while a project filter or "show more" hides rows -> the hidden keys stay in the saved order; owner TASK-280.
- A drag in status grouping, or a drop on another project -> nothing moves; owner TASK-280.

## Handoff

- Built on `feat/gpui-sidebar-saved-order` in `9cc6e3d50`, not pushed.
- Next action: Hiep runs the Q-180 drag checks and the Q-182 order check on a check bundle. Then answer both rows in [watchtower/QUESTIONS.md](watchtower/QUESTIONS.md) and run `/watchtower verify`.
- Plan Verify: the crate tests pass. The clippy line fails on old code outside this plan; use `./cargo.sh clippy -p bachuc-sidebar --no-deps --lib --test model --test electron_order -- -D warnings -A clippy::too_many_arguments`.

## Archive

- None.
