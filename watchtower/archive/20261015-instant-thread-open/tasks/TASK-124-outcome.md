# TASK-124 Outcome

## Status

DONE. Not committed. The live-app line of Verify is UNVERIFIED.

## Changed

- `prefetch.ts`: `createPrefetchIntent()` with `PREFETCH_INTENT_MS = 120`.
- `page.tsx`: `onPrefetch` passes from the board to each column, cluster, and card. It is omitted when the host has no `prefetchAgent`.
- `latest-prompt-anchor.ts`: `cancelLatestPromptAnchor()`, called from `host-navigation.ts` on a bottom open.
- `viewed-timeline-sync.ts`: `isAgentDesired(agentId)`; `agent-prefetch.ts` uses it through `isViewedAgentDesired`.

## Verified

Measured 2026-10-15.

- Five app test files -> 11, 44, 13, 3, and 8 passed.
- Board plugin typecheck, lint, tests -> pass, 127 tests.
- App workspace typecheck -> pass. Root typecheck failed then only in TASK-119 files, which were still in progress; it passed after TASK-120.
- `plugin reload board` on live -> running. Not clicked in the app.

## Lessons

- The card keeps the intent in `useState`, because the cluster test harness stubs `useRef`.
- The follow-ups wrote files that TASK-121, TASK-122, and TASK-123 had owned. It was safe only because those TASKs had finished.
