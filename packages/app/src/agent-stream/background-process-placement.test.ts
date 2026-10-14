import { describe, expect, it } from "vitest";
import type { AgentBackgroundProcess } from "@getpaseo/protocol/messages";
import type { TurnStepGroup } from "@/tool-calls/detail-level/turn/model";
import type { StreamItem, ToolCallItem, UserMessageItem } from "@/types/stream";
import { placeBackgroundProcesses } from "./background-process-placement";

const at = new Date(Date.UTC(2026, 9, 14));

function call(callId: string): ToolCallItem {
  return {
    kind: "tool_call",
    id: callId,
    timestamp: at,
    payload: {
      source: "agent",
      data: {
        provider: "claude",
        callId,
        name: "Bash",
        status: "completed",
        error: null,
        detail: { type: "unknown", input: null, output: null },
      },
    },
  };
}

function user(id: string): UserMessageItem {
  return { kind: "user_message", id, text: "go", timestamp: at };
}

function process(
  id: string,
  toolCallId: string,
  status: AgentBackgroundProcess["status"],
): AgentBackgroundProcess {
  return {
    id,
    kind: "bash",
    description: id,
    command: null,
    status,
    toolCallId,
    startedAt: at.toISOString(),
    endedAt: status === "running" ? null : at.toISOString(),
    exitCode: null,
    outputFile: null,
    tail: [],
  };
}

/** Each host row hosts the turn group of the calls listed for it. */
function stream(turns: Record<string, string[]>, trailing: StreamItem[] = []) {
  const rows: StreamItem[] = [];
  const groups = new Map<string, TurnStepGroup>();
  for (const [hostId, callIds] of Object.entries(turns)) {
    rows.push(user(`user-${hostId}`));
    const calls = callIds.map(call);
    rows.push(...calls.slice(0, -1), { ...calls[calls.length - 1]!, id: hostId });
    groups.set(hostId, { run: { items: calls } } as unknown as TurnStepGroup);
  }
  rows.push(...trailing);
  return { rows, groupsByHostId: groups };
}

describe("placeBackgroundProcesses", () => {
  it("shows a running process in the newest turn and marks where it started", () => {
    const placement = placeBackgroundProcesses({
      ...stream({ turn7: ["bash-call"], turn8: ["read-call"] }),
      processes: [process("p1", "bash-call", "running")],
    });
    expect(placement.byHostId.get("turn8")?.processes.map((p) => p.id)).toEqual(["p1"]);
    expect(placement.byHostId.get("turn7")).toEqual({ processes: [], movedCount: 1 });
    expect(placement.unplaced).toEqual([]);
  });

  it("returns a finished process to the turn that started it", () => {
    const placement = placeBackgroundProcesses({
      ...stream({ turn7: ["bash-call"], turn8: ["read-call"] }),
      processes: [process("p1", "bash-call", "completed")],
    });
    expect(placement.byHostId.get("turn7")?.processes.map((p) => p.id)).toEqual(["p1"]);
    expect(placement.byHostId.has("turn8")).toBe(false);
  });

  it("puts a running process at the bottom when the newest turn has no step card", () => {
    const placement = placeBackgroundProcesses({
      ...stream({ turn7: ["bash-call"] }, [user("latest")]),
      processes: [process("p1", "bash-call", "running")],
    });
    expect(placement.unplaced.map((p) => p.id)).toEqual(["p1"]);
    expect(placement.byHostId.get("turn7")).toEqual({ processes: [], movedCount: 1 });
  });
});
