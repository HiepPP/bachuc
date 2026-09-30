# TASK-002 Persist project Spaces

Group: A (shared plugin contracts and page)
Class: code

## Brief

Goal: Store ordered Spaces and host-qualified project memberships.

Change: Implement the approved desktop sidebar adapter; do not change Paseo source.

How:

- Use revision-checked settings. Default all unassigned projects to Workspace 1.
- Preserve existing projects, agents, paths, and unrelated changes.

Files:

- [Implementation](../../../../plugins/workspace-spaces/shared/spaces.ts) and focused tests under [tests](../../../../plugins/workspace-spaces/tests).
- [Plugin README](../../../../plugins/workspace-spaces/README.md) and [catalog](../../../../README.md).

Expected result: Store ordered Spaces and host-qualified project memberships.

## Verify

- Test defaults, repeated initialization, cross-host identity, invalid targets, and saved state reload.
- Run plugin formatting, typecheck, lint, and tests before runtime acceptance.
