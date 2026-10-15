# TASK-119 Outcome

## Status

DONE. Not committed.

## Changed

- `agent-manager.ts`: the read methods above. `TimelineReadSource` is `"memory" | "durable" | "missing"`. Cold-timeline cap 64 and backfill limit 2.
- The old `AgentTimelineStore` interface is deleted.
- `ProviderSubagentStore.restore(parentAgentId, descriptors)`.
- One test was rewritten as "a failed history replay commits no partial rows".

## Contract

- A non-live memory timeline is readable only when its cache entry is complete.
- Backfill builds all rows first, then loads memory in one step.
- Provider subagents are written to the cache only on close and after a backfill.
- Clearing attention on a stored agent keeps its `updatedAt`.

## Verified

Measured 2026-10-15.

- Durable timeline test -> 8 passed; the lead re-ran it -> 8 passed.
- `agent-manager.test.ts` -> 194 passed. Stream coalescing -> 26 passed. Subagent store -> 4 passed.
- Typecheck, lint, format -> pass.

## Lessons

- The design asked for a cache reset inside `primeTimelineFromLegacyProviderHistory`. That reset would drop rows memory still holds, so it was left out.
- Synchronous `reset` and `deleteAgent` were left unguarded; TASK-120 guarded them.
- Codex backfill was decided from code only.
