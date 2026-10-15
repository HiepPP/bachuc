import { describe, expect, it } from "vitest";
import { InMemoryAgentTimelineStore } from "./agent-timeline-store.js";

describe("InMemoryAgentTimelineStore", () => {
  it("keeps turn metrics per turn and clears them on delete and initialize", () => {
    const store = new InMemoryAgentTimelineStore();
    store.initialize("agent-1");
    store.setTurnMetrics("agent-1", "turn-1", { ttftMs: 250, outputTokens: 40 });
    store.setTurnMetrics("agent-1", "turn-2", { ttftMs: 90 });

    expect(store.getTurnMetrics("agent-1", "turn-1")).toEqual({ ttftMs: 250, outputTokens: 40 });
    expect(store.getTurnMetrics("agent-1", "turn-2")).toEqual({ ttftMs: 90 });
    expect(store.getTurnMetrics("agent-1", "turn-3")).toBeUndefined();

    store.initialize("agent-1");
    expect(store.getTurnMetrics("agent-1", "turn-1")).toBeUndefined();

    store.setTurnMetrics("agent-1", "turn-1", { ttftMs: 10 });
    store.delete("agent-1");
    expect(store.getTurnMetrics("agent-1", "turn-1")).toBeUndefined();
  });

  it("clamps an overshooting before cursor into the bounded tail window", () => {
    const store = new InMemoryAgentTimelineStore();
    store.initialize("agent-1", {
      epoch: "epoch-1",
      nextSeq: 8,
      rows: [
        {
          seq: 5,
          timestamp: "2026-01-01T00:00:00.000Z",
          item: { type: "assistant_message", text: "five", messageId: "five" },
        },
        {
          seq: 6,
          timestamp: "2026-01-01T00:00:01.000Z",
          item: { type: "assistant_message", text: "six", messageId: "six" },
        },
        {
          seq: 7,
          timestamp: "2026-01-01T00:00:02.000Z",
          item: { type: "assistant_message", text: "seven", messageId: "seven" },
        },
      ],
    });

    const result = store.fetch("agent-1", {
      direction: "before",
      cursor: { epoch: "epoch-1", seq: 100 },
      limit: 2,
    });

    expect(result).toMatchObject({
      epoch: "epoch-1",
      direction: "before",
      reset: false,
      staleCursor: false,
      gap: false,
      window: { minSeq: 5, maxSeq: 7, nextSeq: 8 },
      hasOlder: true,
      hasNewer: false,
      rows: [
        {
          seq: 6,
          timestamp: "2026-01-01T00:00:01.000Z",
          item: { type: "assistant_message", text: "six", messageId: "six" },
        },
        {
          seq: 7,
          timestamp: "2026-01-01T00:00:02.000Z",
          item: { type: "assistant_message", text: "seven", messageId: "seven" },
        },
      ],
    });
  });

  it("returns a bounded reset window when an after cursor is behind retained history", () => {
    const store = new InMemoryAgentTimelineStore();
    store.initialize("agent-1", {
      epoch: "epoch-1",
      nextSeq: 8,
      rows: [
        {
          seq: 5,
          timestamp: "2026-01-01T00:00:00.000Z",
          item: { type: "assistant_message", text: "five", messageId: "five" },
        },
        {
          seq: 6,
          timestamp: "2026-01-01T00:00:01.000Z",
          item: { type: "assistant_message", text: "six", messageId: "six" },
        },
        {
          seq: 7,
          timestamp: "2026-01-01T00:00:02.000Z",
          item: { type: "assistant_message", text: "seven", messageId: "seven" },
        },
      ],
    });

    const result = store.fetch("agent-1", {
      direction: "after",
      cursor: { epoch: "epoch-1", seq: 1 },
      limit: 1,
    });

    expect(result).toMatchObject({
      epoch: "epoch-1",
      direction: "after",
      reset: true,
      staleCursor: false,
      gap: true,
      window: { minSeq: 5, maxSeq: 7, nextSeq: 8 },
      hasOlder: true,
      hasNewer: false,
      rows: [
        {
          seq: 7,
          timestamp: "2026-01-01T00:00:02.000Z",
          item: { type: "assistant_message", text: "seven", messageId: "seven" },
        },
      ],
    });
  });
});

describe("projected timeline retention", () => {
  it("retains one full tool state while streaming every source update", () => {
    const store = new InMemoryAgentTimelineStore();
    store.initialize("agent");
    for (let seq = 1; seq <= 2000; seq++) {
      const item = {
        type: "tool_call" as const,
        callId: "child",
        name: "task",
        status: "running" as const,
        error: null,
        detail: { type: "plain_text" as const, label: "Child", text: "x".repeat(seq * 128) },
      };
      expect(store.append("agent", item)).toMatchObject({ seq, item });
    }
    const result = store.fetch("agent", { limit: 0 });
    expect(result.rows).toHaveLength(1);
    expect(JSON.stringify(result.rows).length).toBeLessThan(260_000);
    expect(result.window.maxSeq).toBe(2000);
  });

  it("catches up a mid-message cursor with the complete projected message", () => {
    const store = new InMemoryAgentTimelineStore();
    store.initialize("agent", { epoch: "e" });
    store.append("agent", { type: "assistant_message", messageId: "m", text: "A" });
    store.append("agent", { type: "assistant_message", messageId: "m", text: "B" });
    expect(
      store.fetch("agent", { direction: "after", cursor: { epoch: "e", seq: 1 } }).rows,
    ).toMatchObject([
      {
        item: { text: "AB" },
        seqStart: 1,
        seqEnd: 2,
        sourceSeqRanges: [{ startSeq: 1, endSeq: 2 }],
      },
    ]);
  });
});

describe("projected sequence ownership", () => {
  it("preserves fetched coverage when another source chunk arrives", () => {
    const store = new InMemoryAgentTimelineStore();
    store.initialize("a");
    store.append("a", { type: "assistant_message", text: "A" });
    const before = store.fetch("a");
    store.append("a", { type: "assistant_message", text: "B" });
    expect(before.rows[0]).toMatchObject({
      item: { text: "A" },
      seqEnd: 1,
      sourceSeqRanges: [{ startSeq: 1, endSeq: 1 }],
    });
    expect(store.fetch("a").rows[0]).toMatchObject({
      item: { text: "AB" },
      seqEnd: 2,
      sourceSeqRanges: [{ startSeq: 1, endSeq: 2 }],
    });
  });
  it("preserves source positions when seeding a projected history whose last update is an earlier tool", () => {
    const store = new InMemoryAgentTimelineStore();
    store.initialize("a");
    const tool = {
      type: "tool_call" as const,
      callId: "t",
      name: "shell",
      error: null,
      detail: { type: "plain_text" as const, label: "work" },
    };
    store.append("a", { ...tool, status: "running" });
    store.append("a", { type: "assistant_message", text: "Answer" });
    store.append("a", { ...tool, status: "completed" });
    store.initialize("b", { rows: store.getRows("a") });
    expect(store.append("b", { type: "user_message", text: "next" }).seq).toBe(4);
    expect(store.fetch("b").rows.map((row) => row.seqStart)).toEqual([1, 2, 4]);
  });
  it("returns the last projected assistant message without joining different messages", () => {
    const store = new InMemoryAgentTimelineStore();
    store.initialize("a");
    store.append("a", { type: "assistant_message", messageId: "first", text: "First" });
    store.append("a", { type: "assistant_message", messageId: "second", text: "Sec" });
    store.append("a", { type: "assistant_message", messageId: "second", text: "ond" });
    expect(store.getLastAssistantMessage("a")).toBe("Second");
  });
});

it("includes transitive tool updates in a contiguous projected tail", () => {
  const store = new InMemoryAgentTimelineStore();
  store.initialize("a");
  const tool = (callId: string, status: "running" | "completed") => ({
    type: "tool_call" as const,
    callId,
    name: "shell",
    status,
    error: null,
    detail: { type: "plain_text" as const, label: "work" },
  });
  store.append("a", tool("first", "running"));
  store.append("a", tool("second", "running"));
  store.append("a", { type: "user_message", text: "Continue" });
  store.append("a", tool("first", "completed"));
  store.append("a", { type: "assistant_message", text: "Answer" });
  store.append("a", tool("second", "completed"));
  const tail = store.fetch("a", { limit: 1 });
  expect(tail.rows.map((row) => row.seqStart)).toEqual([1, 2, 3, 5]);
  expect(tail).toMatchObject({ startSeq: 1, endSeq: 6, hasOlder: false });
});

describe("latest prompt tail", () => {
  function storeWithPromptThenErrors(input: { errorsAfterPrompt: number }) {
    const store = new InMemoryAgentTimelineStore();
    store.initialize("a", { epoch: "epoch-1" });
    store.append("a", { type: "error", message: "before" });
    store.append("a", { type: "user_message", text: "Prompt", messageId: "prompt" });
    for (let index = 0; index < input.errorsAfterPrompt; index++) {
      store.append("a", { type: "error", message: `after-${index}` });
    }
    return store;
  }

  it("widens the tail to the latest user message", () => {
    const store = storeWithPromptThenErrors({ errorsAfterPrompt: 5 });

    const page = store.fetch("a", { direction: "tail", limit: 2, includeLatestPrompt: true });

    expect(page.rows.map((row) => row.item.type)).toEqual([
      "user_message",
      "error",
      "error",
      "error",
      "error",
      "error",
    ]);
    expect(page).toMatchObject({ startSeq: 2, endSeq: 7, hasOlder: true, hasNewer: false });
  });

  it("returns the normal tail when the prompt is more than 200 items back", () => {
    const store = storeWithPromptThenErrors({ errorsAfterPrompt: 200 });

    const page = store.fetch("a", { direction: "tail", limit: 3, includeLatestPrompt: true });

    expect(page.rows.map((row) => row.seq)).toEqual([200, 201, 202]);
    expect(page.hasOlder).toBe(true);
  });

  it("widens up to exactly 200 items", () => {
    const store = storeWithPromptThenErrors({ errorsAfterPrompt: 199 });

    const page = store.fetch("a", { direction: "tail", limit: 3, includeLatestPrompt: true });

    expect(page.rows).toHaveLength(200);
    expect(page.rows[0]?.item).toEqual({
      type: "user_message",
      text: "Prompt",
      messageId: "prompt",
    });
  });

  it("returns the normal tail when no user message exists", () => {
    const store = new InMemoryAgentTimelineStore();
    store.initialize("a", { epoch: "epoch-1" });
    for (let index = 0; index < 5; index++) {
      store.append("a", { type: "error", message: `error-${index}` });
    }

    const page = store.fetch("a", { direction: "tail", limit: 2, includeLatestPrompt: true });

    expect(page.rows.map((row) => row.seq)).toEqual([4, 5]);
    expect(page.hasOlder).toBe(true);
  });
});
