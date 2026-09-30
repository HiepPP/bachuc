# Learn 20260920-arc-style-project-spaces

## Summary

Three tasks remain open: TASK-004, TASK-013, and TASK-014. Archiving does not complete their acceptance checks.
This review uses recorded outcomes and this session's Board implementation evidence. Earlier checks were not rerun.

## Per TASK

- TASK-001: match after the approved sidebar update. Earlier standalone-only contract text remains historical.
- TASK-002: match. Recorded checks cover persistent Spaces and restored project membership.
- TASK-003: match for desktop tabs and navigation. Narrow/mobile acceptance remains limited.
- TASK-004: gesture code exists, but physical trackpad acceptance remains BLOCKED. Fix: test on the physical device.
- TASK-005: match for project moves and persistence. Its TASK-004 dependency stayed blocked on a separate hardware check.
- TASK-006: match for conversation cards, outcomes, navigation, and removal. Native mobile remains unverified.
- TASK-007: match for broker routing and settings preservation. The later direct route has separate recorded evidence.
- TASK-008: match for bounded recovery logic. Live escalation benefits remain unmeasured.
- TASK-009: match for discovery ordering and evidence handoff. Provider read-only instructions are not an OS sandbox.
- TASK-010: match for independent review and authoritative checks. Unequal review scope limits timing comparisons.
- TASK-011: match for declared dependency and resource locks. Undeclared shared resources remain outside enforcement.
- TASK-012: measurement work matches the plan. Benchmarks did not show token or speed savings.
- TASK-013: implementation and live sends are recorded. Layout, retry, toggle-off, and cleanup acceptance remain open.
- TASK-014: project groups and starred placement are implemented and reloaded. Desktop checks and 27 tests passed.
  Multi-client, compact/mobile, finished-card, and Needs input runtime checks remain BLOCKED by missing test states or clients.

## Plan-Level

- Earlier outcome contract sections describe standalone-only Spaces, while later sections record approved private sidebar integration.
  Cause: updates were appended after scope changed. Fix: distinguish historical contracts from the final contract in future outcomes.
- TASK-005 is DONE despite its dependency on BLOCKED TASK-004. The remaining dependency is hardware acceptance, not move implementation.
  Fix: separate implementation dependencies from device acceptance gates when authoring the next plan.
- The title broadened while the original slug remained stable. This is expected; archive reuses that slug.
- Keep the outstanding TASK-013 shared-fixture cleanup visible. Confirm ownership before changing that fixture.
- Code changes from TASK-014 remain uncommitted. This archive changes planning files only.

## Lessons

- Prepare representative finished, Needs input, compact, and multi-client states before UI acceptance.
- Keep group identity separate from project display names.
- Split starred cards before grouping to avoid duplicates and incorrect counts.
- Preserve missing checks and negative benchmark results when archiving.
