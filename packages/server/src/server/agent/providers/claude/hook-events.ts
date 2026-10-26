import type { AgentTimelineItem, ToolCallHook } from "../../agent-sdk-types.js";

export const MAX_HOOK_OUTPUT_CHARS = 2048;

export type ClaudeHookMessage =
  | { phase: "started"; id: string; name: string; event: string }
  | {
      phase: "response";
      id: string;
      name: string;
      event: string;
      status: Exclude<ToolCallHook["status"], "running">;
      exitCode?: number;
      output?: string;
    };

const TOOL_HOOK_NAME = /^(?:PreToolUse|PostToolUse|PostToolUseFailure):(.+)$/;

function readString(value: unknown): string | null {
  return typeof value === "string" && value.length > 0 ? value : null;
}

/** Reads an SDK `hook_started` or `hook_response` system message; anything else is null. */
export function readClaudeHookMessage(message: unknown): ClaudeHookMessage | null {
  if (typeof message !== "object" || message === null) return null;
  const record = message as Record<string, unknown>;
  if (record.type !== "system") return null;
  const id = readString(record.hook_id);
  const name = readString(record.hook_name);
  if (!id || !name) return null;
  const event = readString(record.hook_event) ?? name.split(":")[0] ?? name;
  if (record.subtype === "hook_started") {
    return { phase: "started", id, name, event };
  }
  if (record.subtype !== "hook_response") return null;
  const exitCode = typeof record.exit_code === "number" ? record.exit_code : undefined;
  const status = readHookStatus(record.outcome, exitCode);
  const output = readString(record.output);
  return {
    phase: "response",
    id,
    name,
    event,
    status,
    ...(exitCode === undefined ? {} : { exitCode }),
    ...(output ? { output } : {}),
  };
}

function readHookStatus(
  outcome: unknown,
  exitCode: number | undefined,
): Exclude<ToolCallHook["status"], "running"> {
  if (outcome === "success" || outcome === "error" || outcome === "cancelled") return outcome;
  return exitCode !== undefined && exitCode !== 0 ? "error" : "success";
}

/** The tool a `PreToolUse:Bash`-style hook name targets, or null for a hook with no tool. */
export function hookToolName(hookName: string): string | null {
  return TOOL_HOOK_NAME.exec(hookName)?.[1] ?? null;
}

export function truncateHookOutput(output: string): string {
  if (output.length <= MAX_HOOK_OUTPUT_CHARS) return output;
  let end = MAX_HOOK_OUTPUT_CHARS - 1;
  const last = output.charCodeAt(end - 1);
  // Do not split a surrogate pair.
  if (last >= 0xd800 && last <= 0xdbff) end -= 1;
  return `${output.slice(0, end)}…`;
}

/** The skill a prompt invokes as `/<name>`, when `<name>` is in the init `skills` list. */
export function slashCommandSkill(prompt: string, skills: readonly string[]): string | null {
  const match = /^\s*\/(\S+)/.exec(prompt);
  const name = match?.[1];
  if (!name) return null;
  return skills.includes(name) ? name : null;
}

type HookRowItem = Extract<AgentTimelineItem, { type: "tool_call" }>;

/** The own timeline row of a hook that has no tool call to nest in. */
export function hookRowItem(hook: ToolCallHook): HookRowItem {
  const base = {
    type: "tool_call" as const,
    callId: `hook:${hook.id}`,
    name: "hook",
    detail: {
      type: "plain_text" as const,
      label: hook.name,
      ...(hook.output ? { text: hook.output } : {}),
      icon: "wrench" as const,
    },
    hooks: [hook],
  };
  switch (hook.status) {
    case "running":
      return { ...base, status: "running", error: null };
    case "success":
      return { ...base, status: "completed", error: null };
    case "cancelled":
      return { ...base, status: "canceled", error: null };
    case "error":
      return { ...base, status: "failed", error: hook.output ?? "Hook failed" };
  }
}
