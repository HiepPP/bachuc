import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import {
  appendFileSync,
  existsSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { randomUUID } from "node:crypto";
import pino, { type Logger } from "pino";

import { createTestLogger } from "../../test-utils/test-logger.js";
import { createTestAgentClients } from "../test-utils/fake-agent-client.js";
import { AgentManager, type AgentManagerEvent } from "./agent-manager.js";
import { AgentStorage, type StoredAgentRecord } from "./agent-storage.js";
import { ensureAgentLoaded, ensureAgentTimelineReadable } from "./agent-loading.js";
import { InMemoryAgentTimelineStore } from "./agent-timeline-store.js";
import type {
  CachedHistoryResumeOptions,
  TranscriptFingerprintSource,
} from "./agent-timeline-store-types.js";
import { searchTimeline } from "./chat-search/index.js";
import { openTimelineCacheStore, type SqliteTimelineStore } from "./sqlite-timeline-store.js";
import type {
  AgentClient,
  AgentResumeSessionOptions,
  AgentStreamEvent,
  AgentTimelineItem,
} from "./agent-sdk-types.js";

const logger = createTestLogger();
/** The fake client streams the reply as "the qu" + "ick searcher answers": one word, two chunks. */
const SEARCH_PROMPT = "Respond with exactly: the quick searcher answers";
const SEARCH_USER_MESSAGE: AgentTimelineItem = { type: "user_message", text: "a quick searcher?" };

interface Daemon {
  manager: AgentManager;
  registry: AgentStorage;
  store: SqliteTimelineStore;
  claude: AgentClient;
  codex: AgentClient;
}

/** Read counter for the transcript a provider parses on a history load. */
interface TranscriptProbe {
  bytesRead: number;
  resumeOptions: Array<(AgentResumeSessionOptions & CachedHistoryResumeOptions) | undefined>;
}

/**
 * Gives a fake client the transcript fingerprint hook over `file`. Each resume without
 * `historyCached` reads the whole file, as a provider history parse does, and each line after the
 * first streams as one more history row.
 */
function attachTranscript(client: AgentClient, file: string, withHook = true): TranscriptProbe {
  const probe: TranscriptProbe = { bytesRead: 0, resumeOptions: [] };
  if (withHook) {
    Object.assign(client, {
      getTranscriptFingerprint: async () => {
        if (!existsSync(file)) return null;
        const { size, mtimeMs } = statSync(file);
        return `${size}:${mtimeMs}`;
      },
    } satisfies TranscriptFingerprintSource);
  }
  const resume = client.resumeSession.bind(client);
  client.resumeSession = async (handle, overrides, launchContext, options) => {
    const resumeOptions: (AgentResumeSessionOptions & CachedHistoryResumeOptions) | undefined =
      options;
    probe.resumeOptions.push(resumeOptions);
    const session = await resume(handle, overrides, launchContext, options);
    if (resumeOptions?.historyCached) return session;
    const text = existsSync(file) ? readFileSync(file, "utf8") : "";
    probe.bytesRead += Buffer.byteLength(text);
    const extraRows = text.split("\n").filter(Boolean).slice(1);
    const streamHistory = session.streamHistory.bind(session);
    session.streamHistory = async function* (): AsyncGenerator<AgentStreamEvent> {
      yield* streamHistory();
      for (const row of extraRows) {
        yield {
          type: "timeline",
          provider: client.provider,
          // Not an assistant message, which would merge into the last reply.
          item: { type: "error", message: row },
        };
      }
    };
    return session;
  };
  return probe;
}

let home: string;
let workdir: string;
const daemons: Daemon[] = [];

beforeEach(() => {
  home = mkdtempSync(join(tmpdir(), "agent-manager-durable-timeline-"));
  workdir = mkdtempSync(join(tmpdir(), "agent-manager-durable-timeline-cwd-"));
});

afterEach(async () => {
  for (const daemon of daemons.splice(0)) {
    for (const agent of daemon.manager.listAgents()) {
      await daemon.manager.closeAgent(agent.id).catch(() => undefined);
    }
    daemon.store.close();
  }
  rmSync(home, { recursive: true, force: true });
  rmSync(workdir, { recursive: true, force: true });
});

/** One daemon process on the shared home: its own storage, cache handle, and manager. */
async function startDaemon(
  managerLogger: Logger = logger,
  options?: { timelineTailRows?: number },
): Promise<Daemon> {
  const store = await openTimelineCacheStore(home, logger);
  if (!store) throw new Error("node:sqlite is unavailable");
  const registry = new AgentStorage(join(home, "agents"), logger);
  const clients = createTestAgentClients();
  const manager = new AgentManager({
    clients,
    registry,
    durableTimelineStore: store,
    logger: managerLogger,
    timelineTailRows: options?.timelineTailRows,
  });
  const daemon = { manager, registry, store, claude: clients.claude, codex: clients.codex };
  daemons.push(daemon);
  return daemon;
}

/** Ends a daemon process without closing its agents, as a crash or kill would. */
async function stopDaemon(daemon: Daemon): Promise<void> {
  await daemon.manager.flush();
  await daemon.registry.flush();
  daemons.splice(daemons.indexOf(daemon), 1);
  daemon.store.close();
}

async function createAgentWithTurn(daemon: Daemon, prompt = "hello there"): Promise<string> {
  const agent = await daemon.manager.createAgent({ provider: "claude", cwd: workdir }, undefined, {
    workspaceId: undefined,
  });
  await daemon.manager.runAgent(agent.id, prompt);
  return agent.id;
}

function timelineItems(daemon: Daemon, agentId: string): AgentTimelineItem[] {
  return daemon.manager
    .fetchTimelineForRead(agentId, { direction: "tail", limit: 0 })
    .rows.map((row) => row.item);
}

/** getTimelineRows reads memory only and does not count as a read, so it probes without evicting. */
async function isInMemory(daemon: Daemon, agentId: string): Promise<boolean> {
  try {
    await daemon.manager.getTimelineRows(agentId);
    return true;
  } catch {
    return false;
  }
}

async function requireRecord(daemon: Daemon, agentId: string): Promise<StoredAgentRecord> {
  const record = await daemon.registry.get(agentId);
  if (!record) throw new Error(`Missing record ${agentId}`);
  return record;
}

describe("AgentManager durable timeline", () => {
  test("a closed timeline reads from the cache after a restart without a resume", async () => {
    const first = await startDaemon();
    const agentId = await createAgentWithTurn(first);
    const before = first.manager.fetchTimeline(agentId, { direction: "tail", limit: 0 });
    await first.manager.closeAgent(agentId);
    await stopDaemon(first);

    const second = await startDaemon();
    const resume = vi.spyOn(second.claude, "resumeSession");

    await expect(second.manager.loadTimelineForRead(agentId)).resolves.toBe("durable");
    const after = second.manager.fetchTimelineForRead(agentId, { direction: "tail", limit: 0 });
    expect(after.epoch).toBe(before.epoch);
    expect(after.rows).toEqual(before.rows);
    expect(after.rows.length).toBeGreaterThan(0);
    expect(resume).not.toHaveBeenCalled();
    expect(second.manager.getAgent(agentId)).toBeNull();

    // A later resume keeps the cached timeline instead of replaying provider history.
    await ensureAgentLoaded(agentId, {
      agentManager: second.manager,
      agentStorage: second.registry,
      logger,
    });
    const resumed = second.manager.fetchTimeline(agentId, { direction: "tail", limit: 0 });
    expect(resumed.epoch).toBe(before.epoch);
    expect(resumed.rows).toEqual(before.rows);
  });

  test("close then resume in one daemon keeps the epoch and does not replay", async () => {
    const daemon = await startDaemon();
    const agentId = await createAgentWithTurn(daemon);
    const before = daemon.manager.fetchTimeline(agentId, { direction: "tail", limit: 0 });
    await daemon.manager.closeAgent(agentId);

    await ensureAgentLoaded(agentId, {
      agentManager: daemon.manager,
      agentStorage: daemon.registry,
      logger,
    });

    const after = daemon.manager.fetchTimeline(agentId, { direction: "tail", limit: 0 });
    expect(after.epoch).toBe(before.epoch);
    expect(after.rows).toEqual(before.rows);
  });

  test("a crash leaves the timeline incomplete, and a backfill rebuilds it from provider history", async () => {
    const first = await startDaemon();
    const agentId = await createAgentWithTurn(first);
    const before = first.manager.fetchTimeline(agentId, { direction: "tail", limit: 0 });
    await stopDaemon(first);

    const second = await startDaemon();
    expect(second.store.getMeta(agentId)?.complete).toBe(false);
    await expect(second.manager.loadTimelineForRead(agentId)).resolves.toBe("missing");

    await second.manager.backfillTimelineFromProvider(
      agentId,
      await requireRecord(second, agentId),
    );

    const after = second.manager.fetchTimelineForRead(agentId, { direction: "tail", limit: 0 });
    expect(after.epoch).not.toBe(before.epoch);
    expect(after.rows.map((row) => row.item)).toEqual(before.rows.map((row) => row.item));
    expect(second.store.getMeta(agentId)).toEqual({
      epoch: after.epoch,
      nextSeq: expect.any(Number),
      complete: true,
    });
    expect(second.manager.getAgent(agentId)).toBeNull();

    // The rebuilt entry serves the next daemon directly.
    await stopDaemon(second);
    const third = await startDaemon();
    await expect(third.manager.loadTimelineForRead(agentId)).resolves.toBe("durable");
    expect(third.manager.fetchTimelineForRead(agentId).epoch).toBe(after.epoch);
  });

  test("rewind and reload-rehydrate mint a new epoch and reset the stored rows", async () => {
    const daemon = await startDaemon();
    const agentId = await createAgentWithTurn(daemon);
    const original = daemon.manager.fetchTimeline(agentId);
    const originalRows = (await daemon.store.load(agentId))?.rows ?? [];
    expect(originalRows.length).toBeGreaterThan(0);

    // Rewind replaces the timeline through a forced hydrate.
    await daemon.manager.hydrateTimelineFromProvider(agentId, { force: true });
    const rewound = daemon.manager.fetchTimeline(agentId);
    expect(rewound.epoch).not.toBe(original.epoch);
    const rewoundSnapshot = await daemon.store.load(agentId);
    expect(rewoundSnapshot?.epoch).toBe(rewound.epoch);
    expect(rewoundSnapshot?.rows.map((row) => row.item)).toEqual(
      originalRows.map((row) => row.item),
    );
    expect(rewoundSnapshot?.rows.map((row) => row.seq)).toEqual(originalRows.map((row) => row.seq));

    await daemon.manager.reloadAgentSession(agentId, undefined, { rehydrateFromDisk: true });
    await daemon.manager.hydrateTimelineFromProvider(agentId);
    const reloaded = daemon.manager.fetchTimeline(agentId);
    expect(reloaded.epoch).not.toBe(rewound.epoch);
    const reloadedSnapshot = await daemon.store.load(agentId);
    expect(reloadedSnapshot?.epoch).toBe(reloaded.epoch);
    expect(reloadedSnapshot?.rows.map((row) => row.item)).toEqual(
      originalRows.map((row) => row.item),
    );
    expect(reloadedSnapshot?.rows.map((row) => row.seq)).toEqual(
      originalRows.map((row) => row.seq),
    );
  });

  test("archive keeps the stored timeline readable, and delete removes it", async () => {
    const daemon = await startDaemon();
    const agentId = await createAgentWithTurn(daemon);
    const before = daemon.manager.fetchTimeline(agentId, { direction: "tail", limit: 0 });

    await daemon.manager.archiveAgent(agentId);

    await expect(daemon.manager.loadTimelineForRead(agentId)).resolves.toBe("durable");
    const archived = daemon.manager.fetchTimelineForRead(agentId, { direction: "tail", limit: 0 });
    expect(archived.epoch).toBe(before.epoch);
    expect(archived.rows).toEqual(before.rows);

    await daemon.manager.deleteAgentState(agentId);

    expect(daemon.store.getMeta(agentId)).toBeNull();
    await expect(daemon.manager.loadTimelineForRead(agentId)).resolves.toBe("missing");
  });

  test("clearing attention on a closed agent updates its stored record without a resume", async () => {
    const daemon = await startDaemon();
    const agentId = await createAgentWithTurn(daemon);
    await daemon.manager.closeAgent(agentId);
    const record = await requireRecord(daemon, agentId);
    await daemon.registry.upsert({
      ...record,
      requiresAttention: true,
      attentionReason: "finished",
      attentionTimestamp: record.updatedAt,
    });
    const resume = vi.spyOn(daemon.claude, "resumeSession");
    const events: AgentManagerEvent[] = [];
    const unsubscribe = daemon.manager.subscribe((event) => events.push(event), {
      agentId,
      replayState: false,
    });

    await daemon.manager.clearAgentAttention(agentId);
    unsubscribe();

    expect(await requireRecord(daemon, agentId)).toMatchObject({
      requiresAttention: false,
      attentionReason: null,
      attentionTimestamp: null,
      updatedAt: record.updatedAt,
    });
    expect(events).toEqual([
      {
        type: "agent_state",
        agent: expect.objectContaining({
          id: agentId,
          lifecycle: "closed",
          attention: { requiresAttention: false },
        }),
      },
    ]);
    expect(resume).not.toHaveBeenCalled();
    expect(daemon.manager.getAgent(agentId)).toBeNull();
  });

  test("provider subagent descriptors survive a restart", async () => {
    const first = await startDaemon();
    const agentId = await createAgentWithTurn(first, "Emit a provider child");
    expect(first.manager.listProviderSubagents(agentId)).toHaveLength(1);
    await first.manager.closeAgent(agentId);
    await stopDaemon(first);

    const second = await startDaemon();
    await expect(second.manager.loadTimelineForRead(agentId)).resolves.toBe("durable");

    expect(second.manager.listProviderSubagentsForRead(agentId)).toEqual([
      expect.objectContaining({
        id: "fixture-child",
        parentAgentId: agentId,
        provider: "claude",
        title: "Fixture child",
        status: "canceled",
      }),
    ]);
  });

  test("keeps at most 64 cold timelines in memory and reloads an evicted one under its epoch", async () => {
    const daemon = await startDaemon();
    const agentIds = Array.from({ length: 65 }, () => randomUUID());
    for (const [index, agentId] of agentIds.entries()) {
      daemon.store.reset(agentId, `epoch-${index}`);
      daemon.store.append(agentId, [
        {
          seq: 1,
          timestamp: "2026-10-15T00:00:00.000Z",
          item: { type: "assistant_message", text: `row ${index}` },
        },
      ]);
      daemon.store.setComplete(agentId, true);
    }

    for (const agentId of agentIds) {
      await expect(daemon.manager.loadTimelineForRead(agentId)).resolves.toBe("durable");
    }

    const inMemory = await Promise.all(agentIds.map((agentId) => isInMemory(daemon, agentId)));
    expect(inMemory.filter(Boolean)).toHaveLength(64);
    expect(inMemory[0]).toBe(false);

    await expect(daemon.manager.loadTimelineForRead(agentIds[64])).resolves.toBe("memory");
    await expect(daemon.manager.loadTimelineForRead(agentIds[0])).resolves.toBe("durable");
    expect(daemon.manager.fetchTimelineForRead(agentIds[0]).epoch).toBe("epoch-0");
    expect(timelineItems(daemon, agentIds[0])).toEqual([
      { type: "assistant_message", text: "row 0" },
    ]);
  });

  test("prompt list and search read the cache and match the in-memory path", async () => {
    const daemon = await startDaemon();
    const agentId = await createAgentWithTurn(daemon, SEARCH_PROMPT);
    await daemon.manager.appendTimelineItem(agentId, SEARCH_USER_MESSAGE);
    await daemon.manager.closeAgent(agentId);
    await expect(daemon.manager.loadTimelineForRead(agentId)).resolves.toBe("memory");
    const listPromptRows = vi.spyOn(daemon.store, "listPromptRows");
    const findSearchCandidates = vi.spyOn(daemon.store, "findSearchCandidates");
    const memoryRows = await daemon.manager.getTimelineRows(agentId);

    const prompts = await daemon.manager.getTimelinePromptRows(agentId);
    const search = await daemon.manager.searchTimeline(agentId, { query: "quick searcher" });

    expect(listPromptRows).toHaveReturnedWith(expect.any(Array));
    expect(findSearchCandidates).toHaveReturnedWith(expect.any(Array));
    expect(prompts.map((row) => [row.seq, row.timestamp, row.item])).toEqual(
      memoryRows
        .filter((row) => row.item.type === "user_message")
        .map((row) => [row.seq, row.timestamp, row.item]),
    );
    expect(search).toEqual(await searchTimeline({ rows: memoryRows, query: "quick searcher" }));
    expect(prompts).toHaveLength(1);
    expect(search.locations).toHaveLength(2);
  });

  test("prompt list and search fall back to memory without a usable cache", async () => {
    const registry = new AgentStorage(join(home, "agents"), logger);
    const manager = new AgentManager({ clients: createTestAgentClients(), registry, logger });
    const agent = await manager.createAgent({ provider: "claude", cwd: workdir }, undefined, {
      workspaceId: undefined,
    });
    await manager.runAgent(agent.id, SEARCH_PROMPT);
    await manager.appendTimelineItem(agent.id, SEARCH_USER_MESSAGE);
    const memoryRows = await manager.getTimelineRows(agent.id);

    expect(await manager.getTimelinePromptRows(agent.id)).toEqual([
      expect.objectContaining({ item: SEARCH_USER_MESSAGE }),
    ]);
    expect(await manager.searchTimeline(agent.id, { query: "quick searcher" })).toEqual(
      await searchTimeline({ rows: memoryRows, query: "quick searcher" }),
    );
    await manager.closeAgent(agent.id);

    // A store that lost writes for the agent serves nothing; reads fall back to memory.
    const daemon = await startDaemon();
    const agentId = await createAgentWithTurn(daemon, SEARCH_PROMPT);
    await daemon.manager.appendTimelineItem(agentId, SEARCH_USER_MESSAGE);
    vi.spyOn(daemon.store, "listPromptRows").mockReturnValue(null);
    vi.spyOn(daemon.store, "findSearchCandidates").mockImplementation(() => {
      throw new Error("disk I/O error");
    });
    const rows = await daemon.manager.getTimelineRows(agentId);
    expect(await daemon.manager.getTimelinePromptRows(agentId)).toEqual([
      expect.objectContaining({ item: SEARCH_USER_MESSAGE }),
    ]);
    expect(await daemon.manager.searchTimeline(agentId, { query: "quick searcher" })).toEqual(
      await searchTimeline({ rows, query: "quick searcher" }),
    );
  });

  test("a live agent keeps 1,000 source rows in memory after a flush of 5,000", async () => {
    const daemon = await startDaemon();
    const agent = await daemon.manager.createAgent(
      { provider: "claude", cwd: workdir },
      undefined,
      { workspaceId: undefined },
    );
    for (let index = 0; index < 5_000; index++) {
      await daemon.manager.appendTimelineItem(agent.id, {
        type: "assistant_message",
        text: `row ${index}`,
        messageId: `message-${index}`,
      });
    }

    const beforeFlush = await daemon.manager.getTimelineRows(agent.id);
    daemon.store.flush();
    const afterFlush = await daemon.manager.getTimelineRows(agent.id);

    expect(beforeFlush.length).toBeGreaterThan(1_000);
    expect(afterFlush).toHaveLength(1_000);
    expect(afterFlush[0]?.item).toMatchObject({ text: "row 4000" });
  });

  test("pages before across the tail edge with whole items, and finds a prompt below it", async () => {
    const daemon = await startDaemon(logger, { timelineTailRows: 20 });
    const agent = await daemon.manager.createAgent(
      { provider: "claude", cwd: workdir },
      undefined,
      { workspaceId: undefined },
    );
    const append = (item: AgentTimelineItem) => daemon.manager.appendTimelineItem(agent.id, item);
    for (let turn = 0; turn < 6; turn++) {
      await append({ type: "user_message", text: `prompt ${turn}` });
      for (let step = 0; step < 8; step++) {
        // Chunks of one message: a page edge must never split them.
        await append({ type: "assistant_message", text: `answer ${turn}.${step} part one, ` });
        await append({ type: "assistant_message", text: "part two" });
        await append({ type: "reasoning", text: `step ${turn}.${step}` });
      }
    }
    await append({ type: "user_message", text: "latest prompt" });
    for (let step = 0; step < 25; step++) {
      await append({ type: "reasoning", text: `tail ${step}` });
      await append({ type: "assistant_message", text: `tail answer ${step}` });
    }
    daemon.store.flush();
    expect((await daemon.manager.getTimelineRows(agent.id)).length).toBeLessThanOrEqual(20);

    const rows = (await daemon.store.load(agent.id))?.rows ?? [];
    expect(rows.length).toBeGreaterThan(200);
    const fullMemory = new InMemoryAgentTimelineStore();
    fullMemory.initialize(agent.id, { rows, epoch: "epoch" });
    const fullTimeline = fullMemory.fetch(agent.id, { direction: "tail", limit: 0 }).rows;

    const epoch = daemon.manager.fetchTimeline(agent.id, { limit: 1 }).epoch;
    const collected: typeof fullTimeline = [];
    let page = daemon.manager.fetchTimeline(agent.id, { direction: "tail", limit: 5 });
    collected.unshift(...page.rows);
    while (page.hasOlder) {
      page = daemon.manager.fetchTimeline(agent.id, {
        direction: "before",
        cursor: { epoch, seq: page.startSeq! },
        limit: 5,
      });
      collected.unshift(...page.rows);
    }
    expect(collected.map(({ seq: _seq, ...entry }) => entry)).toEqual(
      fullTimeline.map(({ seq: _seq, ...entry }) => entry),
    );

    const latest = daemon.manager.fetchTimeline(agent.id, {
      direction: "tail",
      limit: 2,
      includeLatestPrompt: true,
    });
    expect(latest.rows[0]?.item).toEqual({ type: "user_message", text: "latest prompt" });
    expect(latest.rows).toHaveLength(51);
  });

  test("without a cache, timelines stay whole in memory", async () => {
    const manager = new AgentManager({
      clients: createTestAgentClients(),
      registry: new AgentStorage(join(home, "agents"), logger),
      logger,
      timelineTailRows: 5,
    });
    const agent = await manager.createAgent({ provider: "claude", cwd: workdir }, undefined, {
      workspaceId: undefined,
    });
    for (let index = 0; index < 40; index++) {
      await manager.appendTimelineItem(agent.id, {
        type: "assistant_message",
        text: `row ${index}`,
        messageId: `message-${index}`,
      });
    }

    expect(await manager.getTimelineRows(agent.id)).toHaveLength(40);
    expect(manager.fetchTimeline(agent.id, { direction: "tail", limit: 0 }).rows).toHaveLength(40);
    await manager.closeAgent(agent.id);
  });

  test("a cache that rejects writes fails no lifecycle step and stops serving the agent", async () => {
    const lines: string[] = [];
    const daemon = await startDaemon(
      pino({ level: "error" }, { write: (line: string) => lines.push(line) }),
    );
    const diskError = () => {
      throw new Error("disk I/O error");
    };
    vi.spyOn(daemon.store, "reset").mockImplementation(diskError);
    vi.spyOn(daemon.store, "append").mockImplementation(diskError);
    vi.spyOn(daemon.store, "setComplete").mockImplementation(diskError);
    vi.spyOn(daemon.store, "deleteAgent").mockImplementation(diskError);

    const agentId = await createAgentWithTurn(daemon);
    await daemon.manager.closeAgent(agentId);
    await ensureAgentLoaded(agentId, {
      agentManager: daemon.manager,
      agentStorage: daemon.registry,
      logger,
    });
    await daemon.manager.closeAgent(agentId);

    // Memory still holds the timeline, but a cache that missed writes must not serve it.
    await expect(daemon.manager.loadTimelineForRead(agentId)).resolves.toBe("missing");
    await expect(daemon.manager.deleteAgentState(agentId)).resolves.toBeUndefined();

    const failures = lines
      .map((line) => JSON.parse(line) as { agentId?: string; msg?: string })
      .filter((entry) => entry.agentId === agentId && entry.msg?.startsWith("Timeline cache"));
    expect(failures).toHaveLength(1);
  });
});

describe("AgentManager transcript fingerprint", () => {
  function readable(daemon: Daemon, agentId: string) {
    return ensureAgentTimelineReadable(agentId, {
      agentManager: daemon.manager,
      agentStorage: daemon.registry,
      logger,
    });
  }

  function load(daemon: Daemon, agentId: string) {
    return ensureAgentLoaded(agentId, {
      agentManager: daemon.manager,
      agentStorage: daemon.registry,
      logger,
    });
  }

  /** Opens and resumes a closed thread with an unchanged transcript; returns bytes read. */
  async function openAndResumeUnchanged(withHook: boolean) {
    const transcript = join(home, `transcript-${randomUUID()}.jsonl`);
    writeFileSync(transcript, '{"type":"user"}\n');
    const first = await startDaemon();
    attachTranscript(first.claude, transcript, withHook);
    const agentId = await createAgentWithTurn(first);
    const before = first.manager.fetchTimeline(agentId, { direction: "tail", limit: 0 });
    await first.manager.closeAgent(agentId);
    await stopDaemon(first);

    const second = await startDaemon();
    const probe = attachTranscript(second.claude, transcript, withHook);
    await expect(second.manager.loadTimelineForRead(agentId)).resolves.toBe("durable");
    const openBytes = probe.bytesRead;
    await load(second, agentId);
    const resumed = second.manager.fetchTimeline(agentId, { direction: "tail", limit: 0 });
    expect(resumed.epoch).toBe(before.epoch);
    expect(resumed.rows).toEqual(before.rows);
    return { openBytes, resumeBytes: probe.bytesRead - openBytes, probe };
  }

  test("an unchanged transcript serves the cache and skips the history parse", async () => {
    const counter = await openAndResumeUnchanged(true);

    // Read counter for the TASK-130 anti-goal: 0 transcript bytes for the unchanged case.
    expect(counter.openBytes).toBe(0);
    expect(counter.resumeBytes).toBe(0);
    expect(counter.probe.resumeOptions).toEqual([expect.objectContaining({ historyCached: true })]);
  });

  test("a provider without the hook keeps today's path", async () => {
    const counter = await openAndResumeUnchanged(false);

    expect(counter.openBytes).toBe(0);
    expect(counter.resumeBytes).toBeGreaterThan(0);
    expect(counter.probe.resumeOptions).toEqual([{ purpose: "interactive" }]);

    // A Codex thread behaves as before: the cache serves it without a fingerprint.
    const daemon = await startDaemon();
    const agent = await daemon.manager.createAgent({ provider: "codex", cwd: workdir }, undefined, {
      workspaceId: undefined,
    });
    await daemon.manager.runAgent(agent.id, "hello there");
    await daemon.manager.closeAgent(agent.id);
    expect(daemon.store.getMeta(agent.id)).toEqual({
      epoch: expect.any(String),
      nextSeq: expect.any(Number),
      complete: true,
    });
    await expect(daemon.manager.loadTimelineForRead(agent.id)).resolves.toBe("memory");
  });

  test("a transcript appended outside Paseo backfills on open and shows the new rows", async () => {
    const transcript = join(home, "transcript.jsonl");
    writeFileSync(transcript, '{"type":"user"}\n');
    const daemon = await startDaemon();
    const probe = attachTranscript(daemon.claude, transcript);
    const agentId = await createAgentWithTurn(daemon);
    const before = daemon.manager.fetchTimeline(agentId, { direction: "tail", limit: 0 });
    await daemon.manager.closeAgent(agentId);
    await expect(daemon.manager.loadTimelineForRead(agentId)).resolves.toBe("memory");

    appendFileSync(transcript, "written by claude --resume\n");
    await readable(daemon, agentId);

    const after = daemon.manager.fetchTimelineForRead(agentId, { direction: "tail", limit: 0 });
    expect(after.epoch).not.toBe(before.epoch);
    expect(after.rows.at(-1)?.item).toEqual({
      type: "error",
      message: "written by claude --resume",
    });
    expect(probe.resumeOptions).toEqual([{ purpose: "history" }]);
    const bytesAfterBackfill = probe.bytesRead;
    expect(bytesAfterBackfill).toBeGreaterThan(0);

    // The rebuilt entry stores the new fingerprint, so the next open reads nothing.
    await expect(daemon.manager.loadTimelineForRead(agentId)).resolves.toBe("memory");
    expect(probe.bytesRead).toBe(bytesAfterBackfill);
  });

  test("a transcript changed before a resume rebuilds the timeline from provider history", async () => {
    const transcript = join(home, "transcript.jsonl");
    writeFileSync(transcript, '{"type":"user"}\n');
    const daemon = await startDaemon();
    const probe = attachTranscript(daemon.claude, transcript);
    const agentId = await createAgentWithTurn(daemon);
    const before = daemon.manager.fetchTimeline(agentId, { direction: "tail", limit: 0 });
    await daemon.manager.closeAgent(agentId);

    appendFileSync(transcript, "written outside\n");
    await load(daemon, agentId);

    const after = daemon.manager.fetchTimeline(agentId, { direction: "tail", limit: 0 });
    expect(after.epoch).not.toBe(before.epoch);
    expect(after.rows.at(-1)?.item).toEqual({ type: "error", message: "written outside" });
    expect(probe.resumeOptions).toEqual([{ purpose: "interactive" }]);
  });

  test("a deleted transcript backfills on open", async () => {
    const transcript = join(home, "transcript.jsonl");
    writeFileSync(transcript, '{"type":"user"}\n');
    const daemon = await startDaemon();
    const probe = attachTranscript(daemon.claude, transcript);
    const agentId = await createAgentWithTurn(daemon);
    await daemon.manager.closeAgent(agentId);

    rmSync(transcript);
    await expect(daemon.manager.loadTimelineForRead(agentId)).resolves.toBe("missing");
    await expect(readable(daemon, agentId)).resolves.toMatchObject({ kind: "stored" });

    expect(probe.resumeOptions).toEqual([{ purpose: "history" }]);
    expect(daemon.store.getMeta(agentId)).toEqual({
      epoch: expect.any(String),
      nextSeq: expect.any(Number),
      complete: true,
    });
  });
});
