import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import MarkdownIt from "markdown-it";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { AgentTimelineItem } from "@getpaseo/protocol/agent-types";
import { createTestLogger } from "../../../test-utils/test-logger.js";
import type {
  AgentTimelineRow,
  DurableTimelineSearchCandidate,
} from "../agent-timeline-store-types.js";
import { openTimelineCacheStore, type SqliteTimelineStore } from "../sqlite-timeline-store.js";
import { searchCandidates, searchTimeline } from "./index.js";

function rows(items: AgentTimelineItem[]) {
  return items.map((item, index) => ({ item, seq: index + 1, timestamp: "2026-09-12T00:00:00Z" }));
}

const homes: string[] = [];
const stores: SqliteTimelineStore[] = [];

afterEach(async () => {
  for (const store of stores.splice(0)) store.close();
  for (const home of homes.splice(0)) await rm(home, { recursive: true, force: true });
  vi.restoreAllMocks();
});

async function storeWith(source: readonly AgentTimelineRow[]): Promise<SqliteTimelineStore> {
  const home = await mkdtemp(path.join(tmpdir(), "chat-search-"));
  homes.push(home);
  const store = await openTimelineCacheStore(home, createTestLogger());
  if (!store) throw new Error("Timeline cache store did not open");
  stores.push(store);
  store.reset("agent", "epoch");
  store.append("agent", source);
  return store;
}

/** Pages candidates the way AgentManager.searchTimeline does. */
function* storeCandidates(
  store: SqliteTimelineStore,
  query: string,
  cursor: number,
): Generator<DurableTimelineSearchCandidate> {
  let afterSeq = cursor;
  for (;;) {
    const page = store.findSearchCandidates("agent", query, afterSeq, 50) ?? [];
    yield* page;
    if (page.length < 50) return;
    afterSeq = page[page.length - 1]!.seq;
  }
}

async function searchStore(store: SqliteTimelineStore, query: string, cursor = 0) {
  return searchCandidates({
    candidates: storeCandidates(store, query, cursor),
    query,
    cursor,
    foldDiacritics: true,
  });
}

describe("FTS candidates", () => {
  it("returns the same hits as the in-memory search, with a word split across chunks", async () => {
    const source: AgentTimelineRow[] = [
      { seq: 1, timestamp: "t", item: { type: "user_message", text: "find the target please" } },
      {
        seq: 2,
        timestamp: "t",
        turnId: "turn-1",
        item: { type: "assistant_message", text: "a tar" },
      },
      {
        seq: 3,
        timestamp: "t",
        turnId: "turn-1",
        item: { type: "assistant_message", text: "get and **target**" },
      },
      { seq: 4, timestamp: "t", turnId: "turn-1", item: { type: "reasoning", text: "target" } },
      {
        seq: 5,
        timestamp: "t",
        turnId: "turn-1",
        item: { type: "assistant_message", text: "no match here" },
      },
      {
        seq: 6,
        timestamp: "t",
        turnId: "turn-2",
        item: { type: "assistant_message", text: "```\ntarget\n```" },
      },
      { seq: 7, timestamp: "t", item: { type: "user_message", text: "hello &amp; world" } },
      {
        seq: 8,
        timestamp: "t",
        turnId: "turn-3",
        item: { type: "assistant_message", text: "hello &amp; **world**" },
      },
    ];
    const store = await storeWith(source);

    for (const query of ["target", "a target", "hello & world", "TARGET please", "missing"]) {
      expect(await searchStore(store, query)).toEqual(
        await searchTimeline({ rows: source, query }),
      );
    }
    expect((await searchStore(store, "target")).locations).toEqual([
      { seq: 1, role: "user", count: 1 },
      { seq: 3, role: "assistant", count: 2 },
      { seq: 6, role: "assistant", count: 1 },
    ]);
  });

  it("keeps paging parity across the 200-location page", async () => {
    const source = rows(
      Array.from({ length: 202 }, () => ({ type: "user_message", text: "target" })),
    );
    const store = await storeWith(source);
    const first = await searchStore(store, "target");
    expect(first).toEqual(await searchTimeline({ rows: source, query: "target" }));
    const second = await searchStore(store, "target", first.nextCursor!);
    expect(second).toEqual(
      await searchTimeline({ rows: source, query: "target", cursor: first.nextCursor! }),
    );
    expect(second.locations.map((location) => location.seq)).toEqual([201, 202]);
  });

  it("finds Vietnamese text with diacritics from a query without them", async () => {
    const store = await storeWith(
      rows([
        { type: "user_message", text: "Tiếng Việt trên đường phố" },
        { type: "assistant_message", text: "Đường **dài** lắm" },
        { type: "user_message", text: "tieng viet" },
      ]),
    );

    expect((await searchStore(store, "tieng viet")).locations).toEqual([
      { seq: 1, role: "user", count: 1 },
      { seq: 3, role: "user", count: 1 },
    ]);
    expect((await searchStore(store, "duong")).locations).toEqual([
      { seq: 1, role: "user", count: 1 },
      { seq: 2, role: "assistant", count: 1 },
    ]);
    // A query typed with diacritics keeps exact matching.
    expect((await searchStore(store, "tiếng việt")).locations).toEqual([
      { seq: 1, role: "user", count: 1 },
    ]);
  });

  it("parses only candidate messages with markdown-it on a 5,000-message timeline", async () => {
    const source: AgentTimelineRow[] = Array.from({ length: 5_000 }, (_, index) => ({
      seq: index + 1,
      timestamp: "t",
      item: {
        type: "assistant_message",
        text: index % 1_000 === 7 ? "the needle is here" : `filler message ${index}`,
        messageId: `message-${index}`,
      },
    }));
    const store = await storeWith(source);
    store.flush();
    const parse = vi.spyOn(MarkdownIt.prototype, "parse");

    const result = await searchStore(store, "needle");

    expect(result.locations).toHaveLength(5);
    // Parse counter for the TASK-128 anti-goal: candidates only, not all 5,000 messages.
    expect(parse).toHaveBeenCalledTimes(5);
  });
});

it("returns source locations for formatted assistant text and literal user text", async () => {
  const result = await searchTimeline({
    rows: rows([
      { type: "user_message", text: "hello **world**" },
      { type: "assistant_message", text: "İ😀hello **world** and hello *world*" },
      {
        type: "assistant_message",
        text: "[hello &amp; world](https://example.com)",
        messageId: "separate",
      },
    ]),
    query: "hello world",
  });
  expect(result).toEqual({
    locations: [{ seq: 2, role: "assistant", count: 2 }],
    nextCursor: null,
  });
});

it("finds code and literal punctuation without searching tools, reasoning, or task state", async () => {
  const result = await searchTimeline({
    rows: rows([
      { type: "assistant_message", text: "```ts\nconst value = 'a.b';\n```" },
      { type: "reasoning", text: "a.b" },
      { type: "todo", items: [{ text: "a.b", completed: false }] },
      { type: "user_message", text: "axb" },
      { type: "user_message", text: "a.b" },
    ]),
    query: "a.b",
  });
  expect(result.locations).toEqual([
    { seq: 1, role: "assistant", count: 1 },
    { seq: 5, role: "user", count: 1 },
  ]);
});

it("bounds results and resumes without losing locations", async () => {
  const source = rows(
    Array.from({ length: 202 }, () => ({ type: "user_message", text: "target" })),
  );
  const first = await searchTimeline({ rows: source, query: "target" });
  expect(first.locations).toHaveLength(200);
  expect(first.nextCursor).toBe(200);
  const second = await searchTimeline({ rows: source, query: "target", cursor: first.nextCursor! });
  expect(second).toEqual({
    locations: [
      { seq: 201, role: "user", count: 1 },
      { seq: 202, role: "user", count: 1 },
    ],
    nextCursor: null,
  });
});

it("finds ordinary Markdown entities and line breaks without requiring identical parsers", async () => {
  const source = rows([{ type: "assistant_message", text: "hello &amp; **world**\nnext line" }]);
  expect((await searchTimeline({ rows: source, query: "hello & world" })).locations).toEqual([
    { seq: 1, role: "assistant", count: 1 },
  ]);
  expect((await searchTimeline({ rows: source, query: "world next" })).locations).toEqual([
    { seq: 1, role: "assistant", count: 1 },
  ]);
});

it("counts occurrences per rendered block rather than across block boundaries", async () => {
  const source = rows([
    { type: "assistant_message", text: "target one\n\ntarget two\n\n- target three\n- end target" },
    { type: "user_message", text: "TARGET target" },
  ]);
  expect((await searchTimeline({ rows: source, query: "target" })).locations).toEqual([
    { seq: 1, role: "assistant", count: 4 },
    { seq: 2, role: "user", count: 2 },
  ]);
  expect((await searchTimeline({ rows: source, query: "end target" })).locations).toEqual([
    { seq: 1, role: "assistant", count: 1 },
  ]);
  expect((await searchTimeline({ rows: source, query: "two target" })).locations).toEqual([]);
  // Rendered text never runs from the end of one block into the start of the next, so
  // neither does a match, however contiguous the Markdown source looks.
  expect((await searchTimeline({ rows: source, query: "one target" })).locations).toEqual([]);
});
