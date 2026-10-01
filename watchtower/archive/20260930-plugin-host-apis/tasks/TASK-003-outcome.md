# TASK-003 Outcome

## Outcome

Status: DONE

Changed:

- Added `registerTool`, `PluginToolDefinition`, `PluginToolContext`, and `PluginToolResult` in [packages/plugin/src/server/contracts.ts](packages/plugin/src/server/contracts.ts).
- The plugin process validates tools, sends them with JSON schema in `ready`, and handles `tool.call` in [packages/server/src/server/plugins/plugin-process.ts](packages/server/src/server/plugins/plugin-process.ts) and [packages/server/src/server/plugins/plugin-process-protocol.ts](packages/server/src/server/plugins/plugin-process-protocol.ts).
- The runtime keeps tools per plugin and calls them with a 120 s timeout in [packages/server/src/server/plugins/runtime.ts](packages/server/src/server/plugins/runtime.ts). `PluginService` exposes `listPluginTools` and `callPluginTool` in [packages/server/src/server/plugins/index.ts](packages/server/src/server/plugins/index.ts).
- `createPaseoToolCatalog` adds plugin tools after built-ins in [packages/server/src/server/agent/tools/paseo-tools.ts](packages/server/src/server/agent/tools/paseo-tools.ts). [packages/server/src/server/bootstrap.ts](packages/server/src/server/bootstrap.ts) passes the plugin service.
- Added [packages/server/src/server/plugins/plugin-tools.e2e.test.ts](packages/server/src/server/plugins/plugin-tools.e2e.test.ts).

Contract:

- Tools register at setup time. A name taken by a built-in is skipped and logged.
- A handler error returns an MCP error result.
- Native tool catalogs for OpenCode and OMP are built at daemon start, before plugins load, so they do not list plugin tools. The HTTP MCP route builds the catalog per request and does.

Verified:

- `npx vitest run packages/server/src/server/plugins/plugin-tools.e2e.test.ts --bail=1` -> 1 passed. It lists, calls with caller `caller-1`, returns an error result, and skips the clashing `list_agents`.
- `npx vitest run packages/server/src/server/plugins/runtime.posix.test.ts --bail=1` -> 40 passed.
- Server typecheck -> exit 0. `npm run lint` on 9 files -> 0 errors.

Anti-goal:

- Before plugin install: built-in names read from the MCP route in the same test; 2026-09-30T23:47.
- Final: after install, the list equals the built-ins plus `echo_caller` and `fail`; 2026-09-30T23:47.
- Result: PASS against "built-in list unchanged".
