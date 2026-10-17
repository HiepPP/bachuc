# TASK-119 AgentManager durable timeline

Group: C
Class: risky

## Brief

Goal: Load a closed agent's timeline from the cache with its stored epoch, and backfill from provider history when the cache is not trusted.

Change: `AgentManager` uses `DurableTimelineStore`. New `loadTimelineForRead`, `fetchTimelineForRead`, `backfillTimelineFromProvider`, `listProviderSubagentsForRead`, and a stored path for `clearAgentAttention`.

Boundaries:

- Do not edit `session.ts`, `agent-loading.ts`, or `bootstrap.ts`; TASK-120 owns them.
- Backfill uses purpose `history`, no launch context, at most 2 at once.
- Keep at most 64 non-live timelines in memory.

Files:

- [packages/server/src/server/agent/agent-manager.ts](packages/server/src/server/agent/agent-manager.ts)
- [packages/server/src/server/agent/agent-timeline-store-types.ts](packages/server/src/server/agent/agent-timeline-store-types.ts)
- [packages/server/src/server/agent/provider-subagents/store.ts](packages/server/src/server/agent/provider-subagents/store.ts)
- [packages/server/src/server/agent/provider-subagents/store.test.ts](packages/server/src/server/agent/provider-subagents/store.test.ts)
- [packages/server/src/server/agent/agent-manager.durable-timeline.test.ts](packages/server/src/server/agent/agent-manager.durable-timeline.test.ts) (new)
- [packages/server/src/server/agent/agent-manager.test.ts](packages/server/src/server/agent/agent-manager.test.ts)

Expected result:

- Close then restart returns the same epoch and rows with no `resumeSession` call. A crash backfills once with a new epoch.

## Verify

- `npx vitest run src/server/agent/agent-manager.durable-timeline.test.ts --bail=1` in `packages/server` -> pass.
- `npx vitest run src/server/agent/agent-manager.test.ts --bail=1` -> pass.
- `npx vitest run src/server/agent/agent-manager-stream-coalescing.test.ts --bail=1` -> pass.
- `npm run typecheck` -> exit 0.
