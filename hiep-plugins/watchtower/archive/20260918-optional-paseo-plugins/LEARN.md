# Learn 20260918-optional-paseo-plugins

## Summary

Five selected tasks completed. Four unselected proposals archived as TODO at user request.
No unfinished proposal is represented as shipped. Runtime results remain separate from source and test evidence.

## Per TASK

- TASK-001: match. Explicit read-only checks and panel shipped; TASK-009 later added discovery. Recorded compact acceptance uses narrow desktop panels, not a physical mobile device.
- TASK-002: not selected or implemented. Archived TODO; verification not run.
- TASK-003: not selected or implemented. Archived TODO; verification not run.
- TASK-004: not selected or implemented. Archived TODO; verification not run.
- TASK-005: match. Read-only board and native attachment picker shipped. Exact workspace/task search was verified after fixing the host picker filter mismatch.
- TASK-006: not selected or implemented. Archived TODO; verification not run.
- TASK-007: match. Dashboard grouping, progress, errors and themes shipped with recorded wide/narrow desktop checks. Physical mobile acceptance was not run.
- TASK-008: match. Explorer-only placement and project identity are present in source. Recorded native follow-up resolved the obsolete-center-tab cleanup gate.
- TASK-009: match. Discovery, workspace-bound MCP and optional typed Jev handoff shipped. Sixteen offline tests and three one-shot synthetic Jev cases passed. Review found fresh Yarn Classic PnP misclassified as missing node_modules; a regression fixture and fix resolved it. Final PATH probe adjustment passed offline/stdio checks and host reload; the additional native refresh was interrupted by user navigation.

## Plan-Level

- Task selection stayed explicit; the four proposals were never an approved implementation queue.
- Shared Group A files required scoped edits but did not require implementing unrelated tasks.
- Archive preserves task statuses and outcome evidence. Relative links were rebased to keep their original targets.
- Historical outcomes contain local-only artifacts and UI evidence; these are not portable production proof.

## Lessons

- Keep measurements, task coverage and model judgments separate. Evaluator failure must never imply readiness.
- Detect declared PnP before generated PnP files exist; use unknown for unsupported linker layouts.
- Use exact workspace context for MCP binding and preserve pre-existing entries.
- Verify native UI through the exact app bundle identity; retain narrow-desktop versus physical-mobile limits.
- Keep expected live labels outside evaluator inputs and preserve failures without answer retries.
