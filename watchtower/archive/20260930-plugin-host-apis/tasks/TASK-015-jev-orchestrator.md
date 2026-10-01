# TASK-015 P6 jev-orchestrator serves its tools through the daemon

Group: I (hiep-plugins/plugins/jev-orchestrator)
Class: risky

## Brief

Goal: Remove the orchestrator HTTP bridge and its stdio MCP process.

Change: bridge tokens, leases, port state, and an injected stdio MCP server -> `server.registerTool` for all 5 tools, scoped by `callerAgentId`.

Boundaries:

- Tools: `delegate_task`, `orchestrator_status`, `cancel_delegation`, `prepare_native_delegate`, `native_delegation_status`.
- The caller scope (cwd) comes from the caller agent through the Paseo API, not from a lease token.
- Keep native Codex hook setup in `before("agent.session_open")`. Only the bridge-related env goes away.
- Children created by the orchestrator keep `PASEO_ORCH_CHILD=1`. Tools reject calls from them where they do today.
- Raise `requirements.paseo` to `>=0.10.2-beta.900`.

How:

- Move each tool handler from [hiep-plugins/plugins/jev-orchestrator/server/mcp.ts](hiep-plugins/plugins/jev-orchestrator/server/mcp.ts) into `registerTool` calls.
- Remove [hiep-plugins/plugins/jev-orchestrator/server/bridge.ts](hiep-plugins/plugins/jev-orchestrator/server/bridge.ts) and its env wiring.
- Update tests and the README.

Files:

- [hiep-plugins/plugins/jev-orchestrator/index.server.ts](hiep-plugins/plugins/jev-orchestrator/index.server.ts) (tools, hooks)
- Files under [hiep-plugins/plugins/jev-orchestrator/server](hiep-plugins/plugins/jev-orchestrator/server) and [hiep-plugins/plugins/jev-orchestrator/tests](hiep-plugins/plugins/jev-orchestrator/tests)
- [hiep-plugins/plugins/jev-orchestrator/paseo-plugin.json](hiep-plugins/plugins/jev-orchestrator/paseo-plugin.json), [hiep-plugins/plugins/jev-orchestrator/package.json](hiep-plugins/plugins/jev-orchestrator/package.json), [hiep-plugins/plugins/jev-orchestrator/README.md](hiep-plugins/plugins/jev-orchestrator/README.md)

Expected result:

- An agent in Paseo Dev sees `mcp__paseo__delegate_task` and the other 4 tools.
- A call from agent A cannot read or cancel jobs of agent B.

Anti-goal: Delegation behavior stays the same; tripwire; limit engine tests in [hiep-plugins/plugins/jev-orchestrator/tests](hiep-plugins/plugins/jev-orchestrator/tests) that do not test the bridge pass unchanged; read before changes and at completion.

## Verify

- `rg -n "createServer|PASEO_ORCH_URL|PASEO_ORCH_TOKEN" hiep-plugins/plugins/jev-orchestrator/server hiep-plugins/plugins/jev-orchestrator/index.server.ts` -> no match.
- `cd hiep-plugins/plugins/jev-orchestrator && npm run typecheck && npm run lint && npm test` -> exit 0.
- Real host check in Paseo Dev (TASK-017): an agent calls `orchestrator_status`.
- Anti-goal: non-bridge engine tests pass without edits.
