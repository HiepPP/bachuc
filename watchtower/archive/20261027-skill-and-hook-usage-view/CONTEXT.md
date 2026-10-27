# Plan Context

## Shared Context

- The feature shows which skills and hooks a Claude agent used in its session, in the GPUI client only. Electron, mobile, and other providers are out of scope.
- Skills already reach the timeline: the Claude `Skill` tool maps to a `plain_text` detail with `icon: "sparkles"` in [packages/server/src/server/agent/providers/claude/tool-call-detail-parser.ts](packages/server/src/server/agent/providers/claude/tool-call-detail-parser.ts). GPUI draws it in [packages/desktop-gpui/crates/timeline/src/tools.rs](packages/desktop-gpui/crates/timeline/src/tools.rs).
- Hooks do not reach the timeline today. `includeHookEvents` is unset in the Claude options built in [packages/server/src/server/agent/providers/claude/agent.ts](packages/server/src/server/agent/providers/claude/agent.ts).
- The SDK messages are `SDKHookStartedMessage` (`hook_id`, `hook_name`, `hook_event`) and `SDKHookResponseMessage` (adds `output`, `stdout`, `stderr`, `exit_code?`, `outcome: 'success' | 'error' | 'cancelled'`). See `node_modules/@anthropic-ai/claude-agent-sdk/sdk.d.ts`.
- Paseo registers its own callback hooks for `PreToolUse`, `PostToolUse`, and `SubagentStop` in `buildSubagentEffortHooks` in [packages/server/src/server/agent/providers/claude/agent.ts](packages/server/src/server/agent/providers/claude/agent.ts). They must not show.
- The protocol stays backward-compatible: new fields are optional, and no enum gains a value. Read [docs/protocol-compatibility.md](docs/protocol-compatibility.md).

## Decisions

- [ADR-0068](watchtower/decisions/ADR-0068-hooks-in-tool-calls-and-stored-usage-summary.md): tool hooks nest in their tool call; other hooks get a `hook` row; the daemon stores the summary in the agent record.
- A skill counts when the model calls the `Skill` tool, or when the user message starts with `/<name>` and `<name>` is in the Claude init `skills` list.
- Hook output is cut to 2048 characters.
- The summary has no `server_info.features` gate. The field is read-only data, so the chip hides when the field is absent or empty.

## Wire Shapes

- `ToolCallHook`: `{ id: string; name: string; event: string; status: "running" | "success" | "error" | "cancelled"; exitCode?: number; output?: string }`. `name` is the SDK `hook_name`, for example `PreToolUse:Bash`.
- Tool call items gain `hooks?: ToolCallHook[]`.
- A non-tool hook row is a tool call item: `callId: "hook:<hook_id>"`, `name: "hook"`, `detail: { type: "plain_text", label: <hook_name>, text?: <output>, icon: "wrench" }`, `status` `running`, `completed`, or `failed`, and `hooks: [<the one ToolCallHook>]`.
- `AgentSkillHookUsage`: `{ skills: SkillHookUsageEntry[]; hooks: SkillHookUsageEntry[] }`, newest first, at most 50 entries each.
- `SkillHookUsageEntry`: `{ name: string; event?: string; count: number; errorCount: number; lastUsedAt: string }`. `event` is set only for hooks.
- The agent snapshot gains `skillHookUsage?: AgentSkillHookUsage`.

## References

- [docs/gpui.md](docs/gpui.md)
- [docs/protocol-compatibility.md](docs/protocol-compatibility.md)
- [docs/testing.md](docs/testing.md)
