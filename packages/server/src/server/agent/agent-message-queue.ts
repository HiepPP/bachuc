import { randomUUID } from "node:crypto";
import { readdir, readFile, rm } from "node:fs/promises";
import path from "node:path";
import type { Logger } from "pino";
import { z } from "zod";
import {
  AgentQueuedMessageSchema,
  type AgentAttachment,
  type AgentMessageQueueSummary,
  type AgentQueuedMessage,
} from "@getpaseo/protocol/messages";

import { writeJsonFileAtomic } from "../atomic-file.js";
import type { AgentManager, AgentManagerEvent } from "./agent-manager.js";
import { sendPromptToAgent } from "./agent-prompt.js";
import type { AgentStorage } from "./agent-storage.js";
import { buildAgentPrompt } from "./prompt-attachments.js";

export const AGENT_MESSAGE_QUEUE_LIMIT = 50;

const StoredAgentQueueSchema = z.object({
  held: z.boolean(),
  items: z.array(AgentQueuedMessageSchema),
});

interface AgentQueueState {
  held: boolean;
  items: AgentQueuedMessage[];
}

export class AgentMessageQueueError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "AgentMessageQueueError";
  }
}

export interface AgentMessageQueueOptions {
  directory: string;
  agentManager: AgentManager;
  agentStorage: AgentStorage;
  logger: Logger;
}

export interface EnqueueAgentMessageInput {
  text: string;
  images?: Array<{ data: string; mimeType: string }>;
  attachments?: AgentAttachment[];
}

export interface AgentQueueResult {
  queue: AgentMessageQueueSummary;
}

// One queue per daemon, keyed by its manager, so lifecycle commands that only hold the
// manager can still hold a queue on Stop.
const queuesByManager = new WeakMap<object, AgentMessageQueue>();

export interface HoldAgentMessageQueuesInput {
  agentManager: object;
  agentIds: Iterable<string>;
}

export async function holdAgentMessageQueues(input: HoldAgentMessageQueuesInput): Promise<void> {
  const queue = queuesByManager.get(input.agentManager);
  if (!queue) {
    return;
  }
  await Promise.all(Array.from(input.agentIds, (agentId) => queue.hold(agentId)));
}

function summarizeItem(item: AgentQueuedMessage): AgentMessageQueueSummary["items"][number] {
  return {
    id: item.id,
    text: item.text,
    attachmentKinds: [
      ...(item.images ?? []).map(() => "image"),
      ...(item.attachments ?? []).map((attachment) => attachment.type),
    ],
    createdAt: item.createdAt,
  };
}

function summarize(state: AgentQueueState | undefined): AgentMessageQueueSummary {
  return { held: state?.held ?? false, items: (state?.items ?? []).map(summarizeItem) };
}

/**
 * The daemon-owned message queue. A queued message waits until its agent's run ends
 * normally, then the daemon starts it. The queue survives client close and daemon
 * restart, and every client sees it through the agent snapshot.
 */
export class AgentMessageQueue {
  private readonly states = new Map<string, AgentQueueState>();
  private readonly locks = new Map<string, Promise<unknown>>();
  private readonly lastLifecycle = new Map<string, string>();
  private readonly loaded: Promise<void>;
  private readonly unsubscribe: () => void;

  constructor(private readonly options: AgentMessageQueueOptions) {
    queuesByManager.set(options.agentManager, this);
    this.loaded = this.loadAll();
    this.unsubscribe = options.agentManager.subscribe((event) => this.handleEvent(event), {
      replayState: false,
    });
  }

  close(): void {
    this.unsubscribe();
    if (queuesByManager.get(this.options.agentManager) === this) {
      queuesByManager.delete(this.options.agentManager);
    }
  }

  /** The queue as clients see it, or undefined when the agent has none. */
  summary(agentId: string): AgentMessageQueueSummary | undefined {
    const state = this.states.get(agentId);
    if (!state || (state.items.length === 0 && !state.held)) {
      return undefined;
    }
    return summarize(state);
  }

  async enqueue(
    agentId: string,
    input: EnqueueAgentMessageInput,
  ): Promise<AgentQueueResult & { itemId: string }> {
    const result = await this.withLock(agentId, async () => {
      await this.assertNotArchived(agentId);
      const state = this.stateFor(agentId);
      if (state.items.length >= AGENT_MESSAGE_QUEUE_LIMIT) {
        throw new AgentMessageQueueError(
          `The queue already holds ${AGENT_MESSAGE_QUEUE_LIMIT} messages`,
        );
      }
      const item: AgentQueuedMessage = {
        id: randomUUID(),
        text: input.text,
        ...(input.images && input.images.length > 0 ? { images: input.images } : {}),
        ...(input.attachments && input.attachments.length > 0
          ? { attachments: input.attachments }
          : {}),
        createdAt: new Date().toISOString(),
      };
      state.items.push(item);
      await this.commit(agentId, state);
      return { itemId: item.id };
    });
    // A run can end between the client's queue decision and this enqueue.
    await this.drain(agentId);
    return { ...result, queue: summarize(this.states.get(agentId)) };
  }

  async edit(agentId: string, itemId: string, text: string): Promise<AgentQueueResult> {
    return this.withLock(agentId, async () => {
      const state = this.stateFor(agentId);
      const item = this.requireItem(state, itemId);
      item.text = text;
      await this.commit(agentId, state);
      return { queue: summarize(state) };
    });
  }

  async reorder(
    agentId: string,
    itemId: string,
    beforeItemId: string | null,
  ): Promise<AgentQueueResult> {
    return this.withLock(agentId, async () => {
      const state = this.stateFor(agentId);
      const item = this.requireItem(state, itemId);
      if (beforeItemId !== null && beforeItemId !== itemId) {
        this.requireItem(state, beforeItemId);
      }
      if (beforeItemId === itemId) {
        return { queue: summarize(state) };
      }
      state.items = state.items.filter((candidate) => candidate.id !== itemId);
      const beforeIndex =
        beforeItemId === null
          ? state.items.length
          : state.items.findIndex((candidate) => candidate.id === beforeItemId);
      state.items.splice(beforeIndex, 0, item);
      await this.commit(agentId, state);
      return { queue: summarize(state) };
    });
  }

  async cancel(
    agentId: string,
    itemId: string,
  ): Promise<AgentQueueResult & { item: AgentQueuedMessage }> {
    return this.withLock(agentId, async () => {
      const state = this.stateFor(agentId);
      const item = this.requireItem(state, itemId);
      state.items = state.items.filter((candidate) => candidate.id !== itemId);
      await this.commit(agentId, state);
      return { item, queue: summarize(state) };
    });
  }

  /** Sends the item now as a steer, and releases a held queue. */
  async promote(agentId: string, itemId: string): Promise<AgentQueueResult> {
    return this.withLock(agentId, async () => {
      const state = this.stateFor(agentId);
      const item = this.requireItem(state, itemId);
      state.items = state.items.filter((candidate) => candidate.id !== itemId);
      state.held = false;
      await this.commit(agentId, state);
      try {
        await this.send(agentId, item, "steer");
      } catch (error) {
        state.items.unshift(item);
        await this.commit(agentId, state);
        throw error;
      }
      return { queue: summarize(state) };
    });
  }

  async resume(agentId: string): Promise<AgentQueueResult> {
    await this.withLock(agentId, async () => {
      const state = this.stateFor(agentId);
      if (state.held) {
        state.held = false;
        await this.commit(agentId, state);
      }
    });
    // An agent that is already idle has no run end left to start the queue.
    await this.drain(agentId);
    return { queue: summarize(this.states.get(agentId)) };
  }

  /** Stop holds a non-empty queue so the stopped agent does not start its next message. */
  async hold(agentId: string): Promise<void> {
    await this.withLock(agentId, async () => {
      const state = this.states.get(agentId);
      if (!state || state.items.length === 0 || state.held) {
        return;
      }
      state.held = true;
      await this.commit(agentId, state);
    });
  }

  private handleEvent(event: AgentManagerEvent): void {
    if (event.type !== "agent_state") {
      return;
    }
    const agentId = event.agent.id;
    const previous = this.lastLifecycle.get(agentId);
    this.lastLifecycle.set(agentId, event.agent.lifecycle);
    // Only a normal end drains. An error end or a fresh load leaves the queue waiting.
    if (previous !== "running" || event.agent.lifecycle !== "idle") {
      return;
    }
    if (!this.states.get(agentId)?.items.length) {
      return;
    }
    void this.drain(agentId).catch((error: unknown) => {
      this.options.logger.error({ err: error, agentId }, "Failed to drain agent message queue");
    });
  }

  private isRunning(agentId: string): boolean {
    const { agentManager } = this.options;
    return (
      agentManager.hasInFlightRun(agentId) ||
      agentManager.getAgent(agentId)?.lifecycle === "running"
    );
  }

  private async drain(agentId: string): Promise<void> {
    await this.withLock(agentId, async () => {
      const state = this.states.get(agentId);
      if (!state || state.held || state.items.length === 0 || this.isRunning(agentId)) {
        return;
      }
      const item = state.items.shift();
      if (!item) {
        return;
      }
      await this.commit(agentId, state);
      try {
        await this.send(agentId, item, "start");
      } catch (error) {
        // A run that started after the idle check, or an archived agent, keeps the item.
        state.items.unshift(item);
        await this.commit(agentId, state);
        this.options.logger.warn(
          { err: error, agentId, itemId: item.id },
          "Queued agent message did not start; it stays at the head of the queue",
        );
      }
    });
  }

  // "start" never replaces a run that began after the idle check; "steer" is a promote.
  private async send(
    agentId: string,
    item: AgentQueuedMessage,
    mode: "start" | "steer",
  ): Promise<void> {
    await this.assertNotArchived(agentId);
    await sendPromptToAgent({
      agentManager: this.options.agentManager,
      agentStorage: this.options.agentStorage,
      agentId,
      prompt: buildAgentPrompt(item.text, item.images, item.attachments),
      messageId: item.id,
      ...(mode === "steer" ? { activeTurnBehavior: "steer" as const } : { replaceRunning: false }),
      unarchive: false,
      logger: this.options.logger,
    });
  }

  // sendPromptToAgent skips an archived agent without an error, which would drop the item.
  private async assertNotArchived(agentId: string): Promise<void> {
    const record = await this.options.agentStorage.get(agentId);
    if (record?.archivedAt) {
      throw new AgentMessageQueueError(`Agent ${agentId} is archived`);
    }
  }

  private stateFor(agentId: string): AgentQueueState {
    let state = this.states.get(agentId);
    if (!state) {
      state = { held: false, items: [] };
      this.states.set(agentId, state);
    }
    return state;
  }

  private requireItem(state: AgentQueueState, itemId: string): AgentQueuedMessage {
    const item = state.items.find((candidate) => candidate.id === itemId);
    if (!item) {
      throw new AgentMessageQueueError(`Queued message ${itemId} was already sent or removed`);
    }
    return item;
  }

  private async withLock<T>(agentId: string, run: () => Promise<T>): Promise<T> {
    await this.loaded;
    const previous = this.locks.get(agentId) ?? Promise.resolve();
    const next = previous.catch(() => undefined).then(run);
    this.locks.set(agentId, next);
    try {
      return await next;
    } finally {
      if (this.locks.get(agentId) === next) {
        this.locks.delete(agentId);
      }
    }
  }

  private filePath(agentId: string): string {
    if (agentId.includes("/") || agentId.includes("\\") || agentId.startsWith(".")) {
      throw new AgentMessageQueueError(`Invalid agent id for the message queue: ${agentId}`);
    }
    return path.join(this.options.directory, `${agentId}.json`);
  }

  // The empty queue releases its hold and its file, so a later run is never held by a
  // Stop that had nothing to hold.
  private async commit(agentId: string, state: AgentQueueState): Promise<void> {
    if (state.items.length === 0) {
      state.held = false;
    }
    const file = this.filePath(agentId);
    if (state.items.length === 0) {
      this.states.delete(agentId);
      await rm(file, { force: true });
    } else {
      // A failed send restores items into a state an earlier commit already dropped.
      this.states.set(agentId, state);
      await writeJsonFileAtomic(file, { held: state.held, items: state.items });
    }
    this.options.agentManager.notifyAgentState(agentId);
  }

  private async loadAll(): Promise<void> {
    let names: string[];
    try {
      names = await readdir(this.options.directory);
    } catch (error) {
      if (error instanceof Error && "code" in error && error.code === "ENOENT") {
        return;
      }
      throw error;
    }
    for (const name of names) {
      if (!name.endsWith(".json") || name.startsWith(".")) {
        continue;
      }
      const agentId = name.slice(0, -".json".length);
      try {
        const raw: unknown = JSON.parse(
          await readFile(path.join(this.options.directory, name), "utf8"),
        );
        const stored = StoredAgentQueueSchema.parse(raw);
        if (stored.items.length > 0) {
          this.states.set(agentId, { held: stored.held, items: stored.items });
        }
      } catch (error) {
        this.options.logger.warn(
          { err: error, agentId },
          "Skipping an unreadable agent message queue file",
        );
      }
    }
  }
}
