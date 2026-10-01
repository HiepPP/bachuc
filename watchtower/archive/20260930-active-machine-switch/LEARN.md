# Learn 20260930-active-machine-switch

## Summary

Discrepancy: 1 found. All 7 TASKs shipped as planned; the plan grew with several follow-ups recorded only in Handoff.

## Per TASK

- TASK-001: match.
- TASK-002: match.
- TASK-003: match.
- TASK-004: match. History logic moved into `useHistoryHost` to stay under the complexity limit.
- TASK-005: match. `OpenProjectListener` passes "all" after a review finding.
- TASK-006: match.
- TASK-007: plan said build Paseo Dev with the default config -> shipped after a repackage with `hardenedRuntime=false`. Mistake: ad-hoc signing with hardened runtime failed at launch. Fix: write the repackage flag into the build TASK.

## Plan-Level

- Scope creep: the `Cmd+`` host cycle, per-host last route, route reuse on switch, and native sidebar demand diff shipped as Handoff follow-ups without TASK specs. The native demand diff has unit tests only and no device run.

## Lessons

- Add a TASK row when a follow-up changes behavior, so it gets its own Verify.
- Put known build flags for Paseo Dev in the build TASK up front.
