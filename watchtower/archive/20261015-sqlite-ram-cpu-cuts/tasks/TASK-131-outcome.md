# TASK-131 Outcome

## Status

DONE. The coalescing keeps the contract the owner chose in Q-049 (option A). The 2 fake-timer tests that hung now advance or flush the clock, with their assertions unchanged.

## Changed

- `packages/server/src/server/agent/agent-storage.ts`: `upsert` and `applySnapshot` put the record in the cache at once and schedule one write per agent. The write starts 250 ms after the first unsaved update (`AGENT_RECORD_WRITE_WINDOW_MS`, a fixed window that later updates do not reset). The returned promise settles when the file write commits. `flush()` starts every scheduled write at once and waits for all writes in flight. `beginDelete` and `remove` cancel a scheduled write. The atomic temp-file write and the per-agent write chain stay.
- `packages/server/src/server/agent/agent-storage.test.ts`: 6 coalescing cases. The test wraps the real `writeJsonFileAtomic` with `vi.fn` to count file writes.
- `packages/server/src/server/bootstrap.ts`: not changed. The shutdown path already calls `agentStorage.flush()` after `agentManager.flushForShutdown()` (line 1810).
- `packages/server/src/server/session.workspaces.test.ts`: new local helper `settleWithFakeTimers` advances the fake clock in 250 ms steps until a promise settles. "create_agent_request does not title an existing workspace from the agent prompt" wraps `session.handleMessage` in it. One advance is not enough, because real file I/O can schedule the write timer after the advance ends.
- `packages/server/src/server/agent/mcp-server.test.ts`: "notifies the caller when a blocking agent-scoped prompt outlasts the wait" hung in its `finally`, not in the body. A write timer made on the fake clock never fires after `vi.useRealTimers()`, so `agentManager.flush()` waited forever. The `finally` now calls `storage.flush()` after `vi.useRealTimers()`, which starts that write at once.

## Contract

- Readers (`get`, `list`, and the other queries) see the newest record before its file write.
- A write of an agent starts at most 250 ms after its first unsaved update. One write holds every update made in that window.
- The promise of `upsert` and `applySnapshot` settles when a file write that holds the record commits. If that write fails, the promise rejects. Each awaited persist can take up to 250 ms (Q-049, option A).
- `flush()` returns only after every scheduled and in-flight write settles.
- A delete cancels the scheduled write and resolves its promise. Updates after `beginDelete` change nothing.
- `setTitle` no longer waits for the pending write, because the cache holds the newest record.

## Verified

- `npx vitest run packages/server/src/server/agent/agent-storage.test.ts --bail=1` -> PASS 24/24 (2026-10-15T04:52Z).
- `npx vitest run packages/server/src/server/agent/agent-loading.test.ts --bail=1` -> PASS 6/6.
- `npx vitest run packages/server/src/server/persistence-hooks.test.ts --bail=1` -> PASS 7/7.
- `npx vitest run packages/server/src/server/session.workspaces.test.ts --bail=1` -> PASS 108/108, 4 skipped.
- `npx vitest run packages/server/src/server/agent/mcp-server.test.ts` -> 114 pass, 7 fail. The 7 are the Known Failures in `watchtower/AUTORUN.md`: each throws `Cannot read properties of undefined (reading 'size')` at `agent-projections.ts:149`. The send_agent_prompt wait case passes, also 3 times in a row alone.
- `npx vitest run packages/server/src/server/agent/agent-manager.test.ts --bail=1` -> PASS 194/194.
- Fake-timer scan (`rg useFakeTimers` in `packages/server`, files that use agent storage): `agent-manager-stream-coalescing.test.ts` PASS 26/26, `agent-prompt.test.ts` PASS 28/28, `providers/mock-load-test-agent.test.ts` PASS 15/15. `websocket-server.relay-reconnect.test.ts` uses only a storage stub. No other test hangs.
- `npm run typecheck` -> exit 0.
- `rtk proxy npm run lint -- <4 changed files>` -> 0 warnings, 0 errors. `npm run format:files -- <4 changed files>` -> exit 0.

## Anti-goal

Data loss window <= 250 ms of updates and 0 updates lost on a clean shutdown. Tripwire not crossed.

| Checkpoint       | Value                                                                                                                              | Source                  | Time (UTC)        |
| ---------------- | ---------------------------------------------------------------------------------------------------------------------------------- | ----------------------- | ----------------- |
| baseline         | no window; every update writes at once; 18/18 pass                                                                                 | `agent-storage.test.ts` | 2026-10-15T03:52Z |
| after coalescing | write starts 250 ms after the first update, even with updates every 50 ms; shutdown flush leaves 3/3 records with their last value | `agent-storage.test.ts` | 2026-10-15T03:53Z |
| completion       | same, 24/24 pass                                                                                                                   | `agent-storage.test.ts` | 2026-10-15T03:57Z |
| test fixes       | same, 24/24 pass after the Q-049 test fixes                                                                                        | `agent-storage.test.ts` | 2026-10-15T04:52Z |

## Lessons

- A fake-timer test that awaits an agent persist must advance the clock until the persist settles. One `advanceTimersByTimeAsync` can end before real file I/O schedules the write timer.
- A write timer made on the fake clock is dropped by `vi.useRealTimers()`. Call `storage.flush()` after `vi.useRealTimers()` and before `agentManager.flush()`.
- The 7 `backgroundProcesses` failures in `mcp-server.test.ts` exist without this TASK.
