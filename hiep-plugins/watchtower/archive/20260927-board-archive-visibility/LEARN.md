# Learn 20260927-board-archive-visibility

## Summary

Discrepancy: none. Planned checks passed; desktop UI acceptance remains unverified.

## Per TASK

- TASK-001: match. Archived cards disappear on refresh. Tests and live RPC confirm the behavior.
- Extra UI verification stopped when computer use reported that the user changed Paseo. No UI success is claimed.

## Plan-Level

- None. No Paseo source changes or agent archive state changes.

## Lessons

- Reconcile finished card membership as well as running state.
- Keep persisted cards consistent with the host directory after reloads.
