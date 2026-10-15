# TASK-131 Coalesce agent record writes

Group: E (writes `packages/server/src/server/agent/agent-storage.ts`; no shared file with other groups)
Class: risky

## Brief

Goal: A busy agent writes its JSON record at most a few times per second, not once per state change.

Change: every `emitState` and every `agent_state` event rewrites the whole pretty-printed file -> `AgentStorage` keeps the latest record in memory and writes it once per short window.

Boundaries:

- Records stay JSON files. Follows [ADR-0032](watchtower/decisions/ADR-0032-sqlite-served-old-pages-over-memory-only-serving.md): no database for agent records while agent listing stays fast in memory.
- Today `emitState` queues a persist on every emit ([packages/server/src/server/agent/agent-manager.ts](packages/server/src/server/agent/agent-manager.ts) line 5369), about 25 call sites pair it with `touchUpdatedAt`, and [packages/server/src/server/persistence-hooks.ts](packages/server/src/server/persistence-hooks.ts) lines 51-60 write on each `agent_state` event. Each write is a full atomic rewrite with no debounce ([packages/server/src/server/agent/agent-storage.ts](packages/server/src/server/agent/agent-storage.ts) line 187).
- Put the coalescing inside `AgentStorage`, so both write paths gain it and `agent-manager.ts` does not change.
- Window: 250 ms trailing per agent. Readers see the newest record at once from the storage cache.
- Never lose the last write: flush pending writes on daemon shutdown and when a caller awaits a flush. A delete cancels a pending write for that agent.
- Keep the atomic temp-file and rename write and the per-agent write order.
- Q-049 answer (owner, 2026-10-15): option A. The promise of `upsert` and `applySnapshot` settles when the file write commits and rejects on a write error. Accept up to 250 ms per awaited persist. Fix the fake-timer tests that await a persist before they advance the clock: `session.workspaces.test.ts` and the `send_agent_prompt` case in `mcp-server.test.ts`.

How:

- Add a per-agent pending record and a 250 ms timer in `AgentStorage`.
- Make the write method update the cache, mark the agent pending, and return a promise that settles when the file write commits.
- Add or extend a flush method and call it from the existing daemon shutdown path.
- Add tests: 10 updates in 100 ms give 1 file write with the last record, flush writes at once, delete cancels, and a read sees the newest record before the write.

Files:

- [packages/server/src/server/agent/agent-storage.ts](packages/server/src/server/agent/agent-storage.ts) (coalesced writes and flush)
- [packages/server/src/server/agent/agent-storage.test.ts](packages/server/src/server/agent/agent-storage.test.ts) (coalescing cases)
- [packages/server/src/server/bootstrap.ts](packages/server/src/server/bootstrap.ts) (call the flush on shutdown, only if no current shutdown path flushes storage)
- [packages/server/src/server/session.workspaces.test.ts](packages/server/src/server/session.workspaces.test.ts) (advance fake timers while a persist is pending; Q-049)
- [packages/server/src/server/agent/mcp-server.test.ts](packages/server/src/server/agent/mcp-server.test.ts) (same fix for the `send_agent_prompt` wait case; Q-049)

Expected result:

- 10 updates to one agent within 100 ms produce 1 file write that holds the last update.
- Shutdown writes every pending record before the process exits.
- `list` and `get` return the newest record before its file write.

Anti-goal: Data loss window stays at or below 250 ms of updates, and 0 updates are lost on a clean shutdown; tripwire; read from `agent-storage.test.ts` at baseline, after the coalescing change, and at completion.

## Verify

- `npx vitest run packages/server/src/server/agent/agent-storage.test.ts --bail=1` -> pass, including the 4 cases above.
- `npx vitest run packages/server/src/server/agent/agent-loading.test.ts --bail=1` -> pass.
- `npx vitest run packages/server/src/server/persistence-hooks.test.ts --bail=1` -> pass.
- `npm run typecheck` -> exit 0.
- Anti-goal: the shutdown test finds every pending record on disk -> within limit.
