# TASK-004 Swipe between Spaces

Group: A (shared plugin contracts and page)
Class: code

## Brief

Goal: Switch adjacent Spaces inside the native sidebar project list.

Change: Implement the approved desktop sidebar adapter; do not change Paseo source.

How:

- Handle horizontal touch and trackpad intent, boundaries, vertical scrolling, and momentum. Clean up listeners.
- Preserve existing projects, agents, paths, and unrelated changes.

Files:

- [Implementation](../../../../plugins/workspace-spaces/client/gesture.ts) and focused tests under [tests](../../../../plugins/workspace-spaces/tests).
- [Plugin README](../../../../plugins/workspace-spaces/README.md) and [catalog](../../../../README.md).

Expected result: Switch adjacent Spaces inside the native sidebar project list.

## Verify

- Test threshold, directions, diagonal motion, momentum, and boundaries. Record hardware checks separately.
- Run plugin formatting, typecheck, lint, and tests before runtime acceptance.
