import { mkdir, mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";

import { createTestLogger } from "../../test-utils/test-logger.js";
import { AgentManager } from "./agent-manager.js";
import { ensureAgentLoaded, ensureAgentTimelineReadable } from "./agent-loading.js";
import { openTimelineCacheStore, type SqliteTimelineStore } from "./sqlite-timeline-store.js";
import { startAgentRun } from "./agent-prompt.js";
import { AgentStorage } from "./agent-storage.js";
import type {
  AgentClient,
  AgentLaunchContext,
  AgentPersistenceHandle,
  AgentResumeSessionOptions,
  AgentSession,
  AgentSessionConfig,
} from "./agent-sdk-types.js";
import { createTestAgentClients } from "../test-utils/fake-agent-client.js";

test("loads archived records for history and active records with the interactive default", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "agent-loading-purpose-"));
  const logger = createTestLogger();
  const storage = new AgentStorage(path.join(root, "agents"), logger);
  const baseClient = createTestAgentClients().codex;
  if (!baseClient) {
    throw new Error("expected Codex test client");
  }

  const resumeOptions: Array<AgentResumeSessionOptions | undefined> = [];
  const client: AgentClient = {
    provider: baseClient.provider,
    capabilities: baseClient.capabilities,
    createSession: async (
      config: AgentSessionConfig,
      launchContext?: AgentLaunchContext,
    ): Promise<AgentSession> => await baseClient.createSession(config, launchContext),
    resumeSession: async (
      handle: AgentPersistenceHandle,
      overrides?: Partial<AgentSessionConfig>,
      launchContext?: AgentLaunchContext,
      options?: AgentResumeSessionOptions,
    ): Promise<AgentSession> => {
      resumeOptions.push(options);
      return await baseClient.resumeSession(handle, overrides, launchContext);
    },
    fetchCatalog: async (options) => await baseClient.fetchCatalog(options),
    isAvailable: async () => await baseClient.isAvailable(),
  };
  const manager = new AgentManager({
    clients: { codex: client },
    registry: storage,
    logger,
  });

  const archivedId = "00000000-0000-4000-8000-000000000301";
  const activeId = "00000000-0000-4000-8000-000000000302";

  try {
    const archived = await manager.createAgent({ provider: "codex", cwd: root }, archivedId, {
      workspaceId: "workspace-archived",
    });
    await manager.archiveAgent(archived.id);

    const active = await manager.createAgent({ provider: "codex", cwd: root }, activeId, {
      workspaceId: "workspace-active",
    });
    await manager.closeAgent(active.id);

    await ensureAgentLoaded(archived.id, { agentManager: manager, agentStorage: storage, logger });
    await ensureAgentLoaded(active.id, { agentManager: manager, agentStorage: storage, logger });

    expect(resumeOptions).toEqual([{ purpose: "history" }, { purpose: "interactive" }]);
  } finally {
    await Promise.all([
      manager.closeAgent(archivedId).catch(() => undefined),
      manager.closeAgent(activeId).catch(() => undefined),
    ]);
    await manager.flush().catch(() => undefined);
    await storage.flush().catch(() => undefined);
    await rm(root, { recursive: true, force: true });
  }
});

test("resuming a stored agent keeps its unread flag and its last-activity time", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "agent-loading-resume-"));
  const logger = createTestLogger();
  const storage = new AgentStorage(path.join(root, "agents"), logger);
  const manager = new AgentManager({
    clients: createTestAgentClients(),
    registry: storage,
    logger,
  });

  const agentId = "00000000-0000-4000-8000-000000000401";
  const lastActive = "2026-01-02T03:04:05.000Z";
  const markedUnread = "2026-01-09T03:04:05.000Z";

  try {
    const agent = await manager.createAgent({ provider: "codex", cwd: root }, agentId, {
      workspaceId: "workspace-a",
    });
    await manager.closeAgent(agent.id);
    await manager.flush();
    await storage.flush();

    const stored = await storage.get(agentId);
    if (!stored) {
      throw new Error("expected a stored agent");
    }
    // The agent finished days ago, and was marked unread later without being opened, which
    // moves `updatedAt` on its own. Clients already hold that newer time, and
    // `acceptAgentDirectoryUpdate` drops anything older, so the resumed agent must not come
    // back carrying only `lastActivityAt`.
    await storage.upsert({
      ...stored,
      updatedAt: markedUnread,
      lastActivityAt: lastActive,
      requiresAttention: true,
      attentionReason: "finished",
      attentionTimestamp: lastActive,
    });

    await ensureAgentLoaded(agentId, { agentManager: manager, agentStorage: storage, logger });
    await manager.flush();
    await storage.flush();

    // Loading the runtime is neither the agent working nor the user reading the chat.
    // Forging either rewrites the workspace's "last used" and drops it out of Ready to review.
    const resumed = await storage.get(agentId);
    expect(resumed?.requiresAttention).toBe(true);
    expect(resumed?.attentionReason).toBe("finished");
    expect(resumed?.updatedAt).toBe(markedUnread);
    expect(resumed?.lastActivityAt).toBe(markedUnread);
    expect(manager.getAgent(agentId)?.attention.requiresAttention).toBe(true);
    expect(manager.getAgent(agentId)?.updatedAt.toISOString()).toBe(markedUnread);
  } finally {
    await manager.closeAgent(agentId).catch(() => undefined);
    await manager.flush().catch(() => undefined);
    await storage.flush().catch(() => undefined);
    await rm(root, { recursive: true, force: true });
  }
});

test("loads an archived agent's history after its working directory is removed", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "agent-loading-missing-cwd-"));
  const worktree = path.join(root, "managed-worktree");
  await mkdir(worktree, { recursive: true });
  const logger = createTestLogger();
  const storage = new AgentStorage(path.join(root, "agents"), logger);
  const manager = new AgentManager({
    clients: createTestAgentClients(),
    registry: storage,
    logger,
  });

  const agentId = "00000000-0000-4000-8000-000000000501";

  try {
    const agent = await manager.createAgent({ provider: "codex", cwd: worktree }, agentId, {
      workspaceId: "workspace-worktree",
    });
    await startAgentRun(manager, agent.id, "what did you change", logger, {});
    // Dispatching a run does not finish it: the provider appends the reply to its
    // history afterwards, and the turn is finalized only once that append lands.
    // Archive after the turn is finalized so the transcript this test reads back is
    // already on disk when the worktree goes away.
    const finished = await manager.waitForAgentEvent(agent.id);
    expect(finished.status).toBe("idle");
    await manager.archiveAgent(agent.id);
    await manager.closeAgent(agent.id);
    await manager.flush();
    await storage.flush();

    // Archiving the workspace removes the worktree it owned. The agent's history is
    // persisted and reading it must not depend on that directory still being there.
    await rm(worktree, { recursive: true, force: true });

    const loaded = await ensureAgentLoaded(agentId, {
      agentManager: manager,
      agentStorage: storage,
      logger,
    });

    expect(loaded.id).toBe(agentId);
    // The transcript is replayed from the provider's persisted history, so the reply the
    // agent gave before the worktree went away is still readable.
    const replies = manager
      .getTimeline(agentId)
      .filter((item) => item.type === "assistant_message");
    expect(replies.length).toBeGreaterThan(0);
    expect(replies.every((item) => item.type === "assistant_message" && item.text.length > 0)).toBe(
      true,
    );
  } finally {
    await manager.closeAgent(agentId).catch(() => undefined);
    await manager.flush().catch(() => undefined);
    await storage.flush().catch(() => undefined);
    await rm(root, { recursive: true, force: true });
  }
});

describe("ensureAgentTimelineReadable", () => {
  interface Daemon {
    manager: AgentManager;
    storage: AgentStorage;
    store: SqliteTimelineStore;
  }

  let root: string;
  const daemons: Daemon[] = [];

  beforeEach(async () => {
    root = await mkdtemp(path.join(tmpdir(), "agent-loading-readable-"));
  });

  afterEach(async () => {
    for (const daemon of daemons.splice(0)) {
      for (const agent of daemon.manager.listAgents()) {
        await daemon.manager.closeAgent(agent.id).catch(() => undefined);
      }
      daemon.store.close();
    }
    await rm(root, { recursive: true, force: true });
  });

  /** One daemon process on the shared home: its own storage, cache handle, and manager. */
  async function startDaemon(): Promise<Daemon> {
    const logger = createTestLogger();
    const store = await openTimelineCacheStore(root, logger);
    if (!store) throw new Error("node:sqlite is unavailable");
    const storage = new AgentStorage(path.join(root, "agents"), logger);
    const manager = new AgentManager({
      clients: createTestAgentClients(),
      registry: storage,
      durableTimelineStore: store,
      logger,
    });
    const daemon = { manager, storage, store };
    daemons.push(daemon);
    return daemon;
  }

  /** Ends a daemon without closing its agents, as a crash would; their cache stays incomplete. */
  async function crashDaemon(daemon: Daemon): Promise<void> {
    await daemon.manager.flush();
    await daemon.storage.flush();
    daemons.splice(daemons.indexOf(daemon), 1);
    daemon.store.close();
  }

  async function createAgentWithTurn(daemon: Daemon): Promise<string> {
    const agent = await daemon.manager.createAgent({ provider: "claude", cwd: root }, undefined, {
      workspaceId: undefined,
    });
    await daemon.manager.runAgent(agent.id, "hello there");
    return agent.id;
  }

  function deps(daemon: Daemon) {
    return {
      agentManager: daemon.manager,
      agentStorage: daemon.storage,
      logger: createTestLogger(),
    };
  }

  test("reads a closed agent's cached timeline after a restart without a resume", async () => {
    const first = await startDaemon();
    const agentId = await createAgentWithTurn(first);
    const before = first.manager.fetchTimeline(agentId, { direction: "tail", limit: 0 });
    await first.manager.closeAgent(agentId);
    await crashDaemon(first);

    const second = await startDaemon();
    const resume = vi.spyOn(second.manager, "resumeAgentFromPersistence");
    const backfill = vi.spyOn(second.manager, "backfillTimelineFromProvider");

    const readable = await ensureAgentTimelineReadable(agentId, deps(second));

    expect(readable.kind).toBe("stored");
    expect(resume).not.toHaveBeenCalled();
    expect(backfill).not.toHaveBeenCalled();
    expect(second.manager.getAgent(agentId)).toBeNull();
    const after = second.manager.fetchTimelineForRead(agentId, { direction: "tail", limit: 0 });
    expect(after.epoch).toBe(before.epoch);
    expect(after.rows).toEqual(before.rows);
  });

  test("runs one backfill for concurrent reads of an agent with no cached timeline", async () => {
    const first = await startDaemon();
    const agentId = await createAgentWithTurn(first);
    const before = first.manager.fetchTimeline(agentId, { direction: "tail", limit: 0 });
    await crashDaemon(first);

    const second = await startDaemon();
    const resume = vi.spyOn(second.manager, "resumeAgentFromPersistence");
    const backfill = vi.spyOn(second.manager, "backfillTimelineFromProvider");

    const results = await Promise.all([
      ensureAgentTimelineReadable(agentId, deps(second)),
      ensureAgentTimelineReadable(agentId, deps(second)),
      ensureAgentTimelineReadable(agentId, deps(second)),
    ]);

    expect(results.map((result) => result.kind)).toEqual(["stored", "stored", "stored"]);
    expect(backfill).toHaveBeenCalledTimes(1);
    expect(resume).not.toHaveBeenCalled();
    const after = second.manager.fetchTimelineForRead(agentId, { direction: "tail", limit: 0 });
    expect(after.rows.length).toBeGreaterThan(0);
    expect(after.rows.map((row) => row.item)).toEqual(before.rows.map((row) => row.item));
  });

  test("resumes the agent when the backfill fails", async () => {
    const first = await startDaemon();
    const agentId = await createAgentWithTurn(first);
    await crashDaemon(first);

    const second = await startDaemon();
    vi.spyOn(second.manager, "backfillTimelineFromProvider").mockRejectedValue(
      new Error("provider history unreadable"),
    );
    const resume = vi.spyOn(second.manager, "resumeAgentFromPersistence");

    const readable = await ensureAgentTimelineReadable(agentId, deps(second));

    expect(readable.kind).toBe("live");
    expect(resume).toHaveBeenCalledTimes(1);
    expect(second.manager.getAgent(agentId)).not.toBeNull();
  });
});
