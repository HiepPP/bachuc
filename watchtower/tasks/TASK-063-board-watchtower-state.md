# TASK-063 Board server reads Watchtower

Group: B1 (`hiep-plugins/plugins/board/server/**`, `hiep-plugins/plugins/board/shared/**`, their tests)
Class: code

## Brief

Goal: Board's snapshot tells the client which TASK each run belongs to, and gives each project's active plan with its TASK rows and Needs you items.

Change: runs with a project only -> runs with an optional `taskId`, plus one optional `plans` list per snapshot.

Boundaries:

- Follows [ADR-0010](watchtower/decisions/ADR-0010-task-lanes-over-status-columns.md).
- TASK of a run: the `watchtower.task` label first. For a schedule iteration (label `paseo.schedule-run`), use the `watchtower/RUN.md` log row whose Start and End cover the run's start. Otherwise none.
- Plan per project: read `watchtower/NEXT.md`, `QUESTIONS.md`, and the Handoff `Manual check pending:` bullets from the git root of the run's `cwd`. Read each root once per snapshot.
- Copy the small table parser from watchtower-board; do not import across plugins.
- New schema fields are optional, so an old client still parses the snapshot.
- Missing or broken plan files give no plan for that project, never an error for the whole snapshot.

How:

- Add `server/watchtower.ts` with the readers and the run-to-TASK mapping.
- Extend `runSchema` with `taskId` and the snapshot with `plans`: project key, title, done count, total, step, autorun state, rows (`id`, title, status, deps, blocking Q-IDs), questions, and manual checks.
- Add tests with fixture plan folders.

Files:

- [hiep-plugins/plugins/board/server/watchtower.ts](hiep-plugins/plugins/board/server/watchtower.ts) (new)
- [hiep-plugins/plugins/board/server/snapshot.ts](hiep-plugins/plugins/board/server/snapshot.ts) (attach `taskId` and `plans`)
- [hiep-plugins/plugins/board/shared/board.ts](hiep-plugins/plugins/board/shared/board.ts) (schema)
- [hiep-plugins/plugins/board/tests/watchtower.test.ts](hiep-plugins/plugins/board/tests/watchtower.test.ts) (new)

Expected result:

- A subagent labeled `watchtower.task=TASK-043` gets `taskId: "TASK-043"`.
- A schedule iteration that started during the RUN.md row for TASK-042 gets `taskId: "TASK-042"`.
- A project without `watchtower/NEXT.md` has no plan entry.
- A blocked TASK lists its open Q-ID.

Anti-goal: snapshot build time on a host with 50 runs and 3 plan roots stays under 50 ms; drift gauge; read from a timing test in `tests/watchtower.test.ts` before the readers exist, after the mapping, and at completion.

## Verify

- `cd hiep-plugins/plugins/board && npx tsx --test tests/watchtower.test.ts` -> all cases pass.
- `cd hiep-plugins/plugins/board && npx tsx --test tests/snapshot.test.ts` -> old cases pass.
- `cd hiep-plugins/plugins/board && npm run typecheck` -> exit 0.
- The timing case in `tests/watchtower.test.ts` -> under 50 ms.
