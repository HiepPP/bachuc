# TASK-120 Outcome

## Status

DONE. Not committed.

## Changed

- `agent-loading.ts`: `ensureAgentTimelineReadable`, with shared in-flight backfills per agent.
- `session.ts`: the five read handlers. The fetch returns a stored-agent payload and passes `includeLatestPrompt`.
- `bootstrap.ts`: opens the store, closes it after agents close.
- `websocket-server.ts`: `timelineLatestPromptTail: true`.
- `agent-manager.ts`: every cache call is guarded. An error is logged once per agent and the cache entry stays untrusted.
- `wire-compat.test.ts`: `fetchTimelineForRead` on the fake manager (outside the file list).

## Verified

Measured 2026-10-15.

- `agent-loading.test.ts` -> 6 passed; the lead re-ran it -> 6 passed.
- Durable timeline test -> 9 passed, with a store whose writes throw.
- `agent-manager.test.ts` -> 194 passed. `session.test.ts` -> 153 passed. `bootstrap-provider-availability.test.ts` -> 1 passed.
- Live, port 6768: `timeline-cache.sqlite` exists. Closed agent `847221a2` was read twice, the second time after another restart. 0 resume lines for both reads.

## Lessons

- `wire-compat.test.ts` failed because its fake agent had no `backgroundProcesses` field since commit `df71368e7`. The lead fixed it: 8 passed.
- The subagent list still rejects an archived parent. `clear_agent_attention` on an agent with no cache waits for its backfill.
