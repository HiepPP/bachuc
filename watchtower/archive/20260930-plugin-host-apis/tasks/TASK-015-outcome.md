# TASK-015 Outcome

## Outcome

Status: BLOCKED

Blocked:

- The bridge cannot be removed. The native Codex and Claude spawn hooks (`server/native-hook-command.ts`) call it over HTTP with `PASEO_ORCH_URL` and `PASEO_ORCH_TOKEN`, and `session_open` requires that binding for interactive Claude and Codex agents.
- The global hook matcher in `~/.codex/hooks.json`, written by `server/native-install.ts` from another checkout, names `mcp__jev_orchestrator__prepare_native_delegate`. Serving the tool from the daemon renames it to `mcp__paseo__prepare_native_delegate`, which that matcher would not see. This plan must not edit global provider configs.
- Moving only `delegate_task`, `orchestrator_status`, and `cancel_delegation` keeps the bridge and the stdio MCP process and only renames tools for agents.

Decision needed:

- Option 1: move only the 3 generic tools to `server.registerTool`, keep the bridge and the stdio MCP for the 2 native tools.
- Option 2: also reinstall the native hooks from this checkout with the new tool name. This edits `~/.codex/hooks.json` and `~/.claude/settings.json`.
- Option 3: leave jev-orchestrator unchanged.

Changed:

- Nothing. No file in [hiep-plugins/plugins/jev-orchestrator](hiep-plugins/plugins/jev-orchestrator) was edited.

Verified:

- Baseline `npm test` -> 89 passed; 2026-10-01T00:52.
