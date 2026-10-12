# TASK-063 Outcome

## Outcome

Status: DONE

Changed:

- New `hiep-plugins/plugins/board/server/watchtower.ts`: trimmed copies of watchtower-board's table parsers, `parsePlan`, `gitRoot`, `readPlan`, `runTask`, and `attachPlans`.
- `shared/board.ts`: runs carry optional `taskId`, `planRoot`, and `workspaceId`; the snapshot carries optional `plans`; new `WATCHTOWER_TASK_LABEL` and `SCHEDULE_RUN_LABEL`.
- `server/snapshot.ts`: `listBoardAgents` also returns `facts` per agent: the TASK label, whether it is a schedule run, and its workspace.
- `index.server.ts`: the snapshot handler calls `attachPlans`.
- New `tests/watchtower.test.ts`; `tests/snapshot.test.ts` expects the new `facts` field.

Contract:

- A run's TASK comes from `watchtower.task`; for a schedule iteration, from the RUN.md row that covers its start; while it runs with no row yet, from the only IN PROGRESS TASK.
- Plans are read from the git root above each run's `cwd`, once per root per snapshot. Files over 128 KiB, an ARCHIVED plan, or a Tracker with no TASK give no plan.
- New fields are optional, so an older client still parses the snapshot.

Verified:

- `npx tsx --test tests/watchtower.test.ts tests/snapshot.test.ts` -> 12 pass.
- `npm test` -> 118 pass, 0 fail.
- `npm run typecheck` -> exit 0.
- `npm run-script lint` -> 0 warnings, 0 errors.

Anti-goal:

- Before the readers: no timing case existed; `attachPlans` was not called (0 ms added).
- After the mapping: the 50-run, 3-root case passes its 50 ms assertion; whole test 10.0 ms; 2026-10-12.
- Final: same case, 10.0 ms including a warm-up call; `tests/watchtower.test.ts`, 2026-10-12.
- Result: PASS against 50 ms.

Lessons:

- Board's tests run with `tsx --test` (node:test), not vitest.
