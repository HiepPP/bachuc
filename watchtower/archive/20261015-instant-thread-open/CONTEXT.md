# Context

## Goal

A click on a board run card opens the thread at once, at its latest prompt, with no long load and no visible jump.

## Evidence

- Release `daemon.log`, 2026-10-15: 855 `fetch_agent_timeline_request` calls over 500 ms, p50 10.3 s, p90 145 s. A live agent answers in 0-1 ms.
- 12 bursts of about 160 "Agent resumed from persistence" lines in seconds, at client reconnects.
- Steady state holds about 2 agents in memory, because idle-runtime-closer closes runtimes after 30 minutes.
- `closeAgentRuntime` keeps the in-memory timeline and epoch. The epoch changed only on daemon restart, archive, reload with rehydrate, and rewind. The read cost was the resume itself.
- Besides the timeline fetch, `clear_agent_attention` and `agent.provider_subagents.list.request` also resumed the agent on every open.
- The client chain was: observe ack and replica read, then a 40-row tail, then sometimes a prompt list and a window fetch, then a bottom pin, a prompt scroll, and up to 24 frames of re-measure.

## Design

- Phase 1, daemon: [packages/server/src/server/agent/sqlite-timeline-store.ts](packages/server/src/server/agent/sqlite-timeline-store.ts) mirrors source rows into `$PASEO_HOME/timeline-cache.sqlite`. Memory stays the serving layer; loading re-runs the projection. One stored epoch per agent. A `complete` flag marks a trusted entry; anything else backfills from provider history with purpose `history`, at most 2 at once. At most 64 cold timelines stay in memory. `ensureAgentTimelineReadable` in [packages/server/src/server/agent/agent-loading.ts](packages/server/src/server/agent/agent-loading.ts) serves the five read handlers. `ensureAgentLoaded` is unchanged (GitNexus CRITICAL).
- Phase 2, client sync: at most 2 hidden catch-ups run at once; the visible thread goes first.
- Phase 3, one round trip and prefetch: `includeLatestPrompt` on `fetch_agent_timeline_request`, gated on `server_info.features.timelineLatestPromptTail`, widens the tail to the latest user message, cap 200 items. Plugins call `navigation.prefetchAgent`; the board calls it after 120 ms of hover and on press-in.
- Phase 4, no jump: the latest-prompt anchor stays pending until it lands, with a 30 s cap. No bottom pin while it waits, and the transcript stays hidden until the jump settles.

## Decisions

- `node:sqlite` timeline cache: [ADR-0031](watchtower/decisions/ADR-0031-node-sqlite-timeline-cache-over-json-and-better-sqlite3.md).
- Bounded hidden catch-up over lazy load on reveal, because hidden chats keep their replica current.
- Hover intent and press-in prefetch over visibility prefetch, because a board shows many cards.
- No cache reset inside `primeTimelineFromLegacyProviderHistory`, because memory and cache already share one epoch and a reset there drops rows.
- Codex keeps backfill: purpose `history` sends only `initialize` and `thread/read`, so it takes no writer lock. Decided from code.
