# TASK-001 Confirm desktop sidebar integration

Group: A (shared plugin contracts and page)
Class: docs

## Brief

Goal: Confirm surface, host settings, catalog, and workspace navigation support.

Change: Implement the approved desktop sidebar adapter; do not change Paseo source.

How:

- Inspect installed SDK and matching host version. Inject owned sidebar controls and return complete cleanup.
- Preserve existing projects, agents, paths, and unrelated changes.

Files:

- [Implementation](../../../../plugins/workspace-spaces/index.client.tsx) and focused tests under [tests](../../../../plugins/workspace-spaces/tests).
- [Plugin README](../../../../plugins/workspace-spaces/README.md) and [catalog](../../../../README.md).

Expected result: Confirm surface, host settings, catalog, and workspace navigation support.

## Verify

- Install and confirm running without load errors.
- Run plugin formatting, typecheck, lint, and tests before runtime acceptance.
