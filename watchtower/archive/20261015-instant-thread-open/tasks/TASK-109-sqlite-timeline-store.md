# TASK-109 SQLite timeline store

Group: B
Class: risky

## Brief

Goal: Keep committed timeline rows across runtime close and daemon restart, so a read needs no resume.

Change: a `DurableTimelineStore` on `node:sqlite` in `$PASEO_HOME/timeline-cache.sqlite`, plus `includeLatestPrompt` in `InMemoryAgentTimelineStore.fetch`.

Boundaries:

- The file is a rebuildable cache: a wrong `user_version` or a failed `quick_check` deletes and rebuilds it. No migration code. See [ADR-0031](watchtower/decisions/ADR-0031-node-sqlite-timeline-cache-over-json-and-better-sqlite3.md).
- `@types/node` has no `node:sqlite` typings: use a local interface and a dynamic import, as in [packages/server/src/services/quota-fetcher/providers/cursor.ts](packages/server/src/services/quota-fetcher/providers/cursor.ts).
- Keep the old `AgentTimelineStore` interface until TASK-110 switches over.

Files:

- [packages/server/src/server/agent/sqlite-timeline-store.ts](packages/server/src/server/agent/sqlite-timeline-store.ts) (new)
- [packages/server/src/server/agent/sqlite-timeline-store.test.ts](packages/server/src/server/agent/sqlite-timeline-store.test.ts) (new)
- [packages/server/src/server/agent/agent-timeline-store-types.ts](packages/server/src/server/agent/agent-timeline-store-types.ts)
- [packages/server/src/server/agent/agent-timeline-store.ts](packages/server/src/server/agent/agent-timeline-store.ts)
- [packages/server/src/server/agent/agent-timeline-store.test.ts](packages/server/src/server/agent/agent-timeline-store.test.ts)

Expected result:

- Rows, epoch, and the `complete` flag survive a reopen. A missing `node:sqlite` returns no store.

## Verify

- `npx vitest run src/server/agent/sqlite-timeline-store.test.ts --bail=1` in `packages/server` -> pass, with real temp-dir databases.
- `npx vitest run src/server/agent/agent-timeline-store.test.ts --bail=1` -> pass.
- `npm run typecheck` in `packages/server` -> exit 0.
