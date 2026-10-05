import { test } from "node:test";
import assert from "node:assert/strict";
import type { AgentTimelineItem } from "@getpaseo/protocol/agent-types";
import { toEditRow } from "../shared/edit-item";

type ToolCallItem = Extract<AgentTimelineItem, { type: "tool_call" }>;

function toolCall(detail: ToolCallItem["detail"]): ToolCallItem {
  return {
    type: "tool_call",
    callId: "call-1",
    name: "tool",
    status: "completed",
    detail,
    error: null,
  } as ToolCallItem;
}

test("write calls become write rows and edit calls stay edit rows", () => {
  assert.deepEqual(toEditRow(toolCall({ type: "write", filePath: "/a.ts", content: "x" })), {
    action: "write",
    filePath: "/a.ts",
    content: "x",
    status: "completed",
  });
  assert.equal(
    toEditRow(toolCall({ type: "edit", filePath: "/a.ts", newString: "y" }))?.action,
    "edit",
  );
});

test("shell and read calls keep their native rows", () => {
  assert.equal(toEditRow(toolCall({ type: "shell", command: "ls" })), undefined);
  assert.equal(toEditRow(toolCall({ type: "read", filePath: "/a.ts" })), undefined);
});
