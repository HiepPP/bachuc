# TASK-305 Capture Claude hooks and skill use in the provider

Group: A (shared file with TASK-306: `packages/protocol/src/messages.ts`, `packages/server/src/server/agent/agent-sdk-types.ts`)
Class: risky

## Brief

Goal: The Claude provider turns SDK hook messages into timeline data and emits one internal event per skill use and per finished hook.

Change: hooks are dropped and slash-command skills are not marked -> tool hooks nest in their tool call, other hooks get a `hook` row, and `skill_used` and `hook_used` events reach the agent manager.

Boundaries:

- Follows [ADR-0068](watchtower/decisions/ADR-0068-hooks-in-tool-calls-and-stored-usage-summary.md): nest tool hooks in the tool call; do not add one row per tool hook.
- Use the shapes in `## Wire Shapes` of [watchtower/CONTEXT.md](watchtower/CONTEXT.md) exactly.
- Protocol: add `hooks` as an optional field on `ToolCallBasePayloadSchema`. Do not add a value to `TOOL_CALL_ICON_NAMES`, because an old client fails to parse an unknown enum value.
- Planner call: match a tool hook to the newest main-thread tool call whose `name` equals the tool part of `hook_name` (`PreToolUse:Bash` -> `Bash`). A `PreToolUse` hook matches a `running` call; a `PostToolUse` or `PostToolUseFailure` hook matches the newest call of that name in the current turn. With no match, the hook gets its own `hook` row.
- Planner call: emit `hook_used` only on `hook_response`, so each hook counts once.
- Planner call: emit `skill_used` with `source: "tool"` once per `Skill` tool call id, from the live path `translateMessageToEvents` in `agent.ts`. Keep a set of counted call ids for the session.
- Planner call: emit `source: "slash_command"` where the provider builds the SDK `type: "user"` prompt message for a live send. Match when the prompt, after leading whitespace, starts with `/<name>` and `<name>` is in the `skills` list of the latest Claude `init` message. The match is case-sensitive. Keep the list as new state in `handleSystemMessage`; the SDK init type has `skills: string[]`.
- Planner call: when a turn ends, fails, or is interrupted, re-emit each hook still `running` as `cancelled`.
- Paseo callback hooks: before you write the filter, log the raw `hook_started` messages of one real turn. If the SDK emits events for the callbacks from `buildSubagentEffortHooks`, drop them by a rule you prove in the log. If no rule tells them apart, keep them and write the fact in the outcome.
- Trap: tool call re-emit in `agent.ts`: the agent manager replaces a tool call item by `callId` (see `timelineItemIdentity` in [packages/protocol/src/timeline-identity.ts](packages/protocol/src/timeline-identity.ts)). Re-emit the newest full item with the new `hooks` array. Re-emitting an older `running` copy turns a finished call back to running.
- Trap: `mapBlocksToTimeline` in `agent.ts`: both the live path and history replay (`convertClaudeHistoryEntry`, then `streamHistory`) use it. Emitting `skill_used` there counts every replayed `Skill` call again. Emit the events only from `translateMessageToEvents` and the prompt send path.
- Trap: `appendUserMessageEvents` in `agent.ts`: it returns early for synthetic, compacting, and duplicate messages. Do not put the slash-command check there.
- Trap: `limitAgentTimelineItemContent` in [packages/server/src/server/agent/agent-timeline-content.ts](packages/server/src/server/agent/agent-timeline-content.ts) may cut items too. Cut hook output to 2048 characters in the provider anyway, so the live stream stays small.

How:

- Add pure helpers in a new `hook-events.ts`: `readClaudeHookMessage`, `hookToolName`, `truncateHookOutput`, and `slashCommandSkill`.
- In `agent.ts`, set `includeHookEvents: true` in the base Claude options, and handle the `hook_started` and `hook_response` subtypes in `appendSystemMessageEvents`.
- Keep the newest tool call item per `callId` for the current turn, and keep the `skills` list from the `init` message.
- Add the `ToolCallHook` type, the `hooks` field, and the two internal stream events to the server and protocol types.

Files:

- [packages/server/src/server/agent/providers/claude/hook-events.ts](packages/server/src/server/agent/providers/claude/hook-events.ts) (new pure helpers)
- [packages/server/src/server/agent/providers/claude/hook-events.test.ts](packages/server/src/server/agent/providers/claude/hook-events.test.ts) (new tests)
- [packages/server/src/server/agent/providers/claude/agent.ts](packages/server/src/server/agent/providers/claude/agent.ts) (option, hook handling, tool call re-emit, skill events)
- [packages/server/src/server/agent/providers/claude/agent.test.ts](packages/server/src/server/agent/providers/claude/agent.test.ts) (stream tests for nesting, `hook` rows, and events)
- [packages/server/src/server/agent/agent-sdk-types.ts](packages/server/src/server/agent/agent-sdk-types.ts) (`ToolCallHook`, `hooks?`, `skill_used`, `hook_used`)
- [packages/protocol/src/agent-types.ts](packages/protocol/src/agent-types.ts) (`ToolCallHook`, `hooks?` on `ToolCallTimelineItem`)
- [packages/protocol/src/messages.ts](packages/protocol/src/messages.ts) (`ToolCallHookPayloadSchema`, optional `hooks` on `ToolCallBasePayloadSchema`)
- [packages/protocol/src/messages.wire-compat.test.ts](packages/protocol/src/messages.wire-compat.test.ts) (an item with `hooks` and an item without it both parse)

Interfaces:

- Produces: `ToolCallHook = { id: string; name: string; event: string; status: "running" | "success" | "error" | "cancelled"; exitCode?: number; output?: string }` for TASK-306, TASK-307
- Produces: `hooks?: ToolCallHook[]` on tool call timeline items for TASK-307
- Produces: tool call item `name: "hook"`, `callId: "hook:<hook_id>"` for non-tool hooks, for TASK-307
- Produces: `AgentStreamEvent` member `{ type: "skill_used"; provider: AgentProvider; name: string; source: "tool" | "slash_command" }` for TASK-306
- Produces: `AgentStreamEvent` member `{ type: "hook_used"; provider: AgentProvider; hook: ToolCallHook }` for TASK-306

Expected result:

- WHEN a `PreToolUse:Bash` hook starts and ends during a running Bash call THE Claude provider SHALL re-emit that call with one `hooks` entry, first `running`, then `success`, and SHALL add no timeline row.
- WHEN a `SessionStart` or `UserPromptSubmit` hook ends THE Claude provider SHALL emit one tool call item named `hook` with `callId` `hook:<hook_id>`.
- WHEN a hook ends with `outcome: "error"` THE Claude provider SHALL set its status to `error` and keep `exitCode`.
- IF hook output is longer than 2048 characters THEN THE Claude provider SHALL store at most 2048 characters.
- WHEN the model calls the `Skill` tool THE Claude provider SHALL emit one `skill_used` event with `source: "tool"` per call id.
- WHEN the user prompt is `/watchtower new` and `watchtower` is in the init `skills` list THE Claude provider SHALL emit `skill_used` with `name: "watchtower"` and `source: "slash_command"`.
- IF the prompt is `/compact` and `compact` is not in the `skills` list THEN THE Claude provider SHALL emit no `skill_used` event.
- WHILE the provider replays history, THE Claude provider SHALL emit no `skill_used` or `hook_used` event.
- IF a hook started and the turn ends before its `hook_response` THEN THE Claude provider SHALL re-emit that hook as `cancelled`.

Red tests:

File: [packages/server/src/server/agent/providers/claude/hook-events.test.ts](packages/server/src/server/agent/providers/claude/hook-events.test.ts)

```ts
import { describe, expect, it } from "vitest";
import {
  hookToolName,
  readClaudeHookMessage,
  slashCommandSkill,
  truncateHookOutput,
} from "./hook-events.js";

describe("readClaudeHookMessage", () => {
  it("reads a started hook", () => {
    expect(
      readClaudeHookMessage({
        type: "system",
        subtype: "hook_started",
        hook_id: "h1",
        hook_name: "PreToolUse:Bash",
        hook_event: "PreToolUse",
        uuid: "u1",
        session_id: "s1",
      }),
    ).toEqual({ phase: "started", id: "h1", name: "PreToolUse:Bash", event: "PreToolUse" });
  });

  it("reads a failed hook response with its exit code and output", () => {
    expect(
      readClaudeHookMessage({
        type: "system",
        subtype: "hook_response",
        hook_id: "h1",
        hook_name: "PreToolUse:Bash",
        hook_event: "PreToolUse",
        output: "blocked",
        stdout: "",
        stderr: "blocked",
        exit_code: 2,
        outcome: "error",
        uuid: "u2",
        session_id: "s1",
      }),
    ).toEqual({
      phase: "response",
      id: "h1",
      name: "PreToolUse:Bash",
      event: "PreToolUse",
      status: "error",
      exitCode: 2,
      output: "blocked",
    });
  });

  it("ignores other system messages", () => {
    expect(readClaudeHookMessage({ type: "system", subtype: "status" })).toBeNull();
  });
});

describe("hookToolName", () => {
  it("returns the tool of a tool hook", () => {
    expect(hookToolName("PreToolUse:Bash")).toBe("Bash");
    expect(hookToolName("PostToolUse:Grep")).toBe("Grep");
    expect(hookToolName("PostToolUseFailure:Edit")).toBe("Edit");
  });

  it("returns null for a hook that has no tool", () => {
    expect(hookToolName("SessionStart:startup")).toBeNull();
    expect(hookToolName("UserPromptSubmit")).toBeNull();
    expect(hookToolName("Stop")).toBeNull();
  });
});

describe("truncateHookOutput", () => {
  it("keeps short output", () => {
    expect(truncateHookOutput("ok")).toBe("ok");
  });

  it("cuts long output to 2048 characters", () => {
    expect(truncateHookOutput("x".repeat(5000)).length).toBeLessThanOrEqual(2048);
  });
});

describe("slashCommandSkill", () => {
  it("returns a known skill", () => {
    expect(slashCommandSkill("/watchtower new tasks", ["watchtower", "drill-me"])).toBe(
      "watchtower",
    );
  });

  it("returns a plugin-scoped skill", () => {
    expect(slashCommandSkill("/anthropic-skills:pdf", ["anthropic-skills:pdf"])).toBe(
      "anthropic-skills:pdf",
    );
  });

  it("ignores a command that is not a skill", () => {
    expect(slashCommandSkill("/compact", ["watchtower"])).toBeNull();
  });

  it("allows leading whitespace", () => {
    expect(slashCommandSkill("\n  /watchtower", ["watchtower"])).toBe("watchtower");
  });

  it("ignores text that does not start with a slash", () => {
    expect(slashCommandSkill("run /watchtower", ["watchtower"])).toBeNull();
  });
});
```

Anti-goal: timeline rows per turn; a turn with one Bash call and its `PreToolUse` and `PostToolUse` hooks has exactly one tool call identity; tripwire; read from the stream test in [packages/server/src/server/agent/providers/claude/agent.test.ts](packages/server/src/server/agent/providers/claude/agent.test.ts) before changes, after the nesting change, and at completion.

## Verify

- `npx vitest run packages/server/src/server/agent/providers/claude/hook-events.test.ts --bail=1` -> all tests pass.
- `npx vitest run packages/server/src/server/agent/providers/claude/agent.test.ts --bail=1` -> new tests pass for: nesting, a `SessionStart` `hook` row, the error status, `skill_used` once per `Skill` call id, the slash-command skill, a hook left `running` at turn end becoming `cancelled`, and no events from `streamHistory`. Old tests stay green.
- `npx vitest run packages/protocol/src/messages.wire-compat.test.ts --bail=1` -> a tool call item with `hooks` and one without it both parse.
- A turn whose hook ends with `outcome: "error"` and `exit_code: 2` -> the stream test shows the call's hook with status `error` and `exitCode` 2.
- Two Bash calls in one assistant message with one `PreToolUse:Bash` hook each -> the stream test shows each hook on a different call, or the outcome states the mis-attach limit.
- `npm run build:server && npm run typecheck` -> exit 0.
- `npm run lint -- packages/server/src/server/agent/providers/claude packages/protocol/src` -> exit 0.
- Live probe, agent-run: start live, and add a temporary debug log of raw `hook_started` messages. Run one Claude agent turn that uses Bash with the owner's GitNexus hook. Save the lines in the outcome, then remove the debug log -> the Bash tool call item carries a `hooks` entry, and Paseo callback hooks are absent or the outcome names why they stay.
- Anti-goal read: the one-Bash-turn stream test -> exactly one tool call identity at each checkpoint.
