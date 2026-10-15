# TASK-127 Outcome

## Status

DONE. Every spec check passes, and the code is committed in `1c5c4d2d3`. The lead ran the live check after the owner allowed a live restart (Q-045).

## Changed

- `packages/app/src/stores/session-store.ts`: new action `evictAgentStream(serverId, agentId)`. It deletes the thread's `agentStreamTail`, `agentStreamHead`, `agentTimelineCursor`, `agentTimelineHasOlder`, `agentTimelineHasNewer`, `agentTimelineOlderFetchInFlight`, `agentHistorySyncGeneration`, and `agentAuthoritativeHistoryApplied` entries in one store update. Agent directory, tasks, submissions, unread, and attention state stay. A thread with nothing to drop leaves the store unchanged.
- `packages/app/src/timeline/viewed-timeline-sync.ts`:
  - `RECENT_STREAM_LIMIT = 8` (exported).
  - `ViewedTimelineSyncPorts` has two optional ports, `listStreamAgentIds()` and `evictStream(agentId): boolean`. The owner supplies them, so `ViewedTimelineOwnerPorts` omits them and `session-context.tsx` needs no change.
  - `createViewedTimelineSync` keeps a most-recently-viewed order per host. A thread moves to the front when it enters the visible set. When a thread leaves the visible set, the sync counts threads that hold items plus visible threads. Above 8, it evicts the oldest hidden threads. It skips a visible thread, a thread with a cache load in flight, and a thread with a running catch-up.
  - An evicted thread skips catch-up (reconnect, gap recovery, hidden catch-ups) until it is visible again. When it is visible again, it reloads the replica cache, then runs the ordinary resume fetch. Its status shows `pending` meanwhile.
  - `createViewedTimelineOwner` wires the ports to the store. It refuses to evict the focused thread or a thread with an active message submission. It flushes queued events before the eviction, so the replica cache holds the newest rows. It drops live events of an evicted thread until the thread prepares again or an accepted timeline page lands.
- `packages/app/src/stores/session-store.test.ts`: 2 eviction action cases.
- `packages/app/src/timeline/viewed-timeline-sync.test.ts`: `TimelineWorld` has a fake held-stream set and the eviction ports. 5 new cases: cap of 8 over 12 threads, split pane never evicted, two hosts, evicted hidden thread waits for its next view, and an owner test with the real store for the reopen path.

## Contract

- Per host, at most 8 threads hold stream items after a thread leaves the visible set. A visible thread counts toward the 8 even before its items land.
- Never evicted: a visible thread (any pane or tab source), the focused thread, a thread with an active submission, a thread with a cache load or catch-up in flight.
- An evicted thread reopens through cache paint, then one `after endSeq` fetch from the cached cursor (or one `tail` fetch with no cache entry). Live events apply in order after that.
- No daemon protocol change. Subscriptions do not change: an evicted open-tab thread stays subscribed, and the owner drops its live events.

## Verified

- `npx vitest run src/timeline/viewed-timeline-sync.test.ts --bail=1` (from `packages/app`) -> 49 passed, including the 4 spec cases.
- `npx vitest run src/stores/session-store.test.ts --bail=1` -> 24 passed.
- Neighbor files: `src/stores/workspace-layout-store.test.ts` -> 134 passed; `src/timeline/replica.test.ts` -> 16 passed.
- Mutation check: with the owner's live-event drop disabled, the reopen test fails (`expected [ Array(2) ] to deeply equal []`). Reverted.
- `npm run typecheck` -> exit 0 (2026-10-15T04:03Z). An earlier run failed only in `packages/server/src/server/agent/agent-manager.ts` (another group's file), and the re-run passed.
- `rtk proxy npm run lint -- <4 changed files>` -> 0 warnings, 0 errors.
- `npm run format:check:files -- <4 changed files>` -> all files use the correct format.
- Live check, lead run (2026-10-15 12:02 +07): the live web app on `localhost:8081` (Metro of `npm run dev:desktop`) against live 6768 on `3af0405c7`, in a headless browser. The lead opened thread `faa8f035`, then 11 other threads in turn through in-app navigation, then reopened the first. A 50 ms probe found the last row visible from the first sample (271 ms) and the list pinned to the bottom while it grew (2502 to 2836 px). Screenshots at 250 ms and 5 s match. No blank and no jump -> PASS. Limit: the Electron window was not driven; it renders the same web bundle.

## Anti-goal

Tripwire: reopening an evicted thread issues at most 1 tail fetch before it renders, with no duplicate or missing rows. Source: `viewed-timeline-sync.test.ts`.

| Checkpoint            | Value                                                                                         | Time (UTC)           |
| --------------------- | --------------------------------------------------------------------------------------------- | -------------------- |
| baseline              | 44 passed; no reopen test existed, so fetch count not measurable                              | 2026-10-15T03:54:32Z |
| after eviction change | 49 passed; reopen test: 1 fetch (`after` seq 3), rows 1-5 then 6, no duplicate or missing row | 2026-10-15T03:59:43Z |
| completion            | 49 passed; same reopen result after the lint fixes                                            | 2026-10-15T04:03Z    |

Final: PASS, within limit. The tripwire was not crossed.

## Lessons

- An evicted open-tab thread stays subscribed on the daemon. The owner drops its live events, and the reopen `after` fetch covers them. Do not apply live events to a thread with no base.
- Prefetch skips desired threads (`isAgentDesired`), so hovering a Board card of an evicted open-tab thread does not warm it. The reopen still paints from the replica cache.
- `docs/timeline-sync.md` says "There is no recent-agent limit". That sentence is about subscriptions and stays true. The docs do not describe the 8-thread stream cap yet; the spec did not list a doc file.
- In agent shells, `npm run lint` can print "ESLint output (JSON parse failed)" through the rtk wrapper. Use `rtk proxy npm run lint` for the real exit code.
