import type { SDKMessage } from "@anthropic-ai/claude-agent-sdk";

import type {
  AgentBackgroundProcess,
  AgentBackgroundProcessStatus,
} from "../../agent-sdk-types.js";

/**
 * Background shells and monitors, read from Claude Code's task protocol. Verified on the wire
 * (Claude Code 2.1.294):
 *
 *   task_started       task_type "local_bash", tool_use_id, description, is_backgrounded
 *   tool result        "Output is being written to: <path>. ..."
 *   task_updated       patch.status, patch.end_time
 *   task_notification  status, output_file, summary "... completed (exit code 0)"
 *
 * The Monitor tool also announces as "local_bash"; only the tool name of its call tells it
 * apart. MCP monitors announce as "monitor_mcp" or "monitor_ws".
 */

const BASH_TASK_TYPE = "local_bash";
const MONITOR_TASK_TYPES = new Set(["monitor_mcp", "monitor_ws"]);
const MONITOR_TOOL_NAME = "Monitor";
const OUTPUT_PATH_PATTERN = /Output is being written to: (\S+?)\.?(?:\s|$)/;
const EXIT_CODE_PATTERN = /exit code (-?\d+)/;

export interface BackgroundToolUse {
  name: string;
  command: string | null;
}

export interface ClaudeBackgroundProcessesInput {
  lookupToolUse: (toolUseId: string) => BackgroundToolUse | undefined;
  /** A shell started inside a subagent belongs to that child, not to this agent. */
  isOwnedBySubagent: (taskId: string, toolUseId: string | undefined) => boolean;
  now?: () => Date;
}

function readString(value: unknown): string | undefined {
  return typeof value === "string" && value.trim().length > 0 ? value.trim() : undefined;
}

function mapStatus(status: unknown): AgentBackgroundProcessStatus | undefined {
  switch (status) {
    case "pending":
    case "running":
    case "paused":
      return "running";
    case "completed":
      return "completed";
    case "failed":
      return "failed";
    case "killed":
    case "stopped":
      return "stopped";
    default:
      return undefined;
  }
}

function readToolResultText(content: unknown): string {
  if (typeof content === "string") return content;
  if (!Array.isArray(content)) return "";
  return content
    .map((block) =>
      block && typeof block === "object" && typeof block.text === "string" ? block.text : "",
    )
    .join("\n");
}

export class ClaudeBackgroundProcesses {
  private readonly processes = new Map<string, AgentBackgroundProcess>();
  private readonly processIdByToolUseId = new Map<string, string>();
  private readonly lookupToolUse: ClaudeBackgroundProcessesInput["lookupToolUse"];
  private readonly isOwnedBySubagent: ClaudeBackgroundProcessesInput["isOwnedBySubagent"];
  private readonly now: () => Date;

  constructor(input: ClaudeBackgroundProcessesInput) {
    this.lookupToolUse = input.lookupToolUse;
    this.isOwnedBySubagent = input.isOwnedBySubagent;
    this.now = input.now ?? (() => new Date());
  }

  /** Returns the processes this message changed. Feed it live messages only, never history. */
  observe(message: SDKMessage): AgentBackgroundProcess[] {
    if (message.type === "user") return this.observeToolResults(message.message?.content);
    if (message.type !== "system") return [];
    const record = message as unknown as Record<string, unknown>;
    switch (record.subtype) {
      case "task_started":
        return this.observeStarted(record);
      case "task_updated":
        return this.observeUpdated(record);
      case "task_notification":
        return this.observeNotification(record);
      default:
        return [];
    }
  }

  /** The Claude process exited, and every shell and monitor it ran died with it. */
  stopRunning(): AgentBackgroundProcess[] {
    const changed: AgentBackgroundProcess[] = [];
    for (const process of this.processes.values()) {
      if (process.status !== "running") continue;
      changed.push(this.update(process, { status: "stopped", endedAt: this.now().toISOString() }));
    }
    return changed;
  }

  private observeStarted(record: Record<string, unknown>): AgentBackgroundProcess[] {
    const id = readString(record.task_id);
    const taskType = readString(record.task_type);
    if (!id || !taskType) return [];
    if (taskType !== BASH_TASK_TYPE && !MONITOR_TASK_TYPES.has(taskType)) return [];
    const toolUseId = readString(record.tool_use_id);
    if (this.isOwnedBySubagent(id, toolUseId)) return [];
    const toolUse = toolUseId ? this.lookupToolUse(toolUseId) : undefined;
    const isMonitor = MONITOR_TASK_TYPES.has(taskType) || toolUse?.name === MONITOR_TOOL_NAME;
    const process: AgentBackgroundProcess = {
      id,
      kind: isMonitor ? "monitor" : "bash",
      description: readString(record.description) ?? toolUse?.command ?? id,
      command: toolUse?.command ?? null,
      status: "running",
      toolCallId: toolUseId ?? null,
      startedAt: this.now().toISOString(),
      endedAt: null,
      exitCode: null,
      outputFile: null,
      tail: [],
    };
    this.processes.set(id, process);
    if (toolUseId) this.processIdByToolUseId.set(toolUseId, id);
    return [process];
  }

  private observeUpdated(record: Record<string, unknown>): AgentBackgroundProcess[] {
    const process = this.processes.get(readString(record.task_id) ?? "");
    const patch = record.patch as { status?: unknown; end_time?: unknown } | undefined;
    if (!process || !patch) return [];
    const status = mapStatus(patch.status);
    const endedAt =
      typeof patch.end_time === "number" ? new Date(patch.end_time).toISOString() : undefined;
    if (!status && !endedAt) return [];
    return [this.update(process, { status: status ?? process.status, endedAt })];
  }

  private observeNotification(record: Record<string, unknown>): AgentBackgroundProcess[] {
    const process = this.processes.get(readString(record.task_id) ?? "");
    if (!process) return [];
    const exitCode = EXIT_CODE_PATTERN.exec(readString(record.summary) ?? "")?.[1];
    return [
      this.update(process, {
        status: mapStatus(record.status) ?? process.status,
        endedAt: process.endedAt ?? this.now().toISOString(),
        outputFile: readString(record.output_file) ?? process.outputFile,
        exitCode: exitCode === undefined ? process.exitCode : Number(exitCode),
      }),
    ];
  }

  private observeToolResults(content: unknown): AgentBackgroundProcess[] {
    if (!Array.isArray(content)) return [];
    const changed: AgentBackgroundProcess[] = [];
    for (const block of content) {
      if (!block || typeof block !== "object" || block.type !== "tool_result") continue;
      const processId = this.processIdByToolUseId.get(readString(block.tool_use_id) ?? "");
      const process = processId ? this.processes.get(processId) : undefined;
      if (!process || process.outputFile) continue;
      const outputFile = OUTPUT_PATH_PATTERN.exec(readToolResultText(block.content))?.[1];
      if (outputFile) changed.push(this.update(process, { outputFile }));
    }
    return changed;
  }

  private update(
    process: AgentBackgroundProcess,
    patch: Partial<AgentBackgroundProcess>,
  ): AgentBackgroundProcess {
    const next: AgentBackgroundProcess = {
      ...process,
      ...Object.fromEntries(Object.entries(patch).filter(([, value]) => value !== undefined)),
    };
    this.processes.set(next.id, next);
    return next;
  }
}
