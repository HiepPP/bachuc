# TASK-114 Outcome

## Status

DONE. Not committed.

## Changed

- `agent-prefetch.ts`: `AGENT_PREFETCH_TTL_MS = 30_000`, `AGENT_PREFETCH_QUEUE_LIMIT = 3`. A failed prefetch does not start the 30 s wait. The feature is read with `hostSupportsFeature`.
- `host-navigation-model.ts`, `host-navigation.ts`: `prefetchAgent` with the default server id.

## Verified

Measured 2026-10-15.

- `agent-prefetch.test.ts` -> 10 passed. `host-navigation.test.ts` -> 12 passed. `host-navigation.test.tsx` -> 2 passed.
- Risk 7 is not a bug: a tail fetch marks the timeline synced, and `prepareCachedTimeline` stops on synced. A test with the real replica proves it.
- `npm run build:plugin`, typecheck, lint, format -> pass.

## Lessons

- The plugin `dist` declarations were stale; `npm run build:plugin` fixed the type errors.
- The desired check used `getAgentTimelineStatus(agentId) !== "pending"`, which is wrong for a desired agent still pending. TASK-115 replaced it.
