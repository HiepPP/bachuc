import { expect, it, test, vi } from "vitest";
import pino, { type Logger } from "pino";
import { randomUUID } from "node:crypto";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { createTestLogger } from "../../test-utils/test-logger.js";
import { AgentManager } from "./agent-manager.js";
import { AgentStorage } from "./agent-storage.js";
import {
  disarmFinishNotifications,
  FINISH_NOTICE_WINDOW_MS,
  type FinishNotificationWake,
  formatSystemNotificationPrompt,
  isSystemInjectedEnvelope,
  setupFinishNotification,
  waitForAgentRunStartWithTimeout,
} from "./agent-prompt.js";
import { cancelAgentRunCommand } from "./lifecycle-command.js";
import type { AgentManagerEvent, ManagedAgent } from "./agent-manager.js";
import type {
  AgentClient,
  AgentRunResult,
  AgentSession,
  AgentStreamEvent,
} from "./agent-sdk-types.js";

interface CapturedLogger {
  logger: Logger;
  records: Array<Record<string, unknown>>;
  nextRecord: Promise<void>;
}

function createCapturedLogger(): CapturedLogger {
  const records: Array<Record<string, unknown>> = [];
  let resolveNextRecord!: () => void;
  const nextRecord = new Promise<void>((resolve) => {
    resolveNextRecord = resolve;
  });
  const logger = pino(
    { level: "error" },
    {
      write(line: string) {
        records.push(JSON.parse(line) as Record<string, unknown>);
        resolveNextRecord();
      },
    },
  );
  return { logger, records, nextRecord };
}

interface FinishNotificationScenarioOptions {
  childLastAssistantMessage?: string | null;
  childParentAgentId?: string | null;
  requireParentOwnership?: boolean;
  parentPromptError?: Error;
  logger?: Logger;
  gateStorageReads?: boolean;
  coalesceWindowMs?: number;
}

interface FinishNotificationScenario {
  startWatchingChild(): void;
  disarmCallers(callerAgentIds: string[]): void;
  releaseStorageReads(): void;
  stopCallerWhileChildRuns(): Promise<void>;
  requestChildPermission(requestId?: string): void;
  resolveChildPermission(requestId?: string): void;
  resolveChildPermissionFromState(requestId?: string): void;
  resolveChildPermissionWhileIdle(requestId?: string): void;
  finishChild(): void;
  finishChildAndReadParentPrompt(): Promise<string>;
  closeChildAndReadParentPrompt(): Promise<string>;
  parentPrompts(): string[];
  steerAttemptCount(): number;
  wasParentPrompted(): boolean;
}

// The scenario's storage and manager stubs resolve at once, so one macrotask drains
// every pending notice delivery before an absence assertion.
async function flushAsyncWork(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 0));
}

function createFinishNotificationScenario(
  options?: FinishNotificationScenarioOptions,
): FinishNotificationScenario {
  let subscriber: ((event: AgentManagerEvent) => void) | null = null;
  let resolveParentPrompt: ((prompt: string) => void) | null = null;
  let parentPrompted = false;
  let steerAttemptCount = 0;
  const parentPrompts: string[] = [];

  const childAgent: ManagedAgent = Object.create(null);
  Reflect.set(childAgent, "id", "child-agent");
  Reflect.set(childAgent, "lifecycle", "idle");
  Reflect.set(childAgent, "config", { title: "Child Agent" });
  Reflect.set(childAgent, "pendingPermissions", new Map());

  const callerAgent: ManagedAgent = Object.create(null);
  Reflect.set(callerAgent, "id", "caller-agent");
  Reflect.set(callerAgent, "lifecycle", "idle");
  Reflect.set(callerAgent, "config", { title: "Caller Agent" });

  const agentManager = new AgentManager({ clients: {}, logger: createTestLogger() });
  Reflect.set(agentManager, "getAgent", (agentId: string) => {
    if (agentId === "child-agent") {
      return childAgent;
    }
    if (agentId === "caller-agent") {
      return callerAgent;
    }
    return null;
  });
  Reflect.set(agentManager, "subscribe", (callback: (event: AgentManagerEvent) => void) => {
    subscriber = callback;
    return () => {
      subscriber = null;
    };
  });
  Reflect.set(agentManager, "getLastAssistantMessage", async () => {
    return options?.childLastAssistantMessage ?? null;
  });
  Reflect.set(agentManager, "tryRunOutOfBand", () => false);
  Reflect.set(agentManager, "hasInFlightRun", () => Boolean(options?.parentPromptError));
  Reflect.set(agentManager, "steerOrReplaceActiveTurn", async () => {
    steerAttemptCount += 1;
    return { status: "inactive" };
  });
  Reflect.set(agentManager, "streamAgent", (_agentId: string, prompt: string) => {
    parentPrompted = true;
    parentPrompts.push(prompt);
    resolveParentPrompt?.(prompt);
    return (async function* noop() {})();
  });
  Reflect.set(agentManager, "replaceAgentRun", async (_agentId: string, prompt: string) => {
    resolveParentPrompt?.(prompt);
    throw options?.parentPromptError;
  });

  let releaseStorageReads: () => void = () => {};
  const storageGate = options?.gateStorageReads
    ? new Promise<void>((resolve) => {
        releaseStorageReads = resolve;
      })
    : Promise.resolve();

  const agentStorage: AgentStorage = Object.create(AgentStorage.prototype);
  Reflect.set(agentStorage, "get", async (agentId: string) => {
    await storageGate;
    if (agentId === "child-agent") {
      const parentAgentId =
        options?.childParentAgentId === undefined ? "caller-agent" : options.childParentAgentId;
      return {
        title: "Child Agent",
        labels: parentAgentId ? { "paseo.parent-agent-id": parentAgentId } : {},
      };
    }
    return null;
  });

  return {
    startWatchingChild() {
      setupFinishNotification({
        agentManager,
        agentStorage,
        childAgentId: "child-agent",
        callerAgentId: "caller-agent",
        requireParentOwnership: options?.requireParentOwnership,
        // These cases cover one child, so no notice waits for a sibling.
        coalesceWindowMs: options?.coalesceWindowMs ?? 0,
        logger: options?.logger ?? createTestLogger(),
      });
    },
    disarmCallers(callerAgentIds) {
      disarmFinishNotifications({ agentManager, callerAgentIds: new Set(callerAgentIds) });
    },
    releaseStorageReads() {
      releaseStorageReads();
    },
    async stopCallerWhileChildRuns() {
      childAgent.lifecycle = "running";
      subscriber?.({ type: "agent_state", agent: childAgent });
      Reflect.set(agentManager, "listAgents", () => [
        { id: "caller-agent", labels: {} },
        { id: "child-agent", labels: { "paseo.parent-agent-id": "caller-agent" } },
      ]);
      Reflect.set(
        agentManager,
        "hasInFlightRun",
        (agentId: string) => agentId === "child-agent" && childAgent.lifecycle === "running",
      );
      // The child settles inside its cancel, which is when a live notice would fire.
      Reflect.set(agentManager, "cancelAgentRun", async () => {
        childAgent.lifecycle = "idle";
        subscriber?.({ type: "agent_state", agent: childAgent });
        return { status: "settled" };
      });
      await cancelAgentRunCommand({ agentManager, logger: createTestLogger() }, "caller-agent");
    },
    requestChildPermission(requestId = "permission-1") {
      childAgent.lifecycle = "running";
      childAgent.pendingPermissions.set(requestId, {
        id: requestId,
        provider: "claude",
        kind: "tool",
        name: "Run command",
        description: "Write the QA sentinel",
        input: {
          file_path: "/tmp/permission-qa.txt",
          content: "PASEO_PERMISSION_NOTIFY_QA_OK\n",
        },
      });
      subscriber?.({
        type: "agent_state",
        agent: childAgent,
      });
      subscriber?.({
        type: "agent_stream",
        agentId: "child-agent",
        event: {
          type: "permission_requested",
          provider: "codex",
          request: childAgent.pendingPermissions.get(requestId)!,
        },
      });
    },
    resolveChildPermission(requestId = "permission-1") {
      childAgent.pendingPermissions.delete(requestId);
      subscriber?.({
        type: "agent_stream",
        agentId: "child-agent",
        event: {
          type: "permission_resolved",
          provider: "codex",
          requestId,
          resolution: { behavior: "allow" },
        },
      });
    },
    resolveChildPermissionFromState(requestId = "permission-1") {
      childAgent.pendingPermissions.delete(requestId);
      subscriber?.({ type: "agent_state", agent: childAgent });
    },
    resolveChildPermissionWhileIdle(requestId = "permission-1") {
      childAgent.pendingPermissions.delete(requestId);
      childAgent.lifecycle = "idle";
      subscriber?.({ type: "agent_state", agent: childAgent });
      subscriber?.({
        type: "agent_stream",
        agentId: "child-agent",
        event: {
          type: "permission_resolved",
          provider: "codex",
          requestId,
          resolution: { behavior: "allow" },
        },
      });
    },
    finishChild() {
      childAgent.lifecycle = "running";
      subscriber?.({
        type: "agent_state",
        agent: childAgent,
      });

      childAgent.lifecycle = "idle";
      subscriber?.({
        type: "agent_state",
        agent: childAgent,
      });
    },
    async finishChildAndReadParentPrompt() {
      const parentPrompt = new Promise<string>((resolve) => {
        resolveParentPrompt = resolve;
      });
      this.finishChild();

      return parentPrompt;
    },
    async closeChildAndReadParentPrompt() {
      const parentPrompt = new Promise<string>((resolve) => {
        resolveParentPrompt = resolve;
      });

      childAgent.lifecycle = "running";
      subscriber?.({
        type: "agent_state",
        agent: childAgent,
      });

      childAgent.lifecycle = "closed";
      subscriber?.({
        type: "agent_state",
        agent: childAgent,
      });

      return parentPrompt;
    },
    parentPrompts() {
      return parentPrompts;
    },
    steerAttemptCount() {
      return steerAttemptCount;
    },
    wasParentPrompted() {
      return parentPrompted;
    },
  };
}

test("isSystemInjectedEnvelope matches the envelope formatSystemNotificationPrompt produces", () => {
  expect(isSystemInjectedEnvelope(formatSystemNotificationPrompt("child finished"))).toBe(true);
  expect(isSystemInjectedEnvelope("hello world")).toBe(false);
});

test("finish notifications tell the parent the child's last assistant message", async () => {
  const scenario = createFinishNotificationScenario({
    childLastAssistantMessage: "Implemented the cleanup and all checks pass.",
  });

  scenario.startWatchingChild();
  const parentPrompt = await scenario.finishChildAndReadParentPrompt();

  expect(parentPrompt).toEqual(
    formatSystemNotificationPrompt(
      "Agent child-agent (Child Agent) finished.\n\n<agent-response>\nImplemented the cleanup and all checks pass.\n</agent-response>",
    ),
  );
  expect(scenario.steerAttemptCount()).toBe(1);
});

test("finish notifications truncate oversized child responses", async () => {
  const included = "x".repeat(4000);
  const omitted = "TAIL-MARKER".repeat(50);
  const scenario = createFinishNotificationScenario({
    childLastAssistantMessage: included + omitted,
  });

  scenario.startWatchingChild();
  const parentPrompt = await scenario.finishChildAndReadParentPrompt();

  expect(parentPrompt).toContain(included);
  expect(parentPrompt).toContain(
    `[truncated ${omitted.length} chars; use get_agent_activity for the full response]`,
  );
  expect(parentPrompt).not.toContain("TAIL-MARKER");
});

test("a disarmed caller gets no notice when its child finishes", () => {
  const scenario = createFinishNotificationScenario();

  scenario.startWatchingChild();
  scenario.disarmCallers(["caller-agent"]);
  scenario.finishChild();

  expect(scenario.wasParentPrompted()).toBe(false);
});

test("stopping the caller cancels its running child without a finish notice", async () => {
  const scenario = createFinishNotificationScenario();

  scenario.startWatchingChild();
  await scenario.stopCallerWhileChildRuns();
  await flushAsyncWork();

  expect(scenario.wasParentPrompted()).toBe(false);
});

test("disarm cancels a notice whose delivery is already in flight", async () => {
  const scenario = createFinishNotificationScenario({ gateStorageReads: true });

  scenario.startWatchingChild();
  scenario.finishChild();
  scenario.disarmCallers(["caller-agent"]);
  scenario.releaseStorageReads();
  await flushAsyncWork();

  expect(scenario.wasParentPrompted()).toBe(false);
});

test("an in-flight notice still lands when nobody disarms it", async () => {
  const scenario = createFinishNotificationScenario({ gateStorageReads: true });

  scenario.startWatchingChild();
  scenario.finishChild();
  scenario.releaseStorageReads();

  await vi.waitFor(() => expect(scenario.wasParentPrompted()).toBe(true));
});

test("disarming another caller keeps the notice armed", async () => {
  const scenario = createFinishNotificationScenario();

  scenario.startWatchingChild();
  scenario.disarmCallers(["unrelated-agent"]);
  const parentPrompt = await scenario.finishChildAndReadParentPrompt();

  expect(parentPrompt).toContain("Agent child-agent (Child Agent) finished.");
});

test("closing a watched child notifies the caller", async () => {
  const scenario = createFinishNotificationScenario();

  scenario.startWatchingChild();
  const parentPrompt = await scenario.closeChildAndReadParentPrompt();

  expect(parentPrompt).toEqual(
    formatSystemNotificationPrompt("Agent child-agent (Child Agent) was closed."),
  );
});

test("finish notifications survive permission responses", async () => {
  const scenario = createFinishNotificationScenario();

  scenario.startWatchingChild();
  scenario.requestChildPermission();

  await vi.waitFor(() => {
    expect(scenario.parentPrompts()).toHaveLength(1);
  });
  expect(scenario.parentPrompts()[0]).toContain("needs permission.");
  const permissionPayload = scenario
    .parentPrompts()[0]
    .match(/<permission-request>\n([\s\S]+?)\n<\/permission-request>/)?.[1];
  expect(permissionPayload).toBeDefined();
  expect(JSON.parse(permissionPayload!)).toEqual({
    agentId: "child-agent",
    requestId: "permission-1",
    request: {
      id: "permission-1",
      provider: "claude",
      kind: "tool",
      name: "Run command",
      description: "Write the QA sentinel",
      input: {
        file_path: "/tmp/permission-qa.txt",
        content: "PASEO_PERMISSION_NOTIFY_QA_OK\n",
      },
    },
  });

  scenario.resolveChildPermission();
  scenario.finishChild();

  await vi.waitFor(() => {
    expect(scenario.parentPrompts()).toHaveLength(2);
  });
  expect(scenario.parentPrompts()[1]).toContain("finished.");
});

test("an idle permission resolution waits for the resumed run to finish", async () => {
  const scenario = createFinishNotificationScenario();

  scenario.startWatchingChild();
  scenario.requestChildPermission();
  await vi.waitFor(() => expect(scenario.parentPrompts()).toHaveLength(1));

  scenario.resolveChildPermissionWhileIdle();
  scenario.requestChildPermission("permission-2");
  await vi.waitFor(() => expect(scenario.parentPrompts()).toHaveLength(2));
  expect(scenario.parentPrompts().every((prompt) => prompt.includes("needs permission."))).toBe(
    true,
  );

  scenario.resolveChildPermission("permission-2");
  scenario.finishChild();
  await vi.waitFor(() => expect(scenario.parentPrompts()).toHaveLength(3));
  expect(scenario.parentPrompts()[2]).toContain("finished.");
});

test("finish notifications report every concurrently pending permission", async () => {
  const scenario = createFinishNotificationScenario();

  scenario.startWatchingChild();
  scenario.requestChildPermission("permission-1");
  scenario.requestChildPermission("permission-2");

  await vi.waitFor(() => expect(scenario.parentPrompts()).toHaveLength(2));
  expect(
    scenario.parentPrompts().map((prompt) => {
      const payload = prompt.match(/<permission-request>\n([\s\S]+?)\n<\/permission-request>/)?.[1];
      return JSON.parse(payload!).requestId;
    }),
  ).toEqual(["permission-1", "permission-2"]);

  scenario.resolveChildPermission("permission-1");
  scenario.resolveChildPermission("permission-2");
  scenario.finishChild();

  await vi.waitFor(() => expect(scenario.parentPrompts()).toHaveLength(3));
  expect(scenario.parentPrompts()[2]).toContain("finished.");
});

test("finish notifications survive repeated permission cycles", async () => {
  const scenario = createFinishNotificationScenario();

  scenario.startWatchingChild();
  scenario.requestChildPermission();
  await vi.waitFor(() => expect(scenario.parentPrompts()).toHaveLength(1));
  scenario.resolveChildPermissionFromState();

  scenario.requestChildPermission();
  await vi.waitFor(() => expect(scenario.parentPrompts()).toHaveLength(2));
  scenario.resolveChildPermission();
  scenario.finishChild();

  await vi.waitFor(() => expect(scenario.parentPrompts()).toHaveLength(3));
  expect(
    scenario.parentPrompts().map((prompt) => prompt.match(/(needs permission|finished)\./)?.[1]),
  ).toEqual(["needs permission", "needs permission", "finished"]);
});

test("detaching a child ends its parent-owned finish notification", async () => {
  const scenario = createFinishNotificationScenario({
    childParentAgentId: null,
    requireParentOwnership: true,
  });
  scenario.startWatchingChild();
  scenario.finishChild();
  await new Promise((resolve) => setTimeout(resolve, 0));
  expect(scenario.wasParentPrompted()).toBe(false);
});

test("follow-up finish notifications do not require a parent relationship", async () => {
  const scenario = createFinishNotificationScenario({ childParentAgentId: "another-agent" });

  scenario.startWatchingChild();
  const parentPrompt = await scenario.finishChildAndReadParentPrompt();

  expect(parentPrompt).toContain("Agent child-agent (Child Agent) finished.");
});

test("finish notifications log a rejected parent prompt without an unhandled rejection", async () => {
  const captured = createCapturedLogger();
  const scenario = createFinishNotificationScenario({
    parentPromptError: new Error("parent provider rejected replacement"),
    logger: captured.logger,
  });

  scenario.startWatchingChild();
  await scenario.finishChildAndReadParentPrompt();
  await captured.nextRecord;

  expect(captured.records).toEqual([
    expect.objectContaining({
      msg: "Failed to notify caller agent",
      childAgentId: "child-agent",
      callerAgentId: "caller-agent",
      reason: "finished",
      err: expect.objectContaining({ message: "parent provider rejected replacement" }),
    }),
  ]);
});

it("does not notify archived callers", async () => {
  let subscriber: ((event: AgentManagerEvent) => void) | null = null;

  const childAgent: ManagedAgent = Object.create(null);
  Reflect.set(childAgent, "id", "child-agent");
  Reflect.set(childAgent, "lifecycle", "idle");
  Reflect.set(childAgent, "config", { title: "Child Agent" });
  Reflect.set(childAgent, "pendingPermissions", new Map());

  const callerAgent: ManagedAgent = Object.create(null);
  Reflect.set(callerAgent, "id", "caller-agent");
  Reflect.set(callerAgent, "lifecycle", "idle");
  Reflect.set(callerAgent, "config", { title: "Caller Agent" });

  const streamAgentSpy = vi.fn(() => (async function* noop() {})());
  const replaceAgentRunSpy = vi.fn(() => (async function* noop() {})());

  const agentManager = new AgentManager({ clients: {}, logger: createTestLogger() });
  Reflect.set(
    agentManager,
    "getAgent",
    vi.fn((agentId: string) => {
      if (agentId === "child-agent") {
        return childAgent;
      }
      if (agentId === "caller-agent") {
        return callerAgent;
      }
      return null;
    }),
  );
  Reflect.set(
    agentManager,
    "subscribe",
    vi.fn((callback: (event: AgentManagerEvent) => void) => {
      subscriber = callback;
      return () => {
        subscriber = null;
      };
    }),
  );
  Reflect.set(agentManager, "hasInFlightRun", vi.fn().mockReturnValue(false));
  Reflect.set(agentManager, "streamAgent", streamAgentSpy);
  Reflect.set(agentManager, "replaceAgentRun", replaceAgentRunSpy);

  const agentStorageGetSpy = vi.fn(async (agentId: string) =>
    agentId === "caller-agent" ? { archivedAt: "2024-01-01" } : null,
  );
  const agentStorage: AgentStorage = Object.create(AgentStorage.prototype);
  Reflect.set(agentStorage, "get", agentStorageGetSpy);

  setupFinishNotification({
    agentManager,
    agentStorage,
    childAgentId: "child-agent",
    callerAgentId: "caller-agent",
    logger: createTestLogger(),
  });

  expect(subscriber).not.toBeNull();

  childAgent.lifecycle = "running";
  subscriber?.({
    type: "agent_state",
    agent: childAgent,
  });

  childAgent.lifecycle = "idle";
  subscriber?.({
    type: "agent_state",
    agent: childAgent,
  });

  await vi.waitFor(() => {
    expect(agentStorageGetSpy).toHaveBeenCalledWith("caller-agent");
  });

  expect(streamAgentSpy).not.toHaveBeenCalled();
  expect(replaceAgentRunSpy).not.toHaveBeenCalled();
});

// Deliberately independent literals rather than the production constants these tests
// guard: deriving the boundaries from AGENT_RUN_START_TIMEOUT_MS would keep the tests
// green if that constant were shortened back under a provider's startup budget.
const EXPECTED_RUN_START_BUDGET_MS = 60_000;
// The slowest provider startup budget the run-start wait has to sit outside of today
// (OpenCode's OPENCODE_SERVER_STARTUP_TIMEOUT_MS).
const SLOWEST_PROVIDER_STARTUP_BUDGET_MS = 30_000;

const RUN_START_TEST_CAPABILITIES = {
  supportsStreaming: false,
  supportsSessionPersistence: false,
  supportsSessionListing: true,
  supportsDynamicModes: false,
  supportsMcpServers: false,
  supportsReasoningStream: false,
  supportsToolInvocations: false,
} as const;

/**
 * Provider session whose turn start is held open for a configurable span, so the real
 * AgentManager run-state transition (pendingRun.started -> lifecycle "running" ->
 * agent_state) is what the run-start wait observes. `startDelayMs: null` never starts.
 */
class SlowStartAgentSession implements AgentSession {
  readonly provider = "codex" as const;
  readonly capabilities = RUN_START_TEST_CAPABILITIES;
  readonly id = randomUUID();
  private readonly subscribers = new Set<(event: AgentStreamEvent) => void>();
  private releaseStartTurn!: () => void;
  private readonly released = new Promise<void>((resolve) => {
    this.releaseStartTurn = resolve;
  });

  constructor(private readonly startDelayMs: number | null) {}

  async run(): Promise<AgentRunResult> {
    return { sessionId: this.id, finalText: "", timeline: [] };
  }

  /** Teardown hook so a never-starting turn cannot wedge the suite. */
  release(): void {
    this.releaseStartTurn();
  }

  async startTurn(): Promise<{ turnId: string }> {
    await new Promise<void>((resolve) => {
      if (this.startDelayMs !== null) {
        setTimeout(resolve, this.startDelayMs);
      }
      void this.released.then(resolve);
    });
    const turnId = "turn-1";
    setTimeout(() => {
      this.pushEvent({ type: "turn_started", provider: this.provider, turnId });
      this.pushEvent({ type: "turn_completed", provider: this.provider, turnId });
    }, 0);
    return { turnId };
  }

  subscribe(callback: (event: AgentStreamEvent) => void): () => void {
    this.subscribers.add(callback);
    return () => {
      this.subscribers.delete(callback);
    };
  }

  pushEvent(event: AgentStreamEvent): void {
    for (const callback of this.subscribers) {
      callback(event);
    }
  }

  async *streamHistory(): AsyncGenerator<AgentStreamEvent> {}

  async getRuntimeInfo() {
    return { provider: this.provider, sessionId: this.id, model: null, modeId: null };
  }

  async getAvailableModes() {
    return [];
  }

  async getCurrentMode() {
    return null;
  }

  async setMode(): Promise<void> {}

  getPendingPermissions() {
    return [];
  }

  async respondToPermission(): Promise<void> {}

  describePersistence() {
    return { provider: this.provider, sessionId: this.id };
  }

  async interrupt(): Promise<void> {}

  async close(): Promise<void> {}
}

class SlowStartAgentClient implements AgentClient {
  readonly provider = "codex" as const;
  readonly capabilities = RUN_START_TEST_CAPABILITIES;
  readonly sessions: SlowStartAgentSession[] = [];

  constructor(private readonly startDelayMs: number | null) {}

  async isAvailable(): Promise<boolean> {
    return true;
  }

  async createSession(): Promise<AgentSession> {
    const session = new SlowStartAgentSession(this.startDelayMs);
    this.sessions.push(session);
    return session;
  }

  async fetchCatalog() {
    return { models: [], modes: [] };
  }

  async resumeSession(): Promise<AgentSession> {
    return await this.createSession();
  }
}

/**
 * Real AgentManager driving a real agent, so the run-start wait exercises the production
 * run-state and agent_state subscription path rather than a replaced method.
 */
async function createRunStartScenario(startDelayMs: number | null): Promise<{
  agentManager: AgentManager;
  agentId: string;
  startRun: () => Promise<void>;
  cleanup: () => Promise<void>;
}> {
  const workdir = mkdtempSync(join(tmpdir(), "agent-run-start-budget-"));
  const client = new SlowStartAgentClient(startDelayMs);
  const agentManager = new AgentManager({
    clients: { codex: client },
    logger: createTestLogger(),
  });
  const snapshot = await agentManager.createAgent({ provider: "codex", cwd: workdir }, undefined, {
    workspaceId: undefined,
  });

  let drained: Promise<void> = Promise.resolve();
  return {
    agentManager,
    agentId: snapshot.id,
    // streamAgent registers the pending run synchronously, so the wait always observes it.
    startRun: async () => {
      const run = agentManager.streamAgent(snapshot.id, "start the run");
      drained = (async () => {
        for await (const _event of run) {
          // Drain whatever the turn produces.
        }
      })().catch(() => undefined);
    },
    cleanup: async () => {
      // Release any turn still held open, then close. The drain is deliberately not
      // awaited: depending on how far the turn got, the stream ends either from the
      // release or from the close, and teardown must not depend on which.
      for (const session of client.sessions) {
        session.release();
      }
      await agentManager.closeAgent(snapshot.id).catch(() => undefined);
      void drained;
      rmSync(workdir, { recursive: true, force: true });
    },
  };
}

test("waiting for a run start outlasts the slowest provider startup budget", async () => {
  // A provider is still allowed to be starting here, so the outer wait must not abort it.
  const scenario = await createRunStartScenario(SLOWEST_PROVIDER_STARTUP_BUDGET_MS + 5_000);
  vi.useFakeTimers();

  try {
    await scenario.startRun();
    const wait = waitForAgentRunStartWithTimeout(scenario.agentManager, scenario.agentId);
    let settled = false;
    const markSettled = () => {
      settled = true;
    };
    void wait.then(markSettled, markSettled);

    await vi.advanceTimersByTimeAsync(SLOWEST_PROVIDER_STARTUP_BUDGET_MS);
    expect(settled).toBe(false);
    expect(scenario.agentManager.getAgent(scenario.agentId)?.lifecycle).not.toBe("running");

    await vi.advanceTimersByTimeAsync(5_000);
    await expect(wait).resolves.toBeUndefined();
    expect(scenario.agentManager.getAgent(scenario.agentId)?.lifecycle).toBe("running");
  } finally {
    vi.useRealTimers();
    await scenario.cleanup();
  }
});

test("waiting for a run start still gives up at the run start budget", async () => {
  const scenario = await createRunStartScenario(null);
  vi.useFakeTimers();

  try {
    await scenario.startRun();
    const wait = waitForAgentRunStartWithTimeout(scenario.agentManager, scenario.agentId);
    const rejection = expect(wait).rejects.toThrow(
      "codex run did not start within 60 seconds (phase: run start)",
    );
    let settled = false;
    const markSettled = () => {
      settled = true;
    };
    void wait.then(markSettled, markSettled);

    await vi.advanceTimersByTimeAsync(EXPECTED_RUN_START_BUDGET_MS - 1);
    expect(settled).toBe(false);

    await vi.advanceTimersByTimeAsync(1);
    await rejection;
    expect(settled).toBe(true);
  } finally {
    vi.useRealTimers();
    await scenario.cleanup();
  }
});

interface MailboxScenario {
  watchChildren(options?: { wake?: FinishNotificationWake }): void;
  finishChild(childAgentId: string): void;
  requestPermission(childAgentId: string): void;
  setCallerRunning(running: boolean): void;
  disarmCaller(): void;
  callerPrompts(): string[];
  subscriberCount(agentId: string): number;
}

// Several children watched by one caller, to cover the coalescing window.
function createMailboxScenario(childAgentIds: string[]): MailboxScenario {
  const subscribers = new Map<string, Set<(event: AgentManagerEvent) => void>>();
  const callerPrompts: string[] = [];
  let callerRunning = false;

  function createAgent(id: string, lifecycle: ManagedAgent["lifecycle"]): ManagedAgent {
    const agent: ManagedAgent = Object.create(null);
    Reflect.set(agent, "id", id);
    Reflect.set(agent, "lifecycle", lifecycle);
    Reflect.set(agent, "config", { title: id });
    Reflect.set(agent, "pendingPermissions", new Map());
    return agent;
  }
  const children = new Map(childAgentIds.map((id) => [id, createAgent(id, "idle")]));
  const caller = createAgent("caller-agent", "idle");

  function emit(agentId: string, event: AgentManagerEvent): void {
    for (const subscriber of Array.from(subscribers.get(agentId) ?? [])) {
      subscriber(event);
    }
  }

  const agentManager = new AgentManager({ clients: {}, logger: createTestLogger() });
  Reflect.set(agentManager, "getAgent", (agentId: string) =>
    agentId === "caller-agent" ? caller : (children.get(agentId) ?? null),
  );
  Reflect.set(
    agentManager,
    "subscribe",
    (callback: (event: AgentManagerEvent) => void, options: { agentId: string }) => {
      const set = subscribers.get(options.agentId) ?? new Set();
      set.add(callback);
      subscribers.set(options.agentId, set);
      return () => set.delete(callback);
    },
  );
  Reflect.set(agentManager, "getLastAssistantMessage", async () => null);
  Reflect.set(agentManager, "tryRunOutOfBand", () => false);
  Reflect.set(
    agentManager,
    "hasInFlightRun",
    (agentId: string) => agentId === "caller-agent" && callerRunning,
  );
  Reflect.set(
    agentManager,
    "steerOrReplaceActiveTurn",
    async (_agentId: string, prompt: string) => {
      callerPrompts.push(prompt);
      return { status: "steered" };
    },
  );
  Reflect.set(agentManager, "streamAgent", (_agentId: string, prompt: string) => {
    callerPrompts.push(prompt);
    return (async function* noop() {})();
  });

  const agentStorage: AgentStorage = Object.create(AgentStorage.prototype);
  Reflect.set(agentStorage, "get", async (agentId: string) =>
    children.has(agentId) ? { title: agentId, labels: {} } : null,
  );

  return {
    watchChildren(options) {
      for (const childAgentId of childAgentIds) {
        setupFinishNotification({
          agentManager,
          agentStorage,
          childAgentId,
          callerAgentId: "caller-agent",
          wake: options?.wake,
          logger: createTestLogger(),
        });
      }
    },
    finishChild(childAgentId) {
      const child = children.get(childAgentId)!;
      child.lifecycle = "running";
      emit(childAgentId, { type: "agent_state", agent: child });
      child.lifecycle = "idle";
      emit(childAgentId, { type: "agent_state", agent: child });
    },
    requestPermission(childAgentId) {
      const child = children.get(childAgentId)!;
      const request = {
        id: `${childAgentId}-permission`,
        provider: "claude" as const,
        kind: "tool" as const,
        name: "Run command",
      };
      child.lifecycle = "running";
      child.pendingPermissions.set(request.id, request);
      emit(childAgentId, {
        type: "agent_stream",
        agentId: childAgentId,
        event: { type: "permission_requested", provider: "claude", request },
      });
    },
    setCallerRunning(running) {
      callerRunning = running;
      caller.lifecycle = running ? "running" : "idle";
      emit("caller-agent", { type: "agent_state", agent: caller });
    },
    disarmCaller() {
      disarmFinishNotifications({ agentManager, callerAgentIds: new Set(["caller-agent"]) });
    },
    callerPrompts() {
      return callerPrompts;
    },
    subscriberCount(agentId) {
      return subscribers.get(agentId)?.size ?? 0;
    },
  };
}

async function withFakeTimers(run: () => Promise<void>): Promise<void> {
  vi.useFakeTimers();
  try {
    await run();
  } finally {
    vi.useRealTimers();
  }
}

test("children that finish within the window reach the caller as one notice", async () => {
  await withFakeTimers(async () => {
    const scenario = createMailboxScenario(["child-1", "child-2", "child-3"]);
    scenario.watchChildren();

    scenario.finishChild("child-1");
    await vi.advanceTimersByTimeAsync(500);
    scenario.finishChild("child-2");
    await vi.advanceTimersByTimeAsync(500);
    scenario.finishChild("child-3");
    await vi.advanceTimersByTimeAsync(FINISH_NOTICE_WINDOW_MS);

    expect(scenario.callerPrompts()).toHaveLength(1);
    const [prompt] = scenario.callerPrompts();
    expect(prompt).toContain("Agent child-1 (child-1) finished.");
    expect(prompt).toContain("Agent child-2 (child-2) finished.");
    expect(prompt).toContain("Agent child-3 (child-3) finished.");
  });
});

test("children that finish apart reach the caller as separate notices", async () => {
  await withFakeTimers(async () => {
    const scenario = createMailboxScenario(["child-1", "child-2"]);
    scenario.watchChildren();

    scenario.finishChild("child-1");
    await vi.advanceTimersByTimeAsync(FINISH_NOTICE_WINDOW_MS);
    await vi.advanceTimersByTimeAsync(5000 - FINISH_NOTICE_WINDOW_MS);
    scenario.finishChild("child-2");
    await vi.advanceTimersByTimeAsync(FINISH_NOTICE_WINDOW_MS);

    expect(scenario.callerPrompts()).toHaveLength(2);
    expect(scenario.callerPrompts()[0]).toContain("Agent child-1 (child-1) finished.");
    expect(scenario.callerPrompts()[1]).toContain("Agent child-2 (child-2) finished.");
  });
});

test("a permission notice skips the coalescing window", async () => {
  await withFakeTimers(async () => {
    const scenario = createMailboxScenario(["child-1"]);
    scenario.watchChildren();

    scenario.requestPermission("child-1");
    await vi.advanceTimersByTimeAsync(0);

    expect(scenario.callerPrompts()).toHaveLength(1);
    expect(scenario.callerPrompts()[0]).toContain("Agent child-1 (child-1) needs permission.");
  });
});

test("settled_only holds notices until the caller's run ends", async () => {
  await withFakeTimers(async () => {
    const scenario = createMailboxScenario(["child-1", "child-2"]);
    scenario.setCallerRunning(true);
    scenario.watchChildren({ wake: "settled_only" });

    scenario.finishChild("child-1");
    scenario.finishChild("child-2");
    await vi.advanceTimersByTimeAsync(FINISH_NOTICE_WINDOW_MS * 4);
    expect(scenario.callerPrompts()).toHaveLength(0);

    scenario.setCallerRunning(false);
    await vi.advanceTimersByTimeAsync(0);

    expect(scenario.callerPrompts()).toHaveLength(1);
    expect(scenario.callerPrompts()[0]).toContain("Agent child-1 (child-1) finished.");
    expect(scenario.callerPrompts()[0]).toContain("Agent child-2 (child-2) finished.");
  });
});

test("settled_only still sends a permission notice to a running caller at once", async () => {
  await withFakeTimers(async () => {
    const scenario = createMailboxScenario(["child-1"]);
    scenario.setCallerRunning(true);
    scenario.watchChildren({ wake: "settled_only" });

    scenario.requestPermission("child-1");
    await vi.advanceTimersByTimeAsync(0);

    expect(scenario.callerPrompts()).toHaveLength(1);
    expect(scenario.callerPrompts()[0]).toContain("Agent child-1 (child-1) needs permission.");
  });
});

test("arming a child again replaces its earlier watch", () => {
  const scenario = createMailboxScenario(["child-1"]);

  scenario.watchChildren();
  scenario.watchChildren();

  expect(scenario.subscriberCount("child-1")).toBe(1);
});

test("disarming the caller drops a held settled_only notice and its listener", async () => {
  await withFakeTimers(async () => {
    const scenario = createMailboxScenario(["child-1"]);
    scenario.setCallerRunning(true);
    scenario.watchChildren({ wake: "settled_only" });

    scenario.finishChild("child-1");
    await vi.advanceTimersByTimeAsync(FINISH_NOTICE_WINDOW_MS);
    expect(scenario.subscriberCount("caller-agent")).toBe(1);

    scenario.disarmCaller();
    scenario.setCallerRunning(false);
    await vi.advanceTimersByTimeAsync(FINISH_NOTICE_WINDOW_MS);

    expect(scenario.subscriberCount("caller-agent")).toBe(0);
    expect(scenario.callerPrompts()).toHaveLength(0);
  });
});

test("the same child and reason twice in one window gives one section", async () => {
  await withFakeTimers(async () => {
    const scenario = createMailboxScenario(["child-1"]);
    scenario.watchChildren();
    scenario.finishChild("child-1");
    await vi.advanceTimersByTimeAsync(0);
    scenario.watchChildren();
    scenario.finishChild("child-1");
    await vi.advanceTimersByTimeAsync(FINISH_NOTICE_WINDOW_MS);

    expect(scenario.callerPrompts()).toHaveLength(1);
    expect(
      scenario.callerPrompts()[0]?.match(/Agent child-1 \(child-1\) finished\./g),
    ).toHaveLength(1);
  });
});

test("disarming the caller drops notices still waiting in the window", async () => {
  await withFakeTimers(async () => {
    const scenario = createMailboxScenario(["child-1"]);
    scenario.watchChildren();

    scenario.finishChild("child-1");
    await vi.advanceTimersByTimeAsync(0);
    scenario.disarmCaller();
    await vi.advanceTimersByTimeAsync(FINISH_NOTICE_WINDOW_MS * 2);

    expect(scenario.callerPrompts()).toHaveLength(0);
  });
});
