import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { describe, expect, it } from "vitest";
import { createTestLogger } from "../../test-utils/test-logger.js";
import { InMemoryAgentTimelineStore } from "./agent-timeline-store.js";
import type { AgentTimelineRow } from "./agent-timeline-store-types.js";
import { openTimelineCacheStore } from "./sqlite-timeline-store.js";
import { buildTimelinePromptIndex } from "./timeline-prompt-index.js";

describe("buildTimelinePromptIndex", () => {
  it("builds the same list from cached prompt rows as from the in-memory timeline", async () => {
    const rows: AgentTimelineRow[] = [
      {
        seq: 1,
        timestamp: "2026-01-01T00:00:00.000Z",
        item: { type: "user_message", text: "one" },
      },
      {
        seq: 2,
        timestamp: "2026-01-01T00:00:01.000Z",
        turnId: "turn-1",
        item: { type: "assistant_message", text: "chunk " },
      },
      {
        seq: 3,
        timestamp: "2026-01-01T00:00:02.000Z",
        turnId: "turn-1",
        item: {
          type: "tool_call",
          callId: "call-1",
          name: "Read",
          status: "running",
          detail: { type: "unknown", input: null, output: null },
          error: null,
        },
      },
      {
        seq: 4,
        timestamp: "2026-01-01T00:00:03.000Z",
        item: { type: "user_message", text: "two\n\n  lines", messageId: "user-2" },
        providerMessageId: "provider-2",
      },
    ];
    const memory = new InMemoryAgentTimelineStore();
    memory.initialize("agent", { rows, epoch: "epoch-1" });
    const home = await mkdtemp(path.join(tmpdir(), "timeline-prompt-index-"));
    const store = await openTimelineCacheStore(home, createTestLogger());
    if (!store) throw new Error("Timeline cache store did not open");
    try {
      store.reset("agent", "epoch-1");
      store.append("agent", rows);

      const fromCache = buildTimelinePromptIndex("epoch-1", store.listPromptRows("agent") ?? []);
      expect(fromCache).toEqual(buildTimelinePromptIndex("epoch-1", memory.getRows("agent")));
      expect(fromCache.prompts.map((prompt) => prompt.seq)).toEqual([1, 4]);
    } finally {
      store.close();
      await rm(home, { recursive: true, force: true });
    }
  });

  it("indexes every canonical user prompt with a stable timeline position", () => {
    const rows: AgentTimelineRow[] = [
      {
        seq: 3,
        timestamp: "2026-01-01T00:00:00.000Z",
        item: { type: "user_message", text: "  First\n\n   prompt  " },
      },
      {
        seq: 4,
        timestamp: "2026-01-01T00:00:01.000Z",
        item: { type: "assistant_message", text: "response" },
      },
      {
        seq: 8,
        timestamp: "2026-01-01T00:00:02.000Z",
        item: { type: "user_message", text: "Second prompt" },
      },
    ];

    expect(buildTimelinePromptIndex("epoch-1", rows)).toEqual({
      epoch: "epoch-1",
      prompts: [
        { seq: 3, timestamp: "2026-01-01T00:00:00.000Z", preview: "First prompt" },
        { seq: 8, timestamp: "2026-01-01T00:00:02.000Z", preview: "Second prompt" },
      ],
    });
  });

  it("bounds previews without indexing assistant rows", () => {
    const rows: AgentTimelineRow[] = [
      {
        seq: 1,
        timestamp: "2026-01-01T00:00:00.000Z",
        item: { type: "user_message", text: "x".repeat(200) },
      },
      {
        seq: 2,
        timestamp: "2026-01-01T00:00:01.000Z",
        item: { type: "assistant_message", text: "ignored" },
      },
    ];

    const result = buildTimelinePromptIndex("epoch-1", rows);

    expect(result.prompts).toHaveLength(1);
    expect(result.prompts[0]?.preview).toHaveLength(120);
    expect(result.prompts[0]?.preview.endsWith("…")).toBe(true);
  });
});
