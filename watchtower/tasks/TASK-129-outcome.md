# TASK-129 Outcome

## Status

DONE. Every spec check passes, and the code is committed in `1c4210462`. The lead ran the live anti-goal check after the owner allowed a live restart (Q-046).

## Changed

- `packages/server/src/server/agent/agent-timeline-store.ts`: `TIMELINE_TAIL_ROWS = 1_000`; constructor option `tailRows`. `trimFlushed(agentId, flushedThroughSeq)` drops items whose source rows all sit below `min(flushedThroughSeq + 1, nextSeq - tailRows)`; an item with a newer row stays whole. `fetch(agentId, options, history?)` takes a `TimelineHistoryReader`. Memory serves a page when every item between the selected ones is in memory; otherwise a window of cached rows joins the memory items. The window widens so no assistant or reasoning run is cut, pulls every row of each tool call or plugin item in it (identity rows), and skips rows that memory items already hold. Backward windows start at 256 rows and grow 4x. `includeLatestPrompt` reads the latest prompt seq from the cache when memory has no prompt (up to 10,000 rows back). A trimmed timeline without a reader serves pages within the tail and throws for older ones.
- `packages/server/src/server/agent/sqlite-timeline-store.ts`: `SCHEMA_VERSION` 3; `timeline_rows.identity` with a partial index; `readRowWindow` (paged by 2,000), `readIdentityRows` (`json_each` list), and `setFlushListener`, called after each committed flush with the agent's max seq.
- `packages/server/src/server/agent/agent-timeline-store-types.ts`: `TimelineFlushListener`; optional `readRowWindow?`, `readIdentityRows?`, `setFlushListener?`.
- `packages/server/src/server/agent/agent-manager.ts`: option `timelineTailRows`; the flush listener trims a timeline only while the cache serves it (same epoch, no missed write) and can read windows. `fetchTimeline` and `fetchTimelineForRead` pass a lazy history reader. A restored complete entry trims at once. `rewind` finds an old prompt through `getTimelinePromptRows`.
- Tests: `agent-timeline-store.test.ts` (5 tail cases), `agent-manager.durable-timeline.test.ts` (3 cases), `sqlite-timeline-store.test.ts` (window and listener case).

## Contract

- `TimelineHistoryReader { readRows(fromSeq, toSeq); readIdentityRows(identities, beforeSeq); latestPromptSeq() }`.
- `DurableTimelineStore.readRowWindow?(agentId, fromSeq, toSeq): AgentTimelineRow[] | null`, `readIdentityRows?(agentId, identities, beforeSeq): AgentTimelineRow[] | null`, `setFlushListener?(listener)`.
- The fetch result shape and paging rules are unchanged; `window.minSeq` still reports the whole timeline.

## Verified

- `npx vitest run packages/server/src/server/agent/agent-timeline-store.test.ts --bail=1` -> PASS 18/18: trim after flush, no trim before flush, `before` paging from tail to start returns every item once and equal to a full-memory timeline (about 600 rows, tail 50, page 7), `after` paging reaches the same items, `includeLatestPrompt` across the edge equals the full-memory page, and the no-reader cases.
- `npx vitest run packages/server/src/server/agent/agent-manager.durable-timeline.test.ts --bail=1` -> PASS 19/19, including a live agent with 5,000 rows holding 1,000 after a flush, paging across the edge with whole items, a prompt below the edge, and no trim without a cache.
- `npx vitest run packages/server/src/server/agent/projected-timeline-contract.test.ts --bail=1` -> PASS 3/3.
- `npx vitest run packages/server/src/server/agent/sqlite-timeline-store.test.ts --bail=1` -> PASS 16/16.
- Extra: `agent-manager.test.ts` PASS 194/194; `agent-manager-stream-coalescing.test.ts` PASS 26/26; `agent-loading.test.ts` PASS 6/6.
- Group-level run, once after TASK-130: `npm run typecheck` -> exit 0 (2026-10-15T04:29Z). Lint and format check of the 13 group files -> clean.
- Impact: `InMemoryAgentTimelineStore.getRows` LOW (callers: `getTimelineRows`, `rewind`); `getItems` HIGH (callers: `getTimeline`, `getLastAssistantMessage`, `getMetricsSnapshot`, `dispatchStream`).

## Anti-goal

`eventLoopDelay.maxMs` in `ws_runtime_metrics` stays at or below 100 ms while paging a 5,000-row thread to its start on live.

| Checkpoint              | Value                            | Source                                                                                                                                                                                                                   | Time                    |
| ----------------------- | -------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------- |
| Before the change       | maxMs 46                         | `.dev/paseo-home/daemon.log`, last `ws_runtime_metrics`; live worker 5344 started 03:52 local on old code                                                                                                                | 2026-10-15 11:18:03 +07 |
| After the SQL page path | maxMs 19.1, 21.5 (before paging) | live worker 65079 on `3af0405c7`, restarted 11:55 +07, metrics windows at 12:03:24 and 12:03:54 +07                                                                                                                      | 2026-10-15 12:03 +07    |
| Completion              | maxMs 29.4, 39.9, 37.3           | live worker 65079 on `3af0405c7`; 21 runs of `fetch_agent_timeline` from tail to seq 1 on a 3,701-row mock thread, 88 items each, no duplicates, slowest page 36 ms; the 3 metrics windows from 12:04:24 to 12:05:24 +07 | 2026-10-15 12:05 +07    |

## Lessons

- Full-timeline callers that now see only the tail (listed, not moved): `getTimeline` feeds MCP `get_agent_activity` with no limit, the session `timelineSize` counts, and `import-sessions`; `getLastAssistantMessage` and plugin `publishAgentStream` read the tail. A fix needs async SQL reads in `paseo-tools.ts`, `mcp-shared.ts`, and `session.ts`.
- A tool call dropped from memory that gets an update more than 1,000 rows later shows as a new item in memory-only pages; pages built from the cache merge it at its old place.
- If the cache stops serving a trimmed timeline, pages older than the tail throw until the timeline reloads.
- `timelineTailRows` on `AgentManagerOptions` lets tests use a small tail.
