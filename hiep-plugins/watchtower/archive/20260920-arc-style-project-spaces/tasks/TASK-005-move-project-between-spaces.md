# TASK-005 Move projects between Spaces

Group: A (shared plugin contracts and page)
Class: code

## Brief

Goal: Move existing projects between numbered Spaces without modifying workspace identities.

Change: Implement the approved desktop sidebar adapter; do not change Paseo source.

How:

- Provide a destination picker. Keep the selected Space unchanged after moves. Preserve saved state on failures.
- Preserve existing projects, agents, paths, and unrelated changes.

Files:

- [Implementation](../../../../plugins/workspace-spaces/client/web.ts) and focused tests under [tests](../../../../plugins/workspace-spaces/tests).
- [Plugin README](../../../../plugins/workspace-spaces/README.md) and [catalog](../../../../README.md).

Expected result: Move existing projects between numbered Spaces without modifying workspace identities.

## Verify

- Test moves and rejected targets. Run all checks, install/reload, and verify live page behavior.
- Run plugin formatting, typecheck, lint, and tests before runtime acceptance.
