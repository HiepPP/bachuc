# TASK-106 GLM in the app

Group: C (app icon map and stream reducer; follows TASK-104 by ordering alone)
Class: code

## Brief

Goal: Show the `glm` provider with the GLM icon, and give glm agents the same todo and task list UI that Claude agents get.

Change: `glm` falls back to a generic icon and skips Claude-only stream branches -> it uses the GLM icon and those branches.

Boundaries:

- Reuse [packages/app/src/assets/acp-provider-icons/glm-acp-agent.svg](packages/app/src/assets/acp-provider-icons/glm-acp-agent.svg). Add no new asset.
- [packages/app/src/types/stream.ts](packages/app/src/types/stream.ts) lines 1405 and 1422 only build todo and task lists when `event.provider === "claude"`. The glm agent emits the same Claude tool names, so these branches must also accept `glm`. Use one shared helper or constant, not two copied checks.
- Do not change behavior for any other provider.

How:

- Add `glm` to `BUILTIN_PROVIDER_ICONS` in [packages/app/src/components/provider-icons.ts](packages/app/src/components/provider-icons.ts), pointing at the GLM svg.
- Add a small helper, for example `isClaudeCodeProvider(provider)`, that returns true for `claude` and `glm`. Use it at both stream branches.
- Search the app for other `=== "claude"` checks. Change only the ones that read Claude Code tool output. List the ones you leave in the outcome.
- Add test cases to [packages/app/src/types/stream.test.ts](packages/app/src/types/stream.test.ts) for a glm `TodoWrite` and a glm `TaskCreate`.

Files:

- [packages/app/src/components/provider-icons.ts](packages/app/src/components/provider-icons.ts) (glm icon)
- [packages/app/src/types/stream.ts](packages/app/src/types/stream.ts) (Claude Code provider helper at the two branches)
- [packages/app/src/types/stream.test.ts](packages/app/src/types/stream.test.ts) (glm cases)

Expected result:

- The provider picker and settings show GLM with the GLM icon.
- A glm `TodoWrite` call builds a todo list, like a Claude call does.
- An unparseable `TodoWrite` call is hidden for claude and glm, and stays a visible tool call for codex. (Fixed at implement: the old line said codex `TodoWrite` builds no todo list, but the generic parser builds one for every provider.)

Anti-goal: failing tests in [packages/app/src/types/stream.test.ts](packages/app/src/types/stream.test.ts) stay at 0; tripwire; read before changes, after the helper change, and at completion.

## Verify

- `npx vitest run packages/app/src/types/stream.test.ts --bail=1` -> pass, with new glm cases.
- `npm run typecheck` -> exit 0.
- `npm run lint -- packages/app/src/components/provider-icons.ts packages/app/src/types/stream.ts` -> exit 0.
- In the live desktop app, open the new agent provider picker -> GLM shows with the GLM icon. Needs a human or a browser check.
- Anti-goal: the stream test run above -> 0 failures at each read point.
