# TASK-112 Client sync concurrency and flag

Group: D
Class: code

## Brief

Goal: Stop a reconnect from fetching every open tab at once, and ask for the latest prompt in the tail.

Change: at most 2 hidden catch-ups run at once; visible threads skip the cap; a parked thread starts when it becomes visible; gap recovery and manual retry skip the cap. Tail plans carry `includeLatestPrompt` only when the host has the feature and an anchor is pending.

Files:

- [packages/app/src/timeline/viewed-timeline-sync.ts](packages/app/src/timeline/viewed-timeline-sync.ts)
- [packages/app/src/timeline/viewed-timeline-sync.test.ts](packages/app/src/timeline/viewed-timeline-sync.test.ts)
- [packages/app/src/contexts/session-context.tsx](packages/app/src/contexts/session-context.tsx)
- [packages/app/src/timeline/timeline-sync-plan.ts](packages/app/src/timeline/timeline-sync-plan.ts)
- [packages/app/src/timeline/timeline-sync-plan.test.ts](packages/app/src/timeline/timeline-sync-plan.test.ts)

Expected result:

- With 1 visible and 10 hidden threads, the visible fetch starts first and no more than 2 hidden fetches run.

## Verify

- `npx vitest run src/timeline/viewed-timeline-sync.test.ts --bail=1` in `packages/app` -> pass.
- `npx vitest run src/timeline/timeline-sync-plan.test.ts --bail=1` -> pass.
