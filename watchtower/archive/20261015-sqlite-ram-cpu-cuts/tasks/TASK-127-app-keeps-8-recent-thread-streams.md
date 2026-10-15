# TASK-127 App keeps 8 recent thread streams

Group: B (writes `packages/app/src/stores/session-store.ts` and `packages/app/src/timeline/viewed-timeline-sync.ts`; no shared file with other groups)
Class: risky

## Brief

Goal: The app keeps stream items only for the 8 most recently viewed threads per host, so memory stops growing as the owner opens more threads.

Change: `agentStreamTail` and `agentStreamHead` keep items for every thread ever opened -> threads outside the 8 most recently viewed lose their items and reload on the next view.

Boundaries:

- Today `agentStreamTail` and `agentStreamHead` are `Map<string, StreamItem[]>` per server with no eviction ([packages/app/src/stores/session-store.ts](packages/app/src/stores/session-store.ts) lines 393-394). Only `clearAgentStreamHead` exists (line 1209).
- The cap is 8 per host, chosen by the owner on 2026-10-15 ([watchtower/CONTEXT.md](watchtower/CONTEXT.md)).
- Never evict a thread that is viewed now: focused, visible in any pane or tab, or inside the viewed set of [packages/app/src/timeline/viewed-timeline-sync.ts](packages/app/src/timeline/viewed-timeline-sync.ts).
- An evicted thread must reopen through the normal path: replica cache paint, then the tail fetch. Its sequence cursor must reset with its items, so a live event for it cannot apply on top of a missing base.
- Keep agent directory data, unread state, and attention state. Evict only stream items and their per-thread cursors.
- Do not change the daemon protocol.
- Read [docs/agent-stream-performance.md](docs/agent-stream-performance.md) and [docs/timeline-sync.md](docs/timeline-sync.md) before the change.

How:

- Track a most-recently-viewed order per host, updated when a thread enters the viewed set.
- When a thread leaves the viewed set and more than 8 threads hold items, drop items and cursors of the oldest threads that are not viewed now.
- Make sure a dropped thread that becomes viewed again fetches its tail and renders without a blank or a jump.
- Add tests for the cap, the never-evict-viewed rule, and reopen after eviction.

Files:

- [packages/app/src/stores/session-store.ts](packages/app/src/stores/session-store.ts) (eviction action that clears tail, head, and cursors of one thread)
- [packages/app/src/timeline/viewed-timeline-sync.ts](packages/app/src/timeline/viewed-timeline-sync.ts) (recent order and eviction trigger)
- [packages/app/src/stores/session-store.test.ts](packages/app/src/stores/session-store.test.ts) (eviction action cases)
- [packages/app/src/timeline/viewed-timeline-sync.test.ts](packages/app/src/timeline/viewed-timeline-sync.test.ts) (cap, viewed rule, reopen)

Expected result:

- Case: open 12 threads in turn on one host -> 8 threads hold stream items; the 4 oldest hold none.
- Case: a thread visible in a split pane is never evicted, even when it is the oldest.
- Case: reopen an evicted thread -> it paints from the replica cache, fetches its tail, and then applies live events in order.
- Case: two hosts -> each host has its own cap of 8.

Anti-goal: Reopening an evicted thread issues at most 1 tail fetch before it renders, and no duplicate or missing rows; tripwire; read from `viewed-timeline-sync.test.ts` at baseline, after the eviction change, and at completion.

## Verify

- `npx vitest run packages/app/src/timeline/viewed-timeline-sync.test.ts --bail=1` -> pass, including the 4 cases above.
- `npx vitest run packages/app/src/stores/session-store.test.ts --bail=1` -> pass.
- `npm run typecheck` -> exit 0.
- Live check, human: on `npm run dev:desktop`, open 10 threads in turn, then reopen the first one -> it renders its latest rows with no blank and no jump.
- Anti-goal: the reopen test counts 1 tail fetch and matching rows -> within limit.
