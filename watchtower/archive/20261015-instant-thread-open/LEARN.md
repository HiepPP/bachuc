# Learn 20261015-instant-thread-open

## Summary

Discrepancy: 5 found. The code matches the design in all four phases, but the plan files and the ADR were written after the work, and three TASKs left gaps that later TASKs closed.

## Per TASK

- TASK-117: match.
- TASK-118: match. Chunked `initialize` was cut after a measured 7-14 ms for 5,000 rows.
- TASK-119: plan said reset the cache inside `primeTimelineFromLegacyProviderHistory` -> shipped no reset there. Mistake: a design gap; memory and cache already share one epoch, so the reset drops rows. Fix: none needed. Also left synchronous cache writes unguarded. Mistake: error paths were not in the spec. Fix: TASK-120 guarded them.
- TASK-120: plan listed six files -> shipped also the cache-write guard in `agent-manager.ts` and a fake-manager change in `wire-compat.test.ts`. Mistake: the spec missed that the shared test fake needs the new read method. Fix: list shared test fakes in the spec.
- TASK-121: match.
- TASK-122: plan section 4.4 asked to re-land after an epoch change -> not built. A bottom open through host navigation did not end a pending anchor. Mistake: the file that opens agents was outside its ownership. Fix: TASK-124 added `cancelLatestPromptAnchor`; re-landing stays open.
- TASK-123: plan said skip desired agents -> shipped an approximate check. Mistake: no public desired check existed in a file another TASK owned. Fix: TASK-124 added `isAgentDesired`.
- TASK-124: plan was the board calls only -> shipped two follow-ups in files of TASK-121, TASK-122, and TASK-123. Mistake: none, but it broke the one-owner-per-file rule across groups. Fix: put follow-ups in the group that owns the file.
- TASK-125: match. The orphan `db:query` script in the root `package.json` was left.

## Plan-Level

- No watchtower plan existed when the work started. The design lived only in a `/tmp` scratch file, and this archive was written after the work.
- ADR-0031 was written after implementation, not when the owner chose `node:sqlite`.
- The `wire-compat.test.ts` failure came from commit `df71368e7` of the earlier background-process work, which did not run that test.
- Three checks stay open: a click in the live app, a real Codex backfill, and a release build.

## Lessons

- When the owner approves a multi-TASK design, run `watchtower new` first, so the specs, Verify lines, and ADR exist before builders start.
- Record an ADR in the same step as the choice.
- When a change adds a field to `ManagedAgent` or the agent manager surface, run the tests that hold shared fakes, such as `wire-compat.test.ts`.
- Give each TASK the files it must call into, or plan a follow-up TASK in the owning group.
