# TASK-113 No-jump latest-prompt anchor

Group: D
Class: code

## Brief

Goal: Open a thread at its latest prompt with no bottom pin and no visible jump.

Change: the anchor request stays pending until the jump lands, the reader scrolls, another open replaces it, no prompt exists, or a 30 s cap passes. While it waits, no bottom pin runs and the web scroll container is hidden. The jump runs on a replica-cache paint.

Boundaries:

- Web only. Native stays unchanged.
- Keep `hasPendingLatestPromptAnchor` with its name and meaning; TASK-112 uses it.

Files:

- [packages/app/src/agent-stream/latest-prompt-anchor.ts](packages/app/src/agent-stream/latest-prompt-anchor.ts)
- [packages/app/src/agent-stream/latest-prompt-anchor.test.ts](packages/app/src/agent-stream/latest-prompt-anchor.test.ts)
- [packages/app/src/agent-stream/strategy-web.tsx](packages/app/src/agent-stream/strategy-web.tsx)
- [packages/app/src/agent-stream/strategy-web.test.tsx](packages/app/src/agent-stream/strategy-web.test.tsx)
- [packages/app/src/agent-stream/use-latest-prompt-anchor-loader.ts](packages/app/src/agent-stream/use-latest-prompt-anchor-loader.ts)
- [packages/app/src/agent-stream/use-scroll-to-message.web.ts](packages/app/src/agent-stream/use-scroll-to-message.web.ts)
- [packages/app/src/screens/agent/agent-ready-screen-bottom-anchor.ts](packages/app/src/screens/agent/agent-ready-screen-bottom-anchor.ts)
- [packages/app/src/panels/agent-panel.tsx](packages/app/src/panels/agent-panel.tsx)

Expected result:

- The first visible frame of an anchored open shows the prompt.

## Verify

- `npx vitest run src/agent-stream/latest-prompt-anchor.test.ts --bail=1` in `packages/app` -> pass.
- `npx vitest run src/agent-stream/strategy-web.test.tsx --bail=1` -> pass.
- `npx vitest run src/agent-stream/prompt-jump-settle.test.ts --bail=1` -> pass.
- In the live app, open a board card -> the thread shows at the prompt with no jump. Needs a human or a browser check.
