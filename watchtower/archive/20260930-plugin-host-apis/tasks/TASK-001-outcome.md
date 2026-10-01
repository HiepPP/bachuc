# TASK-001 Outcome

## Outcome

Status: DONE

Changed:

- Added `PluginPromptRequest` and the `agent.prompt` before-hook in [packages/plugin/src/server/lifecycle.ts](packages/plugin/src/server/lifecycle.ts), exported from [packages/plugin/src/server/index.ts](packages/plugin/src/server/index.ts).
- Added the name, zod schema, and "only prompt may change" rule in [packages/server/src/server/plugins/lifecycle/index.ts](packages/server/src/server/plugins/lifecycle/index.ts).
- Added `applyPromptHook` in [packages/server/src/server/agent/agent-manager.ts](packages/server/src/server/agent/agent-manager.ts). It runs before `session.startTurn` and before both `steerActiveTurn` calls, outside the steer barrier.

Contract:

- Second fix from the host check: after a daemon restart, Claude replays its own history, which holds the prompt it received. Appended plugin text is now wrapped in `<paseo-plugin-context>` by [packages/server/src/server/agent/plugin-prompt-context.ts](packages/server/src/server/agent/plugin-prompt-context.ts), and `limitAgentTimelineItemContent` strips it from user messages. A prompt a plugin replaces, rather than extends, is not marked. [packages/server/src/server/agent/plugin-prompt-context.test.ts](packages/server/src/server/agent/plugin-prompt-context.test.ts) -> 3 passed.

- Fix found in the TASK-017 host check: a prompt without a client message ID, such as `paseo run`, showed the plugin context in the user bubble, because the provider echo carried the rewritten text. `startPendingForegroundTurn` now adds a generated `clientMessageId` when the hook changed the prompt, so the original is recorded and the echo is matched. A new e2e case covers `initialPrompt` without a message ID; the lifecycle e2e file passes 9 of 9.

- The timeline records the original prompt. Echo matching uses `clientMessageId`, so a rewritten echo does not add a second bubble.
- Internal agents skip the hook. A hook error fails the turn.

Verified:

- `npx vitest run packages/server/src/server/plugins/lifecycle/handlers.test.ts --bail=1` -> 8 passed (4 old, 4 new).
- `npx vitest run packages/server/src/server/plugins/lifecycle.e2e.test.ts --bail=1` -> 9 passed. The new test shows the fake provider acting on the appended text while the timeline shows "Say hi.".
- `npx tsc -p packages/server/tsconfig.server.typecheck.json --noEmit` -> exit 0. `npm run lint` on 6 changed files -> 0 errors.
- Not covered by a test: the steer path. The fake provider has no steer support.

Anti-goal:

- Before changes: not read separately. The 4 old tests were not edited.
- Final: the 4 old tests pass; 2026-09-30T23:43.
- Result: PASS against "old tests pass unchanged".
