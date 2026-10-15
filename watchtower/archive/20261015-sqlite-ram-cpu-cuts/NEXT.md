# NEXT

## Current Active Plan

- Title: SQLite-backed RAM and CPU cuts
- Slug: 20261015-sqlite-ram-cpu-cuts
- Status: ARCHIVED
- Updated: 2026-10-15

## Tracker

One row per TASK. Group ties together items that write the same files.

| Order | TASK                                          | Group | Status | Spec                                                                                                                                   | Deps                                                                 | Context                                        | Notes                                         |
| ----- | --------------------------------------------- | ----- | ------ | -------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------- | ---------------------------------------------- | --------------------------------------------- |
| 1     | TASK-126 App replica budget from stored sizes | A     | DONE   | [watchtower/tasks/TASK-126-app-replica-budget-from-stored-sizes.md](watchtower/tasks/TASK-126-app-replica-budget-from-stored-sizes.md) | -                                                                    | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Drops the in-memory copy of all cache rows.   |
| 2     | TASK-127 App keeps 8 recent thread streams    | B     | DONE   | [watchtower/tasks/TASK-127-app-keeps-8-recent-thread-streams.md](watchtower/tasks/TASK-127-app-keeps-8-recent-thread-streams.md)       | -                                                                    | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Frees streams of threads left long ago.       |
| 3     | TASK-128 Daemon FTS5 search and prompt index  | D     | DONE   | [watchtower/tasks/TASK-128-daemon-fts5-search-and-prompt-index.md](watchtower/tasks/TASK-128-daemon-fts5-search-and-prompt-index.md)   | -                                                                    | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Moves full-timeline readers to SQL first.     |
| 4     | TASK-129 Daemon bounded tail with SQL pages   | D     | DONE   | [watchtower/tasks/TASK-129-daemon-bounded-tail-with-sql-pages.md](watchtower/tasks/TASK-129-daemon-bounded-tail-with-sql-pages.md)     | TASK-128                                                             | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Shares files with TASK-128; follows ADR-0032. |
| 5     | TASK-130 Daemon transcript fingerprint        | D     | DONE   | [watchtower/tasks/TASK-130-daemon-transcript-fingerprint.md](watchtower/tasks/TASK-130-daemon-transcript-fingerprint.md)               | TASK-129                                                             | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Shares files with TASK-128 and TASK-129.      |
| 6     | TASK-131 Coalesce agent record writes         | E     | DONE   | [watchtower/tasks/TASK-131-coalesce-agent-record-writes.md](watchtower/tasks/TASK-131-coalesce-agent-record-writes.md)                 | -                                                                    | -                                              | Records stay JSON.                            |
| 7     | TASK-132 Expire old agent request receipts    | F     | DONE   | [watchtower/tasks/TASK-132-expire-old-agent-request-receipts.md](watchtower/tasks/TASK-132-expire-old-agent-request-receipts.md)       | -                                                                    | -                                              | 7-day TTL.                                    |
| 8     | TASK-133 Docs for the SQLite cuts             | G     | DONE   | [watchtower/tasks/TASK-133-docs-for-the-sqlite-cuts.md](watchtower/tasks/TASK-133-docs-for-the-sqlite-cuts.md)                         | TASK-126, TASK-127, TASK-128, TASK-129, TASK-130, TASK-131, TASK-132 | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Runs last.                                    |

TASK Status labels: TODO, IN PROGRESS, BLOCKED, DONE.
Plan-level Status header: ACTIVE while any row is open, DONE when all rows DONE, ARCHIVED after archive.

## Plan Verify

- `npm run typecheck` -> exit 0.
- `npm run lint` on every changed file -> exit 0.
- Each changed test file, run alone with `npx vitest run <file> --bail=1` -> pass.
- `rg -n "better-sqlite3|drizzle-orm" package.json packages/*/package.json` -> no match.
- Live check after all groups: start `npm run dev` and `npm run dev:desktop`, open 10 threads in turn, search one thread, and send one prompt. Every thread renders, search returns hits, and `eventLoopDelay.maxMs` in `ws_runtime_metrics` of `.dev/paseo-home/daemon.log` stays at or below 100 ms.

## Handoff

- All 8 TASKs are DONE on `feat/sqlite-ram-cpu-cuts` (2026-10-15). Commits: A `e6758a7c8`, B `1c5c4d2d3`, D `1c4210462`, F `8d040740b`, E `3af0405c7`, G `3124a2191`. Nothing was pushed.
- Plan Verify passed: typecheck, lint, each changed test file, and the dependency grep. The live check ran on live 6768: 12 threads opened in turn, search returned 25 hits, 4 prompts ran on a mock thread, and `eventLoopDelay.maxMs` stayed at 39.9 ms or below.
- Follow-up: since TASK-129, some full-timeline callers see only the 1,000-row memory tail: MCP `get_agent_activity` without a limit, the session `timelineSize` counts, `import-sessions`, and plugin `publishAgentStream`. See the TASK-129 outcome.
- The follow-up shipped: `21a5b892a` moved `get_agent_activity` to the cache path, `bfd79b58f` restores a trimmed timeline when its cache fails, and `a58f5632c` stopped `appendTimelineItem` from waiting for the record write.
- Merged to `main` through PR #70 as `4fb0338b1` on 2026-10-15.

## Archive

- [watchtower/archive/20261015-instant-thread-open/](watchtower/archive/20261015-instant-thread-open/)
- [watchtower/archive/20261014-watchtower-plan-tree/](watchtower/archive/20261014-watchtower-plan-tree/)
- [watchtower/archive/20261014-native-glm-provider/](watchtower/archive/20261014-native-glm-provider/)
- Older plans are under [watchtower/archive/](watchtower/archive/).
- Archived: 2026-10-15 -> watchtower/archive/20261015-sqlite-ram-cpu-cuts/
