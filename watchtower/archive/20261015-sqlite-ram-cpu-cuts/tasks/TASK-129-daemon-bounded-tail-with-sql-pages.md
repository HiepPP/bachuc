# TASK-129 Daemon bounded tail with SQL pages

Group: D (writes `packages/server/src/server/agent/agent-timeline-store.ts`, `agent-manager.ts`, and `sqlite-timeline-store.ts`; shared with TASK-128 and TASK-130)
Class: risky

## Brief

Goal: Each timeline in daemon memory holds only a bounded tail, and older pages come from the timeline cache.

Change: a live agent keeps every row in memory -> memory keeps the newest 1,000 source rows, and `before` pages past the tail read from SQL.

Boundaries:

- Follows [ADR-0032](watchtower/decisions/ADR-0032-sqlite-served-old-pages-over-memory-only-serving.md): memory serves a bounded tail, live or not, while `node:sqlite` runs in the daemon runtime.
- Today `InMemoryAgentTimelineStore` keeps the whole projection per agent ([packages/server/src/server/agent/agent-timeline-store.ts](packages/server/src/server/agent/agent-timeline-store.ts) line 51). Only non-live timelines are capped, at 64 ([packages/server/src/server/agent/agent-manager.ts](packages/server/src/server/agent/agent-manager.ts) line 108).
- Trim only rows already flushed to SQLite in the current epoch. A buffered row stays in memory until its flush commits.
- Rows written by this daemon in the current epoch are trusted for reads, even while the agent is live. The `complete` flag still guards reads across restarts.
- A projected item can span the tail edge, for example merged assistant chunks or one tool call lifecycle. Read enough SQL rows before the edge so the page has whole items, and keep `seqStart`, `seqEnd`, and `sourceSeqRanges` correct.
- Keep the fetch result shape and paging rules from [docs/timeline-sync.md](docs/timeline-sync.md), including `includeLatestPrompt` up to 200 projected items.
- When the store is missing or fails, keep full memory timelines, as today.
- Before the change, run GitNexus impact on `InMemoryAgentTimelineStore.getRows` and `getItems`. Every caller that needs more than the tail must read through SQL or be listed as a blocker.

How:

- Add a tail limit to the in-memory store and drop older flushed rows after each flush.
- Add a store method that reads a seq window of rows for one agent, paged.
- In `fetch`, serve `before` pages past the tail edge from SQL, with whole projected items.
- Move or verify each full-timeline caller found by the impact check.
- Add tests: trim after flush, no trim before flush, a page across the edge, `includeLatestPrompt` across the edge, and the no-store fallback.

Files:

- [packages/server/src/server/agent/agent-timeline-store.ts](packages/server/src/server/agent/agent-timeline-store.ts) (tail limit and trim)
- [packages/server/src/server/agent/sqlite-timeline-store.ts](packages/server/src/server/agent/sqlite-timeline-store.ts) (seq window read)
- [packages/server/src/server/agent/agent-timeline-store-types.ts](packages/server/src/server/agent/agent-timeline-store-types.ts) (window read method)
- [packages/server/src/server/agent/agent-manager.ts](packages/server/src/server/agent/agent-manager.ts) (wire trim to flush, serve old pages)
- [packages/server/src/server/agent/agent-timeline-store.test.ts](packages/server/src/server/agent/agent-timeline-store.test.ts) (tail cases)
- [packages/server/src/server/agent/agent-manager.durable-timeline.test.ts](packages/server/src/server/agent/agent-manager.durable-timeline.test.ts) (edge and fallback cases)

Expected result:

- A live agent with 5,000 source rows holds at most 1,000 source rows in memory after a flush.
- Paging `before` from the tail to the first row returns every projected item once, in order, with the same content as a full-memory timeline.
- `includeLatestPrompt` finds a prompt that sits across the tail edge.
- With no store, all timelines stay fully in memory and every existing test passes.

Anti-goal: `eventLoopDelay.maxMs` in `ws_runtime_metrics` stays at or below 100 ms while paging a 5,000-row thread to its start on live; tripwire; read from `.dev/paseo-home/daemon.log` before the change, after the SQL page path lands, and at completion.

## Verify

- `npx vitest run packages/server/src/server/agent/agent-timeline-store.test.ts --bail=1` -> pass, including the tail cases.
- `npx vitest run packages/server/src/server/agent/agent-manager.durable-timeline.test.ts --bail=1` -> pass, including the edge and fallback cases.
- `npx vitest run packages/server/src/server/agent/projected-timeline-contract.test.ts --bail=1` -> pass.
- `npx vitest run packages/server/src/server/agent/sqlite-timeline-store.test.ts --bail=1` -> pass.
- `npm run typecheck` -> exit 0.
- Anti-goal, live: restart `npm run dev`, scroll one long thread to its start, then read the next `ws_runtime_metrics` lines in `.dev/paseo-home/daemon.log` -> `eventLoopDelay.maxMs` at or below 100.
