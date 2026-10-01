# TASK-001 Server hook before agent.prompt

Group: A (server plugin host files)
Class: risky

## Brief

Goal: Let a plugin rewrite or extend the prompt a provider receives, for every provider, on each turn and steer.

Change: no per-turn prompt hook -> `server.before("agent.prompt", ...)` runs before `startTurn` and both steer calls.

Boundaries:

- Request: `{ agentId, workspaceId, provider, cwd, kind: "turn" | "steer", prompt }`. Only `prompt` may change. `prompt` keeps the `AgentPromptInput` shape (string or content blocks).
- The timeline keeps the original prompt. `recordSubmittedPrompt` stays on the original.
- Skip internal agents and out-of-band commands such as `/goal`.
- Run the hook before `runSteerAdmission`, so a slow plugin does not hold the steer barrier.
- A hook error fails the turn, like other before-hooks.

How:

- Add `agent.prompt` to `PluginBeforeRequests` and `beforeHookNames`, `beforeSchemas`, and `validateBeforeResult`.
- Add a private `applyPromptHook(agent, prompt, kind)` in `AgentManager`. Call it before `session.startTurn` and before both `steerActiveTurn` calls.
- Add unit tests in [packages/server/src/server/plugins/lifecycle/handlers.test.ts](packages/server/src/server/plugins/lifecycle/handlers.test.ts) and a case in [packages/server/src/server/plugins/lifecycle.e2e.test.ts](packages/server/src/server/plugins/lifecycle.e2e.test.ts).

Files:

- [packages/plugin/src/server/lifecycle.ts](packages/plugin/src/server/lifecycle.ts) (request type)
- [packages/server/src/server/plugins/lifecycle/index.ts](packages/server/src/server/plugins/lifecycle/index.ts) (name, schema, result rules)
- [packages/server/src/server/agent/agent-manager.ts](packages/server/src/server/agent/agent-manager.ts) (hook calls)
- [packages/server/src/server/plugins/lifecycle/handlers.test.ts](packages/server/src/server/plugins/lifecycle/handlers.test.ts) (tests)
- [packages/server/src/server/plugins/lifecycle.e2e.test.ts](packages/server/src/server/plugins/lifecycle.e2e.test.ts) (tests)

Expected result:

- A plugin that appends text changes what the provider receives on a turn and on a steer.
- The user timeline item still shows the typed text.
- A plugin that changes `agentId` is rejected.
- Internal agents skip the hook.

Anti-goal: Existing lifecycle behavior; tripwire; limit every test in [packages/server/src/server/plugins/lifecycle/handlers.test.ts](packages/server/src/server/plugins/lifecycle/handlers.test.ts) that passed before still passes; read before changes and at completion.

## Verify

- `npx vitest run packages/server/src/server/plugins/lifecycle/handlers.test.ts --bail=1` -> all pass, including the new `agent.prompt` cases.
- `npx vitest run packages/server/src/server/plugins/lifecycle.e2e.test.ts --bail=1` -> all pass, including a turn whose provider prompt holds the appended text while the timeline holds the typed text.
- `npm run typecheck` -> exit 0.
- Anti-goal: the pre-change passing test count in handlers.test.ts is still passing.
