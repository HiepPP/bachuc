import type { AgentTimelineItem } from "@getpaseo/protocol/agent-types";
import type { EditRowData } from "./contracts";

type ToolCallItem = Extract<AgentTimelineItem, { type: "tool_call" }>;

function errorText(error: unknown): string | undefined {
  if (error === null || error === undefined) return undefined;
  if (typeof error === "string") return error;
  if (typeof error === "object" && "message" in error && typeof error.message === "string") {
    return error.message;
  }
  try {
    return JSON.stringify(error);
  } catch {
    return String(error);
  }
}

/** Plugin row data for an edit or write tool call; undefined keeps every other tool call native. */
export function toEditRow(item: ToolCallItem): EditRowData | undefined {
  const { detail } = item;
  if (detail.type !== "edit" && detail.type !== "write") return undefined;
  const error = errorText(item.error);
  const tail = { status: item.status, ...(error !== undefined ? { error } : {}) };
  if (detail.type === "write") {
    return {
      action: "write",
      filePath: detail.filePath,
      ...(detail.content !== undefined ? { content: detail.content } : {}),
      ...tail,
    };
  }
  return {
    action: "edit",
    filePath: detail.filePath,
    ...(detail.oldString !== undefined ? { oldString: detail.oldString } : {}),
    ...(detail.newString !== undefined ? { newString: detail.newString } : {}),
    ...(detail.unifiedDiff !== undefined ? { unifiedDiff: detail.unifiedDiff } : {}),
    ...tail,
  };
}
