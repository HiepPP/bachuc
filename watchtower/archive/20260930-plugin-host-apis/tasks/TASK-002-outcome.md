# TASK-002 Outcome

## Outcome

Status: DONE

Changed:

- Added `PluginPermissionDecisionRequest` and the `agent.permission` before-hook in [packages/plugin/src/server/lifecycle.ts](packages/plugin/src/server/lifecycle.ts).
- Added the schema and the "only decision may change" rule in [packages/server/src/server/plugins/lifecycle/index.ts](packages/server/src/server/plugins/lifecycle/index.ts).
- Added `onStreamPermissionRequestedWithPlugins` in [packages/server/src/server/agent/agent-manager.ts](packages/server/src/server/agent/agent-manager.ts). A decision calls `session.respondToPermission` and skips pending state, attention, push, stream dispatch, and the `agent.permission_requested` plugin event.

Contract:

- No decision, a hook error, or a timeout keeps today's flow. History replay and internal agents skip the hook.
- The hook runs inside the per-agent event queue, so a slow plugin delays that agent's later events by up to 30 s.

Verified:

- Handler tests -> 8 passed, including decision and rejected request change.
- Lifecycle e2e -> 9 passed. With a deny decision, no `agent.permission_requested` event fires and no permission stays pending. With a throwing hook, the normal event fires and the plugin answers it.
- Server typecheck -> exit 0. Lint -> 0 errors.

Anti-goal:

- Final: the failing-hook e2e test shows the permission reaching the normal flow; 2026-09-30T23:43.
- Result: PASS against a limit of 0 lost permissions.
