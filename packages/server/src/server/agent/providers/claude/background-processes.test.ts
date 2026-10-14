import type { SDKMessage } from "@anthropic-ai/claude-agent-sdk";
import { describe, expect, it } from "vitest";

import { ClaudeBackgroundProcesses, type BackgroundToolUse } from "./background-processes.js";

const OUTPUT_FILE = "/private/tmp/claude-501/-private-tmp-bgprobe/session/tasks/btokf1ni8.output";

function system(fields: Record<string, unknown>): SDKMessage {
  return { type: "system", uuid: "u", session_id: "s", ...fields } as unknown as SDKMessage;
}

function toolResult(toolUseId: string, content: string): SDKMessage {
  return {
    type: "user",
    message: { role: "user", content: [{ type: "tool_result", tool_use_id: toolUseId, content }] },
    parent_tool_use_id: null,
    session_id: "s",
  } as unknown as SDKMessage;
}

function createTracker(input?: {
  tools?: Record<string, BackgroundToolUse>;
  subagentTaskIds?: string[];
}) {
  return new ClaudeBackgroundProcesses({
    lookupToolUse: (id) => input?.tools?.[id],
    isOwnedBySubagent: (taskId) => input?.subagentTaskIds?.includes(taskId) ?? false,
    now: () => new Date("2026-10-14T10:00:00.000Z"),
  });
}

const started = system({
  subtype: "task_started",
  task_id: "btokf1ni8",
  tool_use_id: "toolu_1",
  description: "Background loop printing ticks",
  is_backgrounded: true,
  task_type: "local_bash",
});

describe("ClaudeBackgroundProcesses", () => {
  it("tracks a backgrounded shell from start to its exit code", () => {
    const tracker = createTracker({
      tools: { toolu_1: { name: "Bash", command: "for i in 1 2 3; do echo tick $i; done" } },
    });

    expect(tracker.observe(started)).toEqual([
      {
        id: "btokf1ni8",
        kind: "bash",
        description: "Background loop printing ticks",
        command: "for i in 1 2 3; do echo tick $i; done",
        status: "running",
        toolCallId: "toolu_1",
        startedAt: "2026-10-14T10:00:00.000Z",
        endedAt: null,
        exitCode: null,
        outputFile: null,
        tail: [],
      },
    ]);

    const [withOutput] = tracker.observe(
      toolResult(
        "toolu_1",
        `Command running in background with ID: btokf1ni8. Output is being written to: ${OUTPUT_FILE}. You will be notified when it completes.`,
      ),
    );
    expect(withOutput?.outputFile).toBe(OUTPUT_FILE);

    const [updated] = tracker.observe(
      system({
        subtype: "task_updated",
        task_id: "btokf1ni8",
        patch: { status: "completed", end_time: Date.parse("2026-10-14T10:00:06.000Z") },
      }),
    );
    expect(updated).toMatchObject({ status: "completed", endedAt: "2026-10-14T10:00:06.000Z" });

    const [notified] = tracker.observe(
      system({
        subtype: "task_notification",
        task_id: "btokf1ni8",
        tool_use_id: "toolu_1",
        status: "completed",
        output_file: OUTPUT_FILE,
        summary: 'Background command "Background loop printing ticks" completed (exit code 0)',
      }),
    );
    expect(notified).toMatchObject({
      status: "completed",
      exitCode: 0,
      endedAt: "2026-10-14T10:00:06.000Z",
    });
  });

  it("labels a Monitor tool call as a monitor", () => {
    const tracker = createTracker({
      tools: { toolu_1: { name: "Monitor", command: "tail -f x" } },
    });
    expect(tracker.observe(started)[0]?.kind).toBe("monitor");
  });

  it("ignores subagents, workflows, and shells owned by a subagent", () => {
    const tracker = createTracker({ subagentTaskIds: ["inner"] });
    expect(
      tracker.observe(system({ subtype: "task_started", task_id: "a", task_type: "local_agent" })),
    ).toEqual([]);
    expect(
      tracker.observe(
        system({ subtype: "task_started", task_id: "inner", task_type: "local_bash" }),
      ),
    ).toEqual([]);
  });

  it("marks running processes stopped when the runtime dies", () => {
    const tracker = createTracker();
    tracker.observe(started);
    expect(tracker.stopRunning()).toMatchObject([{ id: "btokf1ni8", status: "stopped" }]);
    expect(tracker.stopRunning()).toEqual([]);
  });
});
