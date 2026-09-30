# TASK-003 Add numbered workspace tabs

Group: A (shared plugin contracts and page)
Class: code

## Brief

Goal: Create and select numbered tabs inside the native desktop sidebar.

Change: Implement the approved desktop sidebar adapter; do not change Paseo source.

How:

- Show project groups and existing coding workspaces. Keep footer visible and support overflow and empty states.
- Preserve existing projects, agents, paths, and unrelated changes.

Files:

- [Implementation](../../../../plugins/workspace-spaces/client/web.ts) and focused tests under [tests](../../../../plugins/workspace-spaces/tests).
- [Plugin README](../../../../plugins/workspace-spaces/README.md) and [catalog](../../../../README.md).

Expected result: Create and select numbered tabs inside the native desktop sidebar.

## Verify

- Check create, click navigation, empty state, workspace links, and narrow layout.
- Run plugin formatting, typecheck, lint, and tests before runtime acceptance.
