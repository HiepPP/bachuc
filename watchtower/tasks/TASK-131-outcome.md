# TASK-131 Outcome

## Status

BLOCKED. The coalescing works and every spec check passes. Two fake-timer tests outside `Files:` now time out, and each awaited persist waits up to 250 ms. The lead must choose the fix (see Blocker).

Blocked: Q-043 asks the owner to choose the persist contract. The code is parked in stash `watchtower E BLOCKED`.

## Changed

- `packages/server/src/server/agent/agent-storage.ts`: `upsert` and `applySnapshot` put the record in the cache at once and schedule one write per agent. The write starts 250 ms after the first unsaved update (`AGENT_RECORD_WRITE_WINDOW_MS`, a fixed window that later updates do not reset). The returned promise settles when the file write commits. `flush()` starts every scheduled write at once and waits for all writes in flight. `beginDelete` and `remove` cancel a scheduled write. The atomic temp-file write and the per-agent write chain stay.
- `packages/server/src/server/agent/agent-storage.test.ts`: 6 coalescing cases. The test wraps the real `writeJsonFileAtomic` with `vi.fn` to count file writes.
- `packages/server/src/server/bootstrap.ts`: not changed. The shutdown path already calls `agentStorage.flush()` after `agentManager.flushForShutdown()` (line 1810).

## Contract

- Readers (`get`, `list`, and the other queries) see the newest record before its file write.
- A write of an agent starts at most 250 ms after its first unsaved update. One write holds every update made in that window.
- The promise of `upsert` and `applySnapshot` settles when a file write that holds the record commits. If that write fails, the promise rejects.
- `flush()` returns only after every scheduled and in-flight write settles.
- A delete cancels the scheduled write and resolves its promise. Updates after `beginDelete` change nothing.
- `setTitle` no longer waits for the pending write, because the cache holds the newest record.

## Verified

- `npx vitest run packages/server/src/server/agent/agent-storage.test.ts --bail=1` -> PASS 24/24 (2026-10-15T03:57Z, after format).
- `npx vitest run packages/server/src/server/agent/agent-loading.test.ts --bail=1` -> PASS 6/6.
- `npx vitest run packages/server/src/server/persistence-hooks.test.ts --bail=1` -> PASS 7/7.
- `npm run typecheck` -> exit 0.
- `rtk proxy npm run lint -- <2 changed files>` -> 0 errors. `npm run format:check:files -- <2 changed files>` -> exit 0.
- Extra caller tests: `agent-manager.test.ts` -> PASS 194/194. `schedule/service.test.ts` -> PASS 60/60.
- `session.workspaces.test.ts` -> FAIL 1: "create_agent_request does not title an existing workspace from the agent prompt" times out (5000 ms).
- `mcp-server.test.ts` -> FAIL 8. One is caused by this TASK: "notifies the caller when a blocking agent-scoped prompt outlasts the wait" times out. The other 7 throw `Cannot read properties of undefined (reading 'size')` in `agent-projections.ts:149` (`backgroundProcesses`), which this TASK does not touch. Their fixtures lack `backgroundProcesses`, so they are not from this change.

## Anti-goal

Data loss window <= 250 ms of updates and 0 updates lost on a clean shutdown. Tripwire not crossed.

| Checkpoint       | Value                                                                                                                              | Source                  | Time (UTC)        |
| ---------------- | ---------------------------------------------------------------------------------------------------------------------------------- | ----------------------- | ----------------- |
| baseline         | no window; every update writes at once; 18/18 pass                                                                                 | `agent-storage.test.ts` | 2026-10-15T03:52Z |
| after coalescing | write starts 250 ms after the first update, even with updates every 50 ms; shutdown flush leaves 3/3 records with their last value | `agent-storage.test.ts` | 2026-10-15T03:53Z |
| completion       | same, 24/24 pass                                                                                                                   | `agent-storage.test.ts` | 2026-10-15T03:57Z |

## Blocker

Awaited persists now wait for the 250 ms timer, as the spec asks ("return a promise that settles when the file write commits"). Two effects follow:

1. Tests that turn on fake timers and then await a persist hang, because nothing advances the clock: `session.workspaces.test.ts` (awaits `session.handleMessage(create_agent_request)` before `vi.runAllTimersAsync()`) and `mcp-server.test.ts` (send_agent_prompt with `vi.useFakeTimers()`). Both files are outside `Files:`.
2. Real latency: `agent-manager.ts` awaits about 23 persists. For example, `archiveAgentUnlocked` awaits `applySnapshot` and then `markRecordArchived`, so archive can take up to 500 ms longer.

Fix options for the lead:

- A (keeps the spec contract): let the two tests advance fake timers while the persist is pending, for example `const pending = session.handleMessage(...); await vi.runAllTimersAsync(); await pending;`. Accept up to 250 ms per awaited persist.
- B (changes the contract): resolve `upsert` and `applySnapshot` once the cache holds the record, and keep durability through the timer and `flush()`. This removes the latency and the hangs. Callers lose write errors, and tests that reload a new `AgentStorage` from disk right after an await must call `flush()` first.

## Lessons

- A fake-timer test that awaits an agent persist must advance the clock now, because the record write waits for a 250 ms `setTimeout`.
- The 7 `backgroundProcesses` failures in `mcp-server.test.ts` exist without this TASK.
