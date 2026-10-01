# TASK-003 Plugin MCP tools served by the daemon

Group: A (server plugin host files)
Class: risky

## Brief

Goal: Let a plugin register MCP tools that the daemon serves to every agent through the `paseo` MCP server.

Change: plugins run their own MCP process and HTTP bridge -> `server.registerTool({ name, description, inputSchema, handler })`.

Boundaries:

- The handler gets `(input, { paseo, callerAgentId, signal })`. `callerAgentId` is the `callerAgentId` the daemon MCP route already reads. It has the same trust level as built-in Paseo tools.
- The handler returns `{ text, structured?, isError? }`.
- Tool names match `^[a-z][a-z0-9_]*$`. A name that clashes with a built-in or another plugin tool is skipped and logged.
- Tool calls use a 120 s timeout, not the 30 s request timeout.
- Plugin tools follow the plugin load state. A disabled plugin has no tools.

How:

- Add `registerTool` to [packages/plugin/src/server/contracts.ts](packages/plugin/src/server/contracts.ts).
- In the plugin process, send tools with JSON schema in `ready` and a `tools.changed` message, and handle a `tool.call` request.
- Extend the protocol schemas in [packages/server/src/server/plugins/plugin-process-protocol.ts](packages/server/src/server/plugins/plugin-process-protocol.ts).
- Store tools in the runtime, add `listTools` and `callTool` to the plugin service.
- Merge plugin tools into `createPaseoToolCatalog`. Convert JSON schema back with zod `fromJSONSchema`.
- Add tests: an e2e plugin registers a tool, the catalog lists it, a call returns the handler result with the caller ID.

Files:

- [packages/plugin/src/server/contracts.ts](packages/plugin/src/server/contracts.ts) (API type)
- [packages/server/src/server/plugins/plugin-process.ts](packages/server/src/server/plugins/plugin-process.ts) (setup and tool calls)
- [packages/server/src/server/plugins/plugin-process-protocol.ts](packages/server/src/server/plugins/plugin-process-protocol.ts) (messages)
- [packages/server/src/server/plugins/runtime.ts](packages/server/src/server/plugins/runtime.ts) (tool state, callTool)
- [packages/server/src/server/plugins/index.ts](packages/server/src/server/plugins/index.ts) (service methods)
- [packages/server/src/server/agent/tools/paseo-tools.ts](packages/server/src/server/agent/tools/paseo-tools.ts) (catalog merge)
- [packages/server/src/server/bootstrap.ts](packages/server/src/server/bootstrap.ts) (wire the plugin service into the catalog)
- A new or existing e2e test under [packages/server/src/server/plugins](packages/server/src/server/plugins)

Expected result:

- An agent can list and call a plugin tool through the daemon MCP route.
- The handler sees the caller agent ID.
- A handler error becomes an MCP error result, not a daemon crash.

Anti-goal: Built-in Paseo tools stay the same; tripwire; limit the built-in tool name list from `createPaseoToolCatalog` with no plugins is unchanged; read before changes and at completion.

## Verify

- `npx vitest run <new plugin tool e2e test> --bail=1` -> pass.
- `npx vitest run packages/server/src/server/plugins/runtime.posix.test.ts --bail=1` -> pass.
- `npm run typecheck` -> exit 0.
- Anti-goal: a test or script prints the built-in tool names with no plugins, and the list matches the pre-change list.
