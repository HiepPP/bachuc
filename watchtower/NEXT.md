# NEXT

## Current Active Plan

- Title: GPUI sidebar saved order like Electron
- Slug: 20261020-gpui-sidebar-saved-order
- Status: ACTIVE
- Updated: 2026-10-20

## Tracker

One row per TASK. Group ties together items that ship as one transaction.

| Order | TASK                                                        | Group | Status | Spec                                                                                                                               | Deps     | Context                                        | Notes                                                                  |
| ----- | ----------------------------------------------------------- | ----- | ------ | ---------------------------------------------------------------------------------------------------------------------------------- | -------- | ---------------------------------------------- | ---------------------------------------------------------------------- |
| 1     | TASK-278 Saved sidebar order with Electron's rules          | A     | TODO   | [watchtower/tasks/TASK-278-gpui-sidebar-saved-order.md](watchtower/tasks/TASK-278-gpui-sidebar-saved-order.md)                     | -        | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Name sort, then the saved order; new threads first, new projects last. |
| 2     | TASK-279 One-time import of Electron's saved order          | A     | TODO   | [watchtower/tasks/TASK-279-gpui-import-electron-sidebar-order.md](watchtower/tasks/TASK-279-gpui-import-electron-sidebar-order.md) | TASK-278 | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Reads a temp copy of Electron's LevelDB store with `rusty-leveldb`.    |
| 3     | TASK-280 Drag to reorder projects, threads, and pinned rows | A     | TODO   | [watchtower/tasks/TASK-280-gpui-sidebar-drag-reorder.md](watchtower/tasks/TASK-280-gpui-sidebar-drag-reorder.md)                   | TASK-278 | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Drag needs the owner's hand check.                                     |

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

- Next action: run `/watchtower implement`. TASK-278 goes first, then TASK-279 and TASK-280 in one group, because all three write `crates/sidebar/src/sidebar.rs`.

## Archive

- None.
