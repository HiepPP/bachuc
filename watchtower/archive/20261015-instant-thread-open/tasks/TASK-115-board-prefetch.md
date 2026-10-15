# TASK-115 Board prefetch and follow-ups

Group: D
Class: code

## Brief

Goal: Warm a thread from the board before the click lands, and close two gaps left by TASK-113 and TASK-114.

Change:

- A board run card calls `navigation.prefetchAgent` after 120 ms of hover and at once on press-in. Hover out cancels the timer.
- A host-navigation open without `anchor: "latest-prompt"` ends a pending anchor.
- `ViewedTimelineSync` gets a public desired check, used by the prefetch.

Boundaries:

- Read [hiep-plugins/AGENTS.md](hiep-plugins/AGENTS.md) first. Keep the existing hover state.
- Reload the plugin on live only.

Files:

- [hiep-plugins/plugins/board/client/page.tsx](hiep-plugins/plugins/board/client/page.tsx)
- [hiep-plugins/plugins/board/client/prefetch.ts](hiep-plugins/plugins/board/client/prefetch.ts) (new)
- [hiep-plugins/plugins/board/tests/prefetch.test.ts](hiep-plugins/plugins/board/tests/prefetch.test.ts) (new)
- [packages/app/src/plugins/host-navigation.ts](packages/app/src/plugins/host-navigation.ts)
- [packages/app/src/agent-stream/latest-prompt-anchor.ts](packages/app/src/agent-stream/latest-prompt-anchor.ts)
- [packages/app/src/timeline/viewed-timeline-sync.ts](packages/app/src/timeline/viewed-timeline-sync.ts)
- [packages/app/src/timeline/agent-prefetch.ts](packages/app/src/timeline/agent-prefetch.ts)
- the tests of those app files

Expected result:

- Resting the pointer on a card for 120 ms starts one prefetch; a pass-over starts none.

## Verify

- The board plugin `npm test`, `npm run typecheck`, `npm run lint` -> pass.
- The app test files of the changed files, one at a time -> pass.
- `npm run cli -- plugin reload board` on live -> status running.
- In the live app, click a closed thread's card -> it opens at once. Needs a human or a browser check.
