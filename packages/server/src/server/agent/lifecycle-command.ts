import type { Logger } from "pino";
import { getParentAgentIdFromLabels } from "@getpaseo/protocol/agent-labels";

import {
  AgentRunCancellationError,
  type AgentRunCancellationResult,
  type ManagedAgent,
} from "./agent-manager.js";
import { holdAgentMessageQueues } from "./agent-message-queue.js";
import { disarmFinishNotifications } from "./agent-prompt.js";
import type { StoredAgentRecord } from "./agent-storage.js";
import type { AgentProviderNotice } from "./agent-sdk-types.js";

export type LifecycleAgentSnapshot = Pick<ManagedAgent, "id" | "cwd" | "lifecycle">;
export type LifecycleAgentLink = Pick<ManagedAgent, "id" | "labels">;

export interface LifecycleAgentManager {
  getAgent(agentId: string): LifecycleAgentSnapshot | null;
  listAgents(): LifecycleAgentLink[];
  hasInFlightRun(agentId: string): boolean;
  cancelAgentRun(agentId: string): Promise<AgentRunCancellationResult>;
  clearAgentAttention(agentId: string): Promise<void>;
  archiveAgent(agentId: string): Promise<{ archivedAt: string }>;
  archiveSnapshot(agentId: string, archivedAt: string): Promise<StoredAgentRecord>;
  closeAgent(agentId: string): Promise<void>;
  setLabels(agentId: string, labels: Record<string, string>): Promise<void>;
  detachAgent(agentId: string): Promise<{
    record: StoredAgentRecord;
    live: boolean;
    previousParentAgentId: string | null;
  }>;
  notifyAgentState(agentId: string): void;
  setAgentMode(agentId: string, modeId: string): Promise<AgentProviderNotice | null>;
  updateAgentMetadata(
    agentId: string,
    updates: {
      title?: string;
      labels?: Record<string, string>;
    },
  ): Promise<void>;
}

export interface LifecycleAgentStorage {
  get(agentId: string): Promise<StoredAgentRecord | null>;
  upsert(record: StoredAgentRecord): Promise<void>;
}

export interface AgentLifecycleCommandDependencies {
  agentManager: LifecycleAgentManager;
  agentStorage: LifecycleAgentStorage;
  logger: Logger;
}

export interface CancelAgentRunResult {
  agent: LifecycleAgentSnapshot;
  cancelled: boolean;
}

interface RequestedAgentRunCancellation extends CancelAgentRunResult {
  cancellation: AgentRunCancellationResult;
}

async function requestAgentRunCancellation(
  dependencies: Pick<AgentLifecycleCommandDependencies, "agentManager" | "logger">,
  agentId: string,
  // A caller that already looked the agent up passes it, so Stop reads it once.
  knownAgent?: LifecycleAgentSnapshot | null,
): Promise<RequestedAgentRunCancellation> {
  const { agentManager, logger } = dependencies;
  const agent = knownAgent === undefined ? agentManager.getAgent(agentId) : knownAgent;
  if (!agent) {
    logger.trace({ agentId }, "cancelAgentRunCommand: agent not found");
    throw new Error(`Agent ${agentId} not found`);
  }

  const hasInFlightRun = agentManager.hasInFlightRun(agentId);
  if (!hasInFlightRun) {
    logger.trace(
      { agentId, lifecycle: agent.lifecycle, hasInFlightRun },
      "cancelAgentRunCommand: skipping because agent is not running",
    );
    return { agent, cancelled: false, cancellation: { status: "not_running" } };
  }

  logger.debug(
    { agentId, lifecycle: agent.lifecycle, hasInFlightRun },
    "cancelAgentRunCommand: interrupting",
  );
  const startedAt = Date.now();
  const cancellation = await agentManager.cancelAgentRun(agentId);
  logger.debug(
    { agentId, cancellation: cancellation.status, durationMs: Date.now() - startedAt },
    "cancelAgentRunCommand: cancelAgentRun completed",
  );

  return {
    agent,
    cancelled: cancellation.status === "settled",
    cancellation,
  };
}

// Live managed descendants found through the parent-agent-id label, deepest first.
// Provider subagents are not managed agents, and the parent's own interrupt stops them.
function listLiveDescendantsDeepestFirst(
  agentManager: Pick<LifecycleAgentManager, "listAgents">,
  rootAgentId: string,
): string[] {
  const childrenByParent = new Map<string, string[]>();
  for (const agent of agentManager.listAgents()) {
    const parentAgentId = getParentAgentIdFromLabels(agent.labels);
    if (!parentAgentId) {
      continue;
    }
    const siblings = childrenByParent.get(parentAgentId) ?? [];
    siblings.push(agent.id);
    childrenByParent.set(parentAgentId, siblings);
  }

  const levels: string[][] = [];
  const visited = new Set<string>([rootAgentId]);
  let frontier = [rootAgentId];
  while (frontier.length > 0) {
    const next: string[] = [];
    for (const parentAgentId of frontier) {
      for (const childAgentId of childrenByParent.get(parentAgentId) ?? []) {
        if (visited.has(childAgentId)) {
          continue;
        }
        visited.add(childAgentId);
        next.push(childAgentId);
      }
    }
    if (next.length > 0) {
      levels.push(next);
    }
    frontier = next;
  }
  return levels.toReversed().flat();
}

function disarmSubtreeNotices(
  agentManager: Pick<LifecycleAgentManager, "listAgents">,
  rootAgentId: string,
): string[] {
  const descendantIds = listLiveDescendantsDeepestFirst(agentManager, rootAgentId);
  disarmFinishNotifications({
    agentManager,
    callerAgentIds: new Set([rootAgentId, ...descendantIds]),
  });
  return descendantIds;
}

// Stop applies to the whole managed subtree. Notices are disarmed first, so a child
// that settles during the cascade cannot start a new turn on its stopped parent.
async function cancelDescendantRuns(
  dependencies: Pick<AgentLifecycleCommandDependencies, "agentManager" | "logger">,
  rootAgentId: string,
): Promise<void> {
  const { agentManager, logger } = dependencies;
  const descendantIds = disarmSubtreeNotices(agentManager, rootAgentId);
  // Hold before any cancel: the idle state a cancel produces would otherwise start the
  // next queued message on an agent the user just stopped.
  await holdAgentMessageQueues({ agentManager, agentIds: [rootAgentId, ...descendantIds] });

  for (const descendantId of descendantIds) {
    if (!agentManager.hasInFlightRun(descendantId)) {
      continue;
    }
    try {
      const cancellation = await agentManager.cancelAgentRun(descendantId);
      if (cancellation.status === "refused") {
        logger.warn(
          { agentId: descendantId, rootAgentId },
          "cancelAgentRunCommand: subagent run was not cancelled",
        );
      }
    } catch (error) {
      logger.warn(
        { err: error, agentId: descendantId, rootAgentId },
        "cancelAgentRunCommand: failed to cancel subagent run",
      );
    }
  }
}

export async function cancelAgentRunCommand(
  dependencies: Pick<AgentLifecycleCommandDependencies, "agentManager" | "logger">,
  agentId: string,
): Promise<CancelAgentRunResult> {
  const agent = dependencies.agentManager.getAgent(agentId);
  try {
    if (agent) {
      await cancelDescendantRuns(dependencies, agentId);
    }
    const result = await requestAgentRunCancellation(dependencies, agentId, agent);
    if (result.cancellation.status === "refused") {
      dependencies.logger.warn(
        { agentId },
        "cancelAgentRunCommand: reported running but no active run was cancelled",
      );
      throw new AgentRunCancellationError(agentId, "stop");
    }

    return { agent: result.agent, cancelled: result.cancelled };
  } finally {
    // The parent keeps running until its own cancel settles, so it can arm new notices
    // or spawn children during the cascade. Disarm the subtree again once it is stopped.
    if (agent) {
      disarmSubtreeNotices(dependencies.agentManager, agentId);
    }
  }
}

export interface ArchiveAgentResult {
  agentId: string;
  archivedAt: string;
  record: StoredAgentRecord;
}

export async function archiveAgentCommand(
  dependencies: AgentLifecycleCommandDependencies,
  agentId: string,
): Promise<ArchiveAgentResult> {
  const liveAgent = dependencies.agentManager.getAgent(agentId);
  let record: StoredAgentRecord | null;
  if (liveAgent) {
    // The cancel's idle state must not start a queued message on an agent being archived.
    await holdAgentMessageQueues({ agentManager: dependencies.agentManager, agentIds: [agentId] });
    await requestAgentRunCancellation(dependencies, agentId);
    await dependencies.agentManager.clearAgentAttention(agentId).catch(() => undefined);
    await dependencies.agentManager.archiveAgent(agentId);
    record = await dependencies.agentStorage.get(agentId);
  } else {
    record = await archiveStoredAgent(dependencies, agentId);
  }

  if (!record) {
    throw new Error(`Agent not found in storage after archive: ${agentId}`);
  }
  if (!record.archivedAt) {
    throw new Error(`Agent missing archivedAt after archive: ${agentId}`);
  }

  return {
    agentId,
    archivedAt: record.archivedAt,
    record,
  };
}

export async function closeAgentCommand(
  dependencies: Pick<AgentLifecycleCommandDependencies, "agentManager">,
  agentId: string,
): Promise<void> {
  await dependencies.agentManager.closeAgent(agentId);
}

export interface UpdateAgentResult {
  accepted: boolean;
  error: string | null;
}

export async function updateAgentCommand(
  dependencies: Pick<AgentLifecycleCommandDependencies, "agentManager">,
  input: {
    agentId: string;
    name?: string;
    labels?: Record<string, string>;
  },
): Promise<UpdateAgentResult> {
  const title = input.name?.trim();
  const labels = input.labels && Object.keys(input.labels).length > 0 ? input.labels : undefined;

  if (!title && !labels) {
    return {
      accepted: false,
      error: "Nothing to update (provide name and/or labels)",
    };
  }

  await dependencies.agentManager.updateAgentMetadata(input.agentId, {
    ...(title ? { title } : {}),
    ...(labels ? { labels } : {}),
  });

  return {
    accepted: true,
    error: null,
  };
}

export interface DetachAgentResult {
  agentId: string;
  record: StoredAgentRecord;
  live: boolean;
  previousParentAgentId: string | null;
}

export async function detachAgentCommand(
  dependencies: Pick<AgentLifecycleCommandDependencies, "agentManager">,
  agentId: string,
): Promise<DetachAgentResult> {
  const result = await dependencies.agentManager.detachAgent(agentId);
  return {
    agentId,
    ...result,
  };
}

export async function setAgentModeCommand(
  dependencies: Pick<AgentLifecycleCommandDependencies, "agentManager">,
  input: {
    agentId: string;
    modeId: string;
  },
): Promise<{ modeId: string; notice: AgentProviderNotice | null }> {
  const notice = await dependencies.agentManager.setAgentMode(input.agentId, input.modeId);
  return { modeId: input.modeId, notice };
}

async function archiveStoredAgent(
  dependencies: Pick<AgentLifecycleCommandDependencies, "agentManager" | "agentStorage">,
  agentId: string,
): Promise<StoredAgentRecord> {
  const existing = await dependencies.agentStorage.get(agentId);
  if (!existing) {
    throw new Error(`Agent not found: ${agentId}`);
  }

  if (existing.archivedAt) {
    return existing;
  }

  const archivedAt = new Date().toISOString();
  return dependencies.agentManager.archiveSnapshot(agentId, archivedAt);
}
