import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { createTestLogger } from "../../test-utils/test-logger.js";
import type { AgentTimelineRow } from "./agent-timeline-store-types.js";
import type { ProviderSubagentDescriptor } from "./provider-subagents/store.js";
import { openTimelineCacheStore, type SqliteTimelineStore } from "./sqlite-timeline-store.js";

// @types/node@20 has no node:sqlite typings; declare the slice the test uses.
interface RawStatement {
  get(...params: unknown[]): Record<string, unknown> | undefined;
}
interface RawDatabase {
  exec(sql: string): void;
  prepare(sql: string): RawStatement;
  close(): void;
}
interface RawSqliteModule {
  DatabaseSync: new (path: string) => RawDatabase;
}

async function openRaw(file: string): Promise<RawDatabase> {
  const specifier: string = "node:sqlite";
  const sqlite = (await import(specifier)) as unknown as RawSqliteModule;
  return new sqlite.DatabaseSync(file);
}

function countRows(db: RawDatabase, agentId: string): number {
  const count = db
    .prepare("SELECT COUNT(*) AS n FROM timeline_rows WHERE agent_id = ?")
    .get(agentId);
  return Number(count?.["n"]);
}

function nextTick(): Promise<void> {
  return new Promise((resolve) => setImmediate(resolve));
}

function row(seq: number, extra?: Partial<AgentTimelineRow>): AgentTimelineRow {
  return {
    seq,
    timestamp: `2026-10-15T00:00:${String(seq % 60).padStart(2, "0")}.000Z`,
    item: { type: "assistant_message", text: `row ${seq}`, messageId: `m-${seq}` },
    ...extra,
  };
}

const descriptor: ProviderSubagentDescriptor = {
  id: "sub-1",
  parentAgentId: "agent-1",
  parentSubagentId: null,
  provider: "claude",
  title: "Explore",
  description: null,
  status: "completed",
  createdAt: "2026-10-15T00:00:00.000Z",
  updatedAt: "2026-10-15T00:00:05.000Z",
  toolCallId: "tool-1",
  cwd: null,
  subtitle: null,
};

describe("SqliteTimelineStore", () => {
  let home: string;
  let dbPath: string;
  const open: SqliteTimelineStore[] = [];

  async function openStore(): Promise<SqliteTimelineStore> {
    const store = await openTimelineCacheStore(home, createTestLogger());
    if (!store) throw new Error("Timeline cache store did not open");
    open.push(store);
    return store;
  }

  beforeEach(async () => {
    home = await mkdtemp(path.join(tmpdir(), "sqlite-timeline-store-"));
    dbPath = path.join(home, "timeline-cache.sqlite");
  });

  afterEach(async () => {
    for (const store of open.splice(0)) store.close();
    await rm(home, { recursive: true, force: true });
  });

  it("round-trips appended rows with every field through flush and load", async () => {
    const store = await openStore();
    store.reset("agent-1", "epoch-1");
    store.append("agent-1", [
      row(1, { turnId: "turn-1" }),
      row(2, { turnId: "turn-1", providerMessageId: "provider-2" }),
      { seq: 3, timestamp: "2026-10-15T00:00:03.000Z", item: { type: "error", message: "boom" } },
    ]);
    store.flush();

    expect(await store.load("agent-1")).toEqual({
      epoch: "epoch-1",
      nextSeq: 4,
      complete: false,
      providerSubagents: [],
      rows: [
        row(1, { turnId: "turn-1" }),
        row(2, { turnId: "turn-1", providerMessageId: "provider-2" }),
        { seq: 3, timestamp: "2026-10-15T00:00:03.000Z", item: { type: "error", message: "boom" } },
      ],
    });
  });

  it("load flushes the agent's buffered rows first", async () => {
    const store = await openStore();
    store.reset("agent-1", "epoch-1");
    store.append("agent-1", [row(1), row(2)]);

    const snapshot = await store.load("agent-1");

    expect(snapshot?.rows).toEqual([row(1), row(2)]);
  });

  it("reset deletes rows and stores the epoch across a reopen", async () => {
    const first = await openStore();
    first.reset("agent-1", "epoch-1");
    first.append("agent-1", [row(1), row(2)]);
    first.setComplete("agent-1", true);
    first.reset("agent-1", "epoch-2");
    first.close();

    const second = await openStore();

    expect(second.getMeta("agent-1")).toEqual({ epoch: "epoch-2", nextSeq: 1, complete: false });
    expect((await second.load("agent-1"))?.rows).toEqual([]);
  });

  it("update replaces the row at the same seq", async () => {
    const store = await openStore();
    store.reset("agent-1", "epoch-1");
    store.append("agent-1", [row(1), row(2)]);
    store.flush();
    store.update("agent-1", row(2, { providerMessageId: "provider-2" }));
    store.flush();

    expect((await store.load("agent-1"))?.rows).toEqual([
      row(1),
      row(2, { providerMessageId: "provider-2" }),
    ]);
  });

  it("setComplete(true) writes buffered rows and survives a reopen", async () => {
    const first = await openStore();
    first.reset("agent-1", "epoch-1");
    first.append("agent-1", [row(1), row(2), row(3)]);
    first.setProviderSubagents("agent-1", [descriptor]);
    first.setComplete("agent-1", true);
    const raw = await openRaw(dbPath);
    const rowsBeforeClose = countRows(raw, "agent-1");
    raw.close();
    first.close();

    const second = await openStore();

    expect(rowsBeforeClose).toBe(3);
    expect(second.getMeta("agent-1")).toEqual({ epoch: "epoch-1", nextSeq: 4, complete: true });
    expect(await second.load("agent-1")).toEqual({
      epoch: "epoch-1",
      nextSeq: 4,
      complete: true,
      rows: [row(1), row(2), row(3)],
      providerSubagents: [descriptor],
    });
  });

  it("rebuilds an empty cache from a file with an older schema version", async () => {
    const raw = await openRaw(dbPath);
    raw.exec("CREATE TABLE timelines (agent_id TEXT PRIMARY KEY, junk TEXT)");
    raw.exec("INSERT INTO timelines VALUES ('agent-1', 'x')");
    raw.exec("PRAGMA user_version = 1");
    raw.close();

    const store = await openStore();

    expect(store.getMeta("agent-1")).toBeNull();
    store.reset("agent-1", "epoch-1");
    expect(store.getMeta("agent-1")).toEqual({ epoch: "epoch-1", nextSeq: 1, complete: false });
  });

  it("rebuilds an empty cache from a garbage file", async () => {
    await writeFile(dbPath, "this is not a sqlite database, only some garbage bytes ".repeat(200));

    const store = await openStore();

    expect(store.getMeta("agent-1")).toBeNull();
    store.reset("agent-1", "epoch-1");
    store.append("agent-1", [row(1)]);
    expect((await store.load("agent-1"))?.rows).toEqual([row(1)]);
  });

  it("deleteAgent removes rows and meta", async () => {
    const store = await openStore();
    store.reset("agent-1", "epoch-1");
    store.append("agent-1", [row(1), row(2)]);
    store.setComplete("agent-1", true);
    store.reset("agent-2", "epoch-2");
    store.append("agent-2", [row(1)]);
    store.flush();

    store.deleteAgent("agent-1");

    const raw = await openRaw(dbPath);
    const remaining = { agent1: countRows(raw, "agent-1"), agent2: countRows(raw, "agent-2") };
    raw.close();
    expect(remaining).toEqual({ agent1: 0, agent2: 1 });
    expect(store.getMeta("agent-1")).toBeNull();
    expect(await store.load("agent-1")).toBeNull();
    expect(store.getMeta("agent-2")).toEqual({ epoch: "epoch-2", nextSeq: 2, complete: false });
  });

  it("flushes 1,200 buffered rows in 3 transactions across event-loop ticks", async () => {
    const store = await openStore();
    store.reset("agent-1", "epoch-1");
    const raw = await openRaw(dbPath);
    const rows = Array.from({ length: 1_200 }, (_, index) => row(index + 1));

    store.append("agent-1", rows);
    const counts = [countRows(raw, "agent-1")];
    for (let tick = 0; tick < 3; tick++) {
      await nextTick();
      counts.push(countRows(raw, "agent-1"));
    }
    raw.close();

    expect(counts).toEqual([0, 500, 1_000, 1_200]);
    expect(store.getMeta("agent-1")).toEqual({ epoch: "epoch-1", nextSeq: 1_201, complete: false });
  });

  it("lists prompt rows by item type, flushing buffered rows first", async () => {
    const store = await openStore();
    store.reset("agent-1", "epoch-1");
    const prompt = (seq: number, text: string): AgentTimelineRow => ({
      seq,
      timestamp: `2026-10-15T00:00:0${seq}.000Z`,
      item: { type: "user_message", text, messageId: `u-${seq}` },
    });
    store.append("agent-1", [prompt(1, "first"), row(2), row(3), prompt(4, "second")]);
    store.update("agent-1", { ...prompt(4, "second"), providerMessageId: "provider-4" });

    expect(store.listPromptRows("agent-1")).toEqual([
      prompt(1, "first"),
      { ...prompt(4, "second"), providerMessageId: "provider-4" },
    ]);
    expect(store.listPromptRows("agent-2")).toEqual([]);
  });

  it("indexes projected messages: chunks of one message join, across flushes", async () => {
    const store = await openStore();
    store.reset("agent-1", "epoch-1");
    const chunk = (seq: number, text: string, turnId = "turn-1"): AgentTimelineRow => ({
      seq,
      timestamp: "2026-10-15T00:00:00.000Z",
      turnId,
      item: { type: "assistant_message", text },
    });
    store.append("agent-1", [
      { seq: 1, timestamp: "2026-10-15T00:00:00.000Z", item: { type: "user_message", text: "go" } },
      chunk(2, "split wo"),
    ]);
    store.flush();
    store.append("agent-1", [chunk(3, "rd here"), chunk(4, "other turn", "turn-2")]);

    expect(store.findSearchCandidates("agent-1", "word", 0, 10)).toEqual([
      { seq: 3, role: "assistant", text: "split word here" },
    ]);
    // Indexed per chunk, "rd" would be a word of its own.
    expect(store.findSearchCandidates("agent-1", "rd", 0, 10)).toEqual([]);
    expect(store.findSearchCandidates("agent-1", "turn", 0, 10)).toEqual([
      { seq: 4, role: "assistant", text: "other turn" },
    ]);
    expect(store.findSearchCandidates("agent-1", "word", 3, 10)).toEqual([]);
    // A query without a word lists every message.
    expect(store.findSearchCandidates("agent-1", "&&", 0, 2)?.map((hit) => hit.seq)).toEqual([
      1, 3,
    ]);
  });

  it("matches without diacritics, including đ", async () => {
    const store = await openStore();
    store.reset("agent-1", "epoch-1");
    store.append("agent-1", [
      { seq: 1, timestamp: "t", item: { type: "user_message", text: "Đường Tiếng Việt" } },
    ]);

    expect(store.findSearchCandidates("agent-1", "duong tieng viet", 0, 10)).toHaveLength(1);
    expect(store.findSearchCandidates("agent-1", "đường", 0, 10)).toHaveLength(1);
  });

  it("re-indexes a rewritten message and drops the index on reset and delete", async () => {
    const store = await openStore();
    store.reset("agent-1", "epoch-1");
    store.append("agent-1", [
      { seq: 1, timestamp: "t", item: { type: "user_message", text: "alpha" } },
    ]);
    store.flush();
    store.update("agent-1", {
      seq: 1,
      timestamp: "t",
      providerMessageId: "provider-1",
      item: { type: "user_message", text: "alpha" },
    });

    expect(store.findSearchCandidates("agent-1", "alpha", 0, 10)).toEqual([
      { seq: 1, role: "user", text: "alpha" },
    ]);
    store.reset("agent-1", "epoch-2");
    expect(store.findSearchCandidates("agent-1", "alpha", 0, 10)).toEqual([]);
    store.append("agent-1", [
      { seq: 1, timestamp: "t", item: { type: "user_message", text: "alpha" } },
    ]);
    store.deleteAgent("agent-1");
    expect(store.findSearchCandidates("agent-1", "alpha", 0, 10)).toEqual([]);
  });

  it("reads seq windows and identity rows, and reports each committed flush", async () => {
    const store = await openStore();
    const flushed: Array<[string, number]> = [];
    store.setFlushListener((agentId, maxSeq) => flushed.push([agentId, maxSeq]));
    store.reset("agent-1", "epoch-1");
    const tool = (seq: number, status: "running" | "completed"): AgentTimelineRow => ({
      seq,
      timestamp: "t",
      item: {
        type: "tool_call",
        callId: "call-1",
        name: "Read",
        status,
        detail: { type: "unknown", input: null, output: null },
        error: null,
      },
    });
    const rows = Array.from({ length: 2_500 }, (_, index) => row(index + 1));
    rows[9] = tool(10, "running");
    rows[2_399] = tool(2_400, "completed");
    store.append("agent-1", rows);

    expect(store.readRowWindow("agent-1", 2_000, 2_002)?.map((entry) => entry.seq)).toEqual([
      2_000, 2_001, 2_002,
    ]);
    expect(store.readRowWindow("agent-1", 1, 2_500)).toEqual(rows);
    expect(store.readIdentityRows("agent-1", ["call-1"], 2_400)).toEqual([rows[9]]);
    expect(store.readIdentityRows("agent-1", ["call-1"], 2_401)).toEqual([rows[9], rows[2_399]]);
    expect(flushed.at(-1)).toEqual(["agent-1", 2_500]);
  });

  it("stores a transcript fingerprint with complete and clears it otherwise", async () => {
    const first = await openStore();
    first.reset("agent-1", "epoch-1");
    first.append("agent-1", [row(1)]);
    first.setComplete("agent-1", true, "fingerprint-1");
    first.close();

    const second = await openStore();
    expect(second.getMeta("agent-1")).toEqual({
      epoch: "epoch-1",
      nextSeq: 2,
      complete: true,
      transcriptFingerprint: "fingerprint-1",
    });
    expect((await second.load("agent-1"))?.transcriptFingerprint).toBe("fingerprint-1");
    second.setComplete("agent-1", false);
    expect(second.getMeta("agent-1")).toEqual({ epoch: "epoch-1", nextSeq: 2, complete: false });
    second.setComplete("agent-1", true);
    second.reset("agent-1", "epoch-2");
    expect(second.getMeta("agent-1")?.transcriptFingerprint).toBeUndefined();
  });

  it("loads more rows than one page in seq order", async () => {
    const store = await openStore();
    store.reset("agent-1", "epoch-1");
    const rows = Array.from({ length: 4_500 }, (_, index) => row(index + 1));
    store.append("agent-1", rows);

    const snapshot = await store.load("agent-1");

    expect(snapshot?.rows).toEqual(rows);
  });
});
