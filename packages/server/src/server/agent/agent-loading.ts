import type { Logger } from "pino";

import type { AgentProvider } from "./agent-sdk-types.js";
import type { AgentManager, ManagedAgent } from "./agent-manager.js";
import type { AgentStorage, StoredAgentRecord } from "./agent-storage.js";
import {
  buildConfigOverrides,
  buildSessionConfig,
  extractAttention,
  extractTimestamps,
  isStoredAgentProviderAvailable,
  toAgentPersistenceHandle,
} from "../persistence-hooks.js";

interface PendingAgentInitialization {
  promise: Promise<ManagedAgent>;
  options: { broadcastTimeline: boolean };
}

const pendingAgentInitializations = new Map<string, PendingAgentInitialization>();

export type AgentLoaderManager = Pick<
  AgentManager,
  | "createAgent"
  | "getAgent"
  | "getRegisteredProviderIds"
  | "hydrateTimelineFromProvider"
  | "resumeAgentFromPersistence"
> &
  Partial<Pick<AgentManager, "waitForAgentClose">>;

export interface EnsureAgentLoadedDeps {
  agentManager: AgentLoaderManager;
  agentStorage: AgentStorage;
  validProviders?: Iterable<AgentProvider>;
  broadcastTimeline?: boolean;
  logger: Logger;
}

export async function ensureUnarchivedAgentLoaded(
  agentId: string,
  deps: EnsureAgentLoadedDeps & {
    agentManager: AgentLoaderManager & Pick<AgentManager, "closeAgent">;
  },
): Promise<ManagedAgent> {
  const record = await deps.agentStorage.get(agentId);
  if (record?.archivedAt) {
    throw new Error(`Agent is archived: ${agentId}`);
  }

  const agent = await ensureAgentLoaded(agentId, deps);
  const latestRecord = await deps.agentStorage.get(agentId);
  if (latestRecord?.archivedAt) {
    await deps.agentManager.closeAgent(agentId).catch((error: unknown) => {
      deps.logger.warn({ err: error, agentId }, "Failed to close concurrently archived agent");
    });
    throw new Error(`Agent is archived: ${agentId}`);
  }

  return agent;
}

export type ReadableAgent =
  | { kind: "live"; agent: ManagedAgent }
  | { kind: "stored"; record: StoredAgentRecord };

export type TimelineReaderManager = AgentLoaderManager &
  Pick<AgentManager, "loadTimelineForRead" | "backfillTimelineFromProvider">;

export interface EnsureAgentTimelineReadableDeps extends EnsureAgentLoadedDeps {
  agentManager: TimelineReaderManager;
}

const pendingTimelineBackfills = new Map<string, Promise<void>>();

/**
 * Makes an agent's timeline readable without resuming it when a stored copy exists: the live
 * agent, else the timeline cache, else a provider-history backfill. Falls back to
 * ensureAgentLoaded only when the backfill cannot run.
 */
export async function ensureAgentTimelineReadable(
  agentId: string,
  deps: EnsureAgentTimelineReadableDeps,
): Promise<ReadableAgent> {
  const live = deps.agentManager.getAgent(agentId);
  if (live) return { kind: "live", agent: live };
  const inflight = pendingAgentInitializations.get(agentId);
  if (inflight) return { kind: "live", agent: await inflight.promise };

  const record = await deps.agentStorage.get(agentId);
  if (!record) {
    throw new Error(`Agent not found: ${agentId}`);
  }

  const source = await deps.agentManager.loadTimelineForRead(agentId);
  if (source === "missing") {
    const validProviders = deps.validProviders ?? deps.agentManager.getRegisteredProviderIds();
    if (!toAgentPersistenceHandle(validProviders, record.persistence)) {
      // Never started, or its provider is gone: there is no history to read, so load as before.
      return { kind: "live", agent: await ensureAgentLoaded(agentId, deps) };
    }
    try {
      await backfillTimelineOnce(agentId, record, deps.agentManager);
    } catch (error) {
      deps.logger.warn({ err: error, agentId }, "Timeline backfill failed; resuming the agent");
      return { kind: "live", agent: await ensureAgentLoaded(agentId, deps) };
    }
  }

  // A resume can register the agent while the read loads; its timeline is the same one.
  const resumed = deps.agentManager.getAgent(agentId);
  return resumed ? { kind: "live", agent: resumed } : { kind: "stored", record };
}

function backfillTimelineOnce(
  agentId: string,
  record: StoredAgentRecord,
  agentManager: TimelineReaderManager,
): Promise<void> {
  const inflight = pendingTimelineBackfills.get(agentId);
  if (inflight) return inflight;
  const backfill = agentManager
    .backfillTimelineFromProvider(agentId, record)
    .finally(() => pendingTimelineBackfills.delete(agentId));
  pendingTimelineBackfills.set(agentId, backfill);
  return backfill;
}

export async function ensureAgentLoaded(
  agentId: string,
  deps: EnsureAgentLoadedDeps,
): Promise<ManagedAgent> {
  await deps.agentManager.waitForAgentClose?.(agentId);

  const inflight = pendingAgentInitializations.get(agentId);
  if (inflight) {
    inflight.options.broadcastTimeline ||= deps.broadcastTimeline === true;
    return inflight.promise;
  }

  const existing = deps.agentManager.getAgent(agentId);
  if (existing) {
    return existing;
  }

  // A close may have started after the first barrier observed no in-flight
  // work. Once the live lookup is empty, this second barrier closes that gap
  // before storage-backed resume begins.
  await deps.agentManager.waitForAgentClose?.(agentId);

  const laterInflight = pendingAgentInitializations.get(agentId);
  if (laterInflight) {
    laterInflight.options.broadcastTimeline ||= deps.broadcastTimeline === true;
    return laterInflight.promise;
  }

  const pendingOptions = {
    broadcastTimeline: deps.broadcastTimeline === true,
  };
  const initPromise = (async () => {
    const record = await deps.agentStorage.get(agentId);
    if (!record) {
      throw new Error(`Agent not found: ${agentId}`);
    }

    const validProviders = deps.validProviders ?? deps.agentManager.getRegisteredProviderIds();
    if (!isStoredAgentProviderAvailable(record, validProviders)) {
      throw new Error(`Agent ${agentId} references unavailable provider '${record.provider}'`);
    }

    const handle = toAgentPersistenceHandle(validProviders, record.persistence);

    let snapshot: ManagedAgent;
    if (handle) {
      snapshot = await deps.agentManager.resumeAgentFromPersistence(
        handle,
        buildConfigOverrides(record),
        agentId,
        { ...extractTimestamps(record), attention: extractAttention(record) },
        record.archivedAt ? { purpose: "history" } : undefined,
      );
      deps.logger.info({ agentId, provider: record.provider }, "Agent resumed from persistence");
    } else {
      // No provider handle to resume: this starts the agent's first session rather than
      // bringing one back, so it stamps activity and carries no stored attention. Records
      // without a handle never got far enough to accumulate either.
      const config = buildSessionConfig(record, {
        validProviders,
      });
      if (!config) {
        throw new Error(`Agent ${agentId} references unavailable provider '${record.provider}'`);
      }
      snapshot = await deps.agentManager.createAgent(config, agentId, {
        labels: record.labels,
        workspaceId: record.workspaceId,
        owner: record.owner,
      });
      deps.logger.info({ agentId, provider: record.provider }, "Agent created from stored config");
    }

    await deps.agentManager.hydrateTimelineFromProvider(agentId, {
      broadcast: () => pendingOptions.broadcastTimeline,
    });
    return deps.agentManager.getAgent(agentId) ?? snapshot;
  })();

  const pending: PendingAgentInitialization = { promise: initPromise, options: pendingOptions };
  pendingAgentInitializations.set(agentId, pending);

  try {
    return await initPromise;
  } finally {
    const current = pendingAgentInitializations.get(agentId);
    if (current === pending) {
      pendingAgentInitializations.delete(agentId);
    }
  }
}
