# TASK-113 Outcome

## Status

DONE. Not committed. The live-app line of Verify is UNVERIFIED.

## Changed

- `latest-prompt-anchor.ts`: `takeLatestPromptAnchor` split into `beginLatestPromptAnchor` and `settleLatestPromptAnchor`. A settle carries a request id. `extendLatestPromptAnchor` removed. Cap 30 s.
- `strategy-web.tsx`: `visibility: hidden` while an anchor is pending and the transcript is not empty; no bottom pin while pending.
- `use-scroll-to-message.web.ts`: learns the end of a jump from the settle controller's user-intent unsubscribe.
- `agent-ready-screen-bottom-anchor.ts`: skips the bottom intent on web only.

## Verified

Measured 2026-10-15.

- Anchor test -> 7 passed. `strategy-web.test.tsx` -> 21 passed, 5 new. `prompt-jump-settle.test.ts` -> 6 passed. Bottom-anchor test -> 4 passed. `host-navigation.test.ts` -> 12 passed.
- Typecheck, lint, format -> pass.

## Lessons

- A bottom open through host navigation did not end a pending anchor. TASK-115 fixed it.
- Re-landing after an epoch change and the "landed under 2 s" rule were not built.
- Hidden-then-visible was tested in jsdom only.
