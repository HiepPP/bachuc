import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { PARENT_AGENT_ID_LABEL } from "@getpaseo/protocol/agent-labels";

import { createTestLogger } from "../../test-utils/test-logger.js";
import { AgentManager, type AgentManagerEvent, type ManagedAgent } from "./agent-manager.js";
import {
  AGENT_MESSAGE_QUEUE_LIMIT,
  AgentMessageQueue,
  holdAgentMessageQueues,
} from "./agent-message-queue.js";
import { sendPromptToAgent } from "./agent-prompt.js";
import { AgentStorage } from "./agent-storage.js";
import { archiveAgentCommand, cancelAgentRunCommand } from "./lifecycle-command.js";

// The queue's own logic is under test, so a send is recorded instead of run.
vi.mock("./agent-prompt.js", async (importOriginal) => ({
  ...(await importOriginal<typeof import("./agent-prompt.js")>()),
  sendPromptToAgent: vi.fn(async () => ({ disposition: "turn_started" })),
}));

const sendSpy = vi.mocked(sendPromptToAgent);

interface QueueScenario {
  agentManager: AgentManager;
  agentStorage: AgentStorage;
  setLifecycle(agentId: string, lifecycle: ManagedAgent["lifecycle"]): void;
  addChild(agentId: string, parentAgentId: string): void;
  archive(agentId: string): void;
  sentMessageIds(): Array<string | undefined>;
}

let directory: string;

beforeEach(() => {
  directory = mkdtempSync(join(tmpdir(), "agent-message-queue-test-"));
  sendSpy.mockClear();
});

afterEach(() => {
  rmSync(directory, { recursive: true, force: true });
});

function createScenario(): QueueScenario {
  const subscribers = new Set<(event: AgentManagerEvent) => void>();
  const agents = new Map<string, ManagedAgent>();
  const labels = new Map<string, Record<string, string>>();
  const archived = new Set<string>();

  function agentFor(agentId: string): ManagedAgent {
    let agent = agents.get(agentId);
    if (!agent) {
      agent = Object.create(null) as ManagedAgent;
      Reflect.set(agent, "id", agentId);
      Reflect.set(agent, "lifecycle", "idle");
      agents.set(agentId, agent);
    }
    return agent;
  }

  const agentManager = new AgentManager({ clients: {}, logger: createTestLogger() });
  Reflect.set(agentManager, "subscribe", (callback: (event: AgentManagerEvent) => void) => {
    subscribers.add(callback);
    return () => subscribers.delete(callback);
  });
  Reflect.set(agentManager, "getAgent", (agentId: string) => agents.get(agentId) ?? null);
  Reflect.set(
    agentManager,
    "hasInFlightRun",
    (agentId: string) => agents.get(agentId)?.lifecycle === "running",
  );
  Reflect.set(agentManager, "notifyAgentState", () => {});
  Reflect.set(agentManager, "listAgents", () =>
    Array.from(agents.keys(), (id) => ({ id, labels: labels.get(id) ?? {} })),
  );
  // Cancelling a running agent ends its run, which emits the idle state Stop must not drain.
  Reflect.set(agentManager, "cancelAgentRun", async (agentId: string) => {
    const agent = agentFor(agentId);
    if (agent.lifecycle !== "running") {
      return { status: "not_running" };
    }
    setLifecycle(agentId, "idle");
    return { status: "settled" };
  });
  Reflect.set(agentManager, "clearAgentAttention", async () => {});
  Reflect.set(agentManager, "archiveAgent", async (agentId: string) => {
    archived.add(agentId);
    return { archivedAt: "2026-10-06T00:00:00.000Z" };
  });

  const agentStorage = Object.create(AgentStorage.prototype) as AgentStorage;
  Reflect.set(agentStorage, "get", async (agentId: string) =>
    archived.has(agentId) ? { id: agentId, archivedAt: "2026-10-06T00:00:00.000Z" } : null,
  );

  function setLifecycle(agentId: string, lifecycle: ManagedAgent["lifecycle"]): void {
    const agent = agentFor(agentId);
    agent.lifecycle = lifecycle;
    for (const subscriber of Array.from(subscribers)) {
      subscriber({ type: "agent_state", agent });
    }
  }

  return {
    agentManager,
    agentStorage,
    setLifecycle,
    addChild(agentId, parentAgentId) {
      agentFor(agentId);
      labels.set(agentId, { [PARENT_AGENT_ID_LABEL]: parentAgentId });
    },
    archive(agentId) {
      archived.add(agentId);
    },
    sentMessageIds() {
      return sendSpy.mock.calls.map(([params]) => params.messageId);
    },
  };
}

function createQueue(scenario: QueueScenario): AgentMessageQueue {
  return new AgentMessageQueue({
    directory,
    agentManager: scenario.agentManager,
    agentStorage: scenario.agentStorage,
    logger: createTestLogger(),
  });
}

async function flushQueueWork(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 0));
}

// Operations on one agent run in order, so this hold resolves only after every drain
// queued before it has finished. The queue is already held, or empty, in these cases.
async function settleAgentLock(queue: AgentMessageQueue, agentId: string): Promise<void> {
  await queue.hold(agentId);
}

describe("AgentMessageQueue", () => {
  test("starts the head message when the agent's run ends normally", async () => {
    const scenario = createScenario();
    const queue = createQueue(scenario);
    scenario.setLifecycle("agent-1", "running");

    const first = await queue.enqueue("agent-1", { text: "First follow-up" });
    const second = await queue.enqueue("agent-1", { text: "Second follow-up" });
    expect(scenario.sentMessageIds()).toEqual([]);

    scenario.setLifecycle("agent-1", "idle");

    await vi.waitFor(() => expect(scenario.sentMessageIds()).toEqual([first.itemId]));
    expect(queue.summary("agent-1")?.items.map((item) => item.id)).toEqual([second.itemId]);
    // A drain never replaces a run that began after its idle check.
    expect(sendSpy.mock.calls[0]?.[0]).toMatchObject({ replaceRunning: false });
    queue.close();
  });

  test("a queued message whose send fails stays at the head of the queue", async () => {
    const scenario = createScenario();
    const queue = createQueue(scenario);
    scenario.setLifecycle("agent-1", "running");
    const first = await queue.enqueue("agent-1", { text: "First" });
    sendSpy.mockRejectedValueOnce(new Error("Agent agent-1 already has an active run"));

    scenario.setLifecycle("agent-1", "idle");
    await vi.waitFor(() => expect(scenario.sentMessageIds()).toEqual([first.itemId]));
    // An unchanged edit waits for the failed drain without holding the queue.
    await queue.edit("agent-1", first.itemId, "First");
    expect(queue.summary("agent-1")?.items.map((item) => item.id)).toEqual([first.itemId]);

    const second = await queue.enqueue("agent-1", { text: "Second" });
    expect(scenario.sentMessageIds()).toEqual([first.itemId, first.itemId]);
    expect(queue.summary("agent-1")?.items.map((item) => item.id)).toEqual([second.itemId]);
    queue.close();
  });

  test("an archived agent takes no new message and keeps the ones it had", async () => {
    const scenario = createScenario();
    const queue = createQueue(scenario);
    scenario.setLifecycle("agent-1", "running");
    const queued = await queue.enqueue("agent-1", { text: "Follow-up" });

    scenario.archive("agent-1");
    await expect(queue.enqueue("agent-1", { text: "Too late" })).rejects.toThrow(
      "Agent agent-1 is archived",
    );
    scenario.setLifecycle("agent-1", "idle");
    await flushQueueWork();
    await queue.edit("agent-1", queued.itemId, "Follow-up");

    expect(scenario.sentMessageIds()).toEqual([]);
    expect(queue.summary("agent-1")?.items.map((item) => item.id)).toEqual([queued.itemId]);
    expect(queue.summary("agent-1")?.held).toBe(false);
    queue.close();
  });

  test("archiving a running agent holds its queue before the cancel", async () => {
    const scenario = createScenario();
    const queue = createQueue(scenario);
    scenario.setLifecycle("agent-1", "running");
    await queue.enqueue("agent-1", { text: "Follow-up" });

    await archiveAgentCommand(
      {
        agentManager: scenario.agentManager,
        agentStorage: scenario.agentStorage,
        logger: createTestLogger(),
      },
      "agent-1",
    );
    await settleAgentLock(queue, "agent-1");

    expect(scenario.sentMessageIds()).toEqual([]);
    expect(queue.summary("agent-1")?.held).toBe(true);
    queue.close();
  });

  test("waits after an error end and after the first state of a restarted daemon", async () => {
    const scenario = createScenario();
    const queue = createQueue(scenario);
    scenario.setLifecycle("agent-1", "running");
    await queue.enqueue("agent-1", { text: "Follow-up" });

    scenario.setLifecycle("agent-1", "error");
    await flushQueueWork();
    expect(scenario.sentMessageIds()).toEqual([]);
    queue.close();

    const restartedScenario = createScenario();
    const restarted = createQueue(restartedScenario);
    restartedScenario.setLifecycle("agent-1", "idle");
    await restarted.hold("other-agent");
    await flushQueueWork();

    expect(restartedScenario.sentMessageIds()).toEqual([]);
    expect(restarted.summary("agent-1")?.items.map((item) => item.text)).toEqual(["Follow-up"]);
    restarted.close();
  });

  test("an enqueue on an idle agent starts at once", async () => {
    const scenario = createScenario();
    const queue = createQueue(scenario);

    const result = await queue.enqueue("agent-1", { text: "Run it" });

    expect(scenario.sentMessageIds()).toEqual([result.itemId]);
    expect(queue.summary("agent-1")).toBeUndefined();
    queue.close();
  });

  test("Stop holds the queue until the user resumes it", async () => {
    const scenario = createScenario();
    const queue = createQueue(scenario);
    scenario.setLifecycle("agent-1", "running");
    const queued = await queue.enqueue("agent-1", { text: "Follow-up" });

    await cancelAgentRunCommand(
      { agentManager: scenario.agentManager, logger: createTestLogger() },
      "agent-1",
    );
    await settleAgentLock(queue, "agent-1");

    expect(scenario.sentMessageIds()).toEqual([]);
    expect(queue.summary("agent-1")?.held).toBe(true);

    await queue.resume("agent-1");
    expect(scenario.sentMessageIds()).toEqual([queued.itemId]);
    queue.close();
  });

  test("Stop also holds the queues of running subagents", async () => {
    const scenario = createScenario();
    const queue = createQueue(scenario);
    scenario.addChild("child-1", "parent");
    scenario.setLifecycle("parent", "running");
    scenario.setLifecycle("child-1", "running");
    await queue.enqueue("child-1", { text: "Child follow-up" });

    await cancelAgentRunCommand(
      { agentManager: scenario.agentManager, logger: createTestLogger() },
      "parent",
    );
    await settleAgentLock(queue, "child-1");

    expect(scenario.sentMessageIds()).toEqual([]);
    expect(queue.summary("child-1")?.held).toBe(true);
    queue.close();
  });

  test("holding an empty queue leaves later runs free", async () => {
    const scenario = createScenario();
    const queue = createQueue(scenario);

    await holdAgentMessageQueues({ agentManager: scenario.agentManager, agentIds: ["agent-1"] });
    expect(queue.summary("agent-1")).toBeUndefined();
    queue.close();
  });

  test("rejects an enqueue past the queue limit", async () => {
    const scenario = createScenario();
    const queue = createQueue(scenario);
    scenario.setLifecycle("agent-1", "running");
    for (let index = 0; index < AGENT_MESSAGE_QUEUE_LIMIT; index += 1) {
      await queue.enqueue("agent-1", { text: `Message ${index}` });
    }

    await expect(queue.enqueue("agent-1", { text: "One too many" })).rejects.toThrow(
      `The queue already holds ${AGENT_MESSAGE_QUEUE_LIMIT} messages`,
    );
    queue.close();
  });

  test("edits, reorders, and cancels items by id", async () => {
    const scenario = createScenario();
    const queue = createQueue(scenario);
    scenario.setLifecycle("agent-1", "running");
    const a = await queue.enqueue("agent-1", { text: "A" });
    const b = await queue.enqueue("agent-1", {
      text: "B",
      images: [{ data: "aW1hZ2U=", mimeType: "image/png" }],
    });
    const c = await queue.enqueue("agent-1", { text: "C" });

    await queue.edit("agent-1", a.itemId, "A edited");
    await queue.reorder("agent-1", c.itemId, a.itemId);
    const reordered = await queue.reorder("agent-1", a.itemId, null);
    expect(reordered.queue.items.map((item) => item.text)).toEqual(["C", "B", "A edited"]);
    expect(reordered.queue.items[1]?.attachmentKinds).toEqual(["image"]);

    const cancelled = await queue.cancel("agent-1", b.itemId);
    expect(cancelled.item.images).toEqual([{ data: "aW1hZ2U=", mimeType: "image/png" }]);
    expect(cancelled.queue.items.map((item) => item.id)).toEqual([c.itemId, a.itemId]);

    await expect(queue.cancel("agent-1", b.itemId)).rejects.toThrow("was already sent or removed");
    queue.close();
  });

  test("promote sends the item as a steer and releases a hold", async () => {
    const scenario = createScenario();
    const queue = createQueue(scenario);
    scenario.setLifecycle("agent-1", "running");
    const queued = await queue.enqueue("agent-1", { text: "Do this now" });
    await queue.enqueue("agent-1", { text: "Later" });
    await queue.hold("agent-1");

    const result = await queue.promote("agent-1", queued.itemId);

    expect(sendSpy.mock.calls.at(-1)?.[0]).toMatchObject({
      messageId: queued.itemId,
      activeTurnBehavior: "steer",
    });
    expect(result.queue.held).toBe(false);
    queue.close();
  });

  test("a promote that races a drain sends the item once", async () => {
    const scenario = createScenario();
    const queue = createQueue(scenario);
    scenario.setLifecycle("agent-1", "running");
    const queued = await queue.enqueue("agent-1", { text: "Only once" });

    scenario.setLifecycle("agent-1", "idle");
    const promote = queue.promote("agent-1", queued.itemId);
    await expect(promote).rejects.toThrow("was already sent or removed");
    await flushQueueWork();

    expect(scenario.sentMessageIds()).toEqual([queued.itemId]);
    queue.close();
  });
});
