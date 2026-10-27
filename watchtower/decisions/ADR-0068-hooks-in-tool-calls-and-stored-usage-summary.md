# ADR-0068 Hooks nested in tool calls with a stored usage summary over per-hook rows and client counting

Status: accepted
Date: 2026-10-27
Supersedes: -
Scope: `packages/protocol/src/messages.ts`, `packages/protocol/src/agent-types.ts`, `packages/server/src/server/agent/providers/claude/**`, `packages/server/src/server/agent/agent-manager.ts`, `packages/server/src/server/agent/agent-storage.ts`, `packages/server/src/server/agent/agent-projections.ts`, `packages/server/src/server/agent/skill-hook-usage.ts`, `packages/desktop-gpui/crates/timeline/**`, `packages/desktop-gpui/crates/client/**`, `packages/desktop-gpui/crates/app/src/chrome.rs`

## Context

- The Claude SDK 0.3.246 emits `hook_started` and `hook_response` system messages only with `includeHookEvents: true`. The messages carry `hook_name` such as `PreToolUse:Bash`, and no `tool_use_id`.
- `PreToolUse` and `PostToolUse` hooks run on most tool calls on this machine, because the owner has a GitNexus hook for Bash and Grep.
- GPUI loads only the timeline tail and pages older rows through `load_older`.
- [ADR-0032](watchtower/decisions/ADR-0032-sqlite-served-old-pages-over-memory-only-serving.md): memory keeps a bounded tail, and every SQL read stays paged and small, because `DatabaseSync` blocks the event loop.

## Decision

- Tool hooks attach to their tool call item through a new optional `hooks` field. Other hooks, such as `SessionStart`, `UserPromptSubmit`, and `Stop`, become their own `tool_call` rows named `hook`.
- The daemon counts skill and hook use per agent, stores the summary in the agent record JSON, and sends it on the agent snapshot as optional `skillHookUsage`.
- Only the Claude provider and only the GPUI client are in scope.
- Hiep chose this on 2026-10-27 in the watchtower interview for TASK-305 to TASK-308.

## Consequences

- Old clients parse new tool call items, because `hooks` is optional. Electron and mobile show `hook` rows as generic tool rows.
- A summary starts at zero for agents created before this change. No timeline scan rebuilds it.
- The provider must re-emit a tool call item with the same `callId` when a hook result arrives.

## Options Considered

- One timeline row per hook: rejected. Tool hooks run on most tool calls, so the rows would double the timeline length.
- GPUI counts skills and hooks from loaded timeline rows: rejected. GPUI loads only the tail, so long sessions would show low counts.
- Rebuild the summary from the stored timeline on load: rejected. A full scan breaks the small paged read rule of ADR-0032.

## Revisit If

- The SDK adds `tool_use_id` to hook messages and a per-hook row stays under one row per tool call.
- A summary of agents created before this change becomes a need.
- ADR-0032 is superseded and full timeline reads become cheap.
