# TASK-002 Server hook before agent.permission

Group: A (server plugin host files)
Class: risky

## Brief

Goal: Let a plugin answer a permission request before clients, attention, and push notifications see it.

Change: plugins only hear `agent.permission_requested` after the broadcast -> `server.before("agent.permission", ...)` can set `decision` first.

Boundaries:

- Request: `{ agentId, workspaceId, provider, cwd, request, decision: null }`. Only `decision` may change. `decision` is an `AgentPermissionResponse` or `null`.
- The hook runs in the `permission_requested` case of `handleStreamEvent`, before `onStreamPermissionRequested`.
- With a decision, call `respondToPermission`, skip pending state, attention, push, and the stream dispatch.
- On a hook error or timeout, log it and continue to the normal flow. Never drop the permission.
- Skip history replay and internal agents.

How:

- Add `agent.permission` to the same places as TASK-001.
- Make the `permission_requested` case await the hook.
- Add tests for allow, deny, no decision, and a failing hook.

Files:

- [packages/plugin/src/server/lifecycle.ts](packages/plugin/src/server/lifecycle.ts) (request type)
- [packages/server/src/server/plugins/lifecycle/index.ts](packages/server/src/server/plugins/lifecycle/index.ts) (name, schema, result rules)
- [packages/server/src/server/agent/agent-manager.ts](packages/server/src/server/agent/agent-manager.ts) (hook call)
- [packages/server/src/server/plugins/lifecycle/handlers.test.ts](packages/server/src/server/plugins/lifecycle/handlers.test.ts) (tests)
- [packages/server/src/server/plugins/lifecycle.e2e.test.ts](packages/server/src/server/plugins/lifecycle.e2e.test.ts) (tests)

Expected result:

- A plugin decision resolves the permission, and no `agent_attention_required` or pending permission is sent.
- No decision keeps today's flow.
- A throwing hook keeps today's flow.

Anti-goal: The permission is never lost; tripwire; limit 0 cases where a hook error leaves the permission without a pending entry or a response; read from the new failing-hook test at completion.

## Verify

- `npx vitest run packages/server/src/server/plugins/lifecycle/handlers.test.ts --bail=1` -> all pass.
- `npx vitest run packages/server/src/server/plugins/lifecycle.e2e.test.ts --bail=1` -> all pass, including allow, deny, no decision, and failing-hook cases.
- `npm run typecheck` -> exit 0.
- Anti-goal: the failing-hook test shows the permission as pending.
