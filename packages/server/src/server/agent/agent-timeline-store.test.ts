import { describe, expect, it } from "vitest";
import { timelineItemIdentity } from "@getpaseo/protocol/timeline-identity";
import type { AgentTimelineItem } from "./agent-sdk-types.js";
import { InMemoryAgentTimelineStore, type TimelineHistoryReader } from "./agent-timeline-store.js";
import type { AgentTimelineFetchResult, AgentTimelineRow } from "./agent-timeline-store-types.js";

function toolCall(callId: string, status: "running" | "completed"): AgentTimelineItem {
  return {
    type: "tool_call",
    callId,
    name: "Read",
    status,
    detail: { type: "unknown", input: null, output: status === "completed" ? "done" : null },
    error: null,
  };
}

/**
 * About 600 rows: prompts, assistant runs of 3 chunks, reasoning runs, short tool calls, and one
 * tool call that starts near the top and completes near the end.
 */
function longTimeline(): AgentTimelineRow[] {
  const items: Array<{ item: AgentTimelineItem; turnId: string }> = [];
  for (let turn = 0; turn < 30; turn++) {
    const turnId = `turn-${turn}`;
    items.push({ item: { type: "user_message", text: `prompt ${turn}` }, turnId });
    if (turn === 0) items.push({ item: toolCall("long", "running"), turnId });
    items.push({ item: { type: "reasoning", text: `think ${turn} ` }, turnId });
    items.push({ item: { type: "reasoning", text: "more" }, turnId });
    for (let step = 0; step < 4; step++) {
      items.push({ item: toolCall(`call-${turn}-${step}`, "running"), turnId });
      items.push({ item: { type: "assistant_message", text: `a${turn}.${step} ` }, turnId });
      items.push({ item: { type: "assistant_message", text: "b " }, turnId });
      items.push({ item: { type: "assistant_message", text: "c" }, turnId });
      items.push({ item: toolCall(`call-${turn}-${step}`, "completed"), turnId });
    }
    if (turn === 28) items.push({ item: toolCall("long", "completed"), turnId: "turn-0" });
  }
  return items.map(({ item, turnId }, index) => ({
    seq: index + 1,
    timestamp: `2026-01-01T00:00:${String(index % 60).padStart(2, "0")}.000Z`,
    turnId,
    item,
  }));
}

function historyOf(source: readonly AgentTimelineRow[]): TimelineHistoryReader {
  return {
    readRows: (fromSeq, toSeq) => source.filter((row) => row.seq >= fromSeq && row.seq <= toSeq),
    readIdentityRows: (identities, beforeSeq) =>
      source.filter(
        (row) => row.seq < beforeSeq && identities.includes(timelineItemIdentity(row.item) ?? "\0"),
      ),
    latestPromptSeq: () => source.findLast((row) => row.item.type === "user_message")?.seq ?? null,
  };
}

function memorySourceRows(store: InMemoryAgentTimelineStore, agentId: string): number {
  return store
    .getRows(agentId)
    .flatMap((row) => row.sourceSeqRanges)
    .reduce((sum, range) => sum + range.endSeq - range.startSeq + 1, 0);
}

function stores(source: AgentTimelineRow[], tailRows: number) {
  const full = new InMemoryAgentTimelineStore();
  full.initialize("agent", { rows: source, epoch: "epoch" });
  const trimmed = new InMemoryAgentTimelineStore({ tailRows });
  trimmed.initialize("agent", { rows: source, epoch: "epoch" });
  trimmed.trimFlushed("agent", source.length);
  return { full, trimmed, history: historyOf(source) };
}

describe("InMemoryAgentTimelineStore tail", () => {
  it("drops flushed rows below the tail and keeps rows that are not flushed", () => {
    const source = longTimeline();
    const store = new InMemoryAgentTimelineStore({ tailRows: 50 });
    store.initialize("agent", { rows: source, epoch: "epoch" });

    store.trimFlushed("agent", 0);
    expect(memorySourceRows(store, "agent")).toBe(source.length);

    store.trimFlushed("agent", 100);
    expect(Math.min(...store.getRows("agent").map((row) => row.seqEnd))).toBeGreaterThan(100);

    store.trimFlushed("agent", source.length);
    // Memory keeps the 50 tail rows plus older rows only of items that reach into the tail:
    // the long tool call and the item that spans the edge.
    const cutoff = source.length + 1 - 50;
    const kept = store.getRows("agent");
    expect(kept.every((row) => row.seqEnd >= cutoff)).toBe(true);
    const olderRows = kept
      .flatMap((row) => row.sourceSeqRanges)
      .filter((range) => range.startSeq < cutoff)
      .reduce((sum, range) => sum + Math.min(range.endSeq, cutoff - 1) - range.startSeq + 1, 0);
    expect(memorySourceRows(store, "agent") - olderRows).toBe(50);
    expect(olderRows).toBeLessThanOrEqual(5);
  });

  it("pages before from the tail to the first row with every item once", () => {
    const source = longTimeline();
    const { full, trimmed, history } = stores(source, 50);
    const expected = full.fetch("agent", { direction: "tail", limit: 0 }).rows;

    const collected: AgentTimelineFetchResult["rows"] = [];
    let page = trimmed.fetch("agent", { direction: "tail", limit: 7 }, history);
    collected.unshift(...page.rows);
    while (page.hasOlder) {
      const cursor = { epoch: "epoch", seq: page.startSeq! };
      page = trimmed.fetch("agent", { direction: "before", cursor, limit: 7 }, history);
      expect(page.rows.length).toBeGreaterThan(0);
      collected.unshift(...page.rows);
    }

    expect(collected).toEqual(expected);
  });

  it("pages after from the first row to the end with the same final items", () => {
    const source = longTimeline();
    const { full, trimmed, history } = stores(source, 50);
    const expected = full.fetch("agent", { direction: "tail", limit: 0 }).rows;

    const bySeqStart = new Map<number, AgentTimelineFetchResult["rows"][number]>();
    let cursorSeq = 0;
    for (let pages = 0; pages < 500; pages++) {
      const cursor = { epoch: "epoch", seq: cursorSeq };
      const page = trimmed.fetch("agent", { direction: "after", cursor, limit: 7 }, history);
      for (const row of page.rows) bySeqStart.set(row.seqStart, row);
      if (!page.hasNewer) break;
      cursorSeq = page.endSeq!;
    }

    expect([...bySeqStart.values()].sort((a, b) => a.seqStart - b.seqStart)).toEqual(expected);
  });

  it("widens the tail to a latest prompt that sits below the tail edge", () => {
    const source = longTimeline().slice(0, 560);
    const prompt = source.findLast((row) => row.item.type === "user_message")!;
    const promptSeq = prompt.seq;
    const tailRows = source.length - promptSeq - 3;
    const { full, trimmed, history } = stores(source, tailRows);
    expect(trimmed.getRows("agent").some((row) => row.item.type === "user_message")).toBe(false);

    const options = { direction: "tail" as const, limit: 2, includeLatestPrompt: true };
    const page = trimmed.fetch("agent", options, history);

    expect(page.rows[0]?.item).toEqual(prompt.item);
    expect(page.rows.length).toBeGreaterThan(2);
    expect(page).toEqual(full.fetch("agent", options));
  });

  it("serves pages within the tail from memory and keeps whole timelines without a trim", () => {
    const source = longTimeline();
    const { full, trimmed } = stores(source, 50);
    const failing: TimelineHistoryReader = {
      readRows: () => {
        throw new Error("no history");
      },
      readIdentityRows: () => {
        throw new Error("no history");
      },
      latestPromptSeq: () => {
        throw new Error("no history");
      },
    };

    const tail = trimmed.fetch("agent", { direction: "tail", limit: 3 }, failing);
    expect(tail.rows).toEqual(full.fetch("agent", { direction: "tail", limit: 3 }).rows);
    expect(tail.hasOlder).toBe(true);
    expect(() => trimmed.fetch("agent", { direction: "tail", limit: 0 })).toThrow(
      /before its in-memory tail is unavailable/,
    );

    const untrimmed = new InMemoryAgentTimelineStore({ tailRows: 50 });
    untrimmed.initialize("agent", { rows: source, epoch: "epoch" });
    expect(memorySourceRows(untrimmed, "agent")).toBe(source.length);
    expect(untrimmed.fetch("agent", { direction: "tail", limit: 0 }, failing)).toEqual(
      full.fetch("agent", { direction: "tail", limit: 0 }),
    );
  });
});

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
