import type { DaemonClient } from "@getpaseo/client/internal/daemon-client";
import { hostSupportsFeature } from "@/runtime/host-features";
import { getHostRuntimeStore } from "@/runtime/host-runtime";
import { useSessionStore } from "@/stores/session-store";
import { planTimelineTailFetch } from "./timeline-sync-plan";

export const AGENT_PREFETCH_TTL_MS = 30_000;
export const AGENT_PREFETCH_QUEUE_LIMIT = 3;

export type AgentPrefetchAnchor = "bottom" | "latest-prompt";

export interface AgentPrefetchInput {
  serverId: string;
  agentId: string;
  anchor?: AgentPrefetchAnchor;
}

type TimelineRequest = NonNullable<Parameters<DaemonClient["fetchAgentTimeline"]>[1]>;

export interface AgentPrefetchPorts {
  /** True when the viewed-timeline sync already keeps this agent current. */
  isAgentDesired(serverId: string, agentId: string): boolean;
  supportsLatestPromptTail(serverId: string): boolean;
  prepareAgentRoute(serverId: string, agentId: string): Promise<void>;
  prepareAgentTimeline(serverId: string, agentId: string): Promise<void>;
  fetchAgentTimeline(serverId: string, agentId: string, request: TimelineRequest): Promise<unknown>;
  now(): number;
  reportError(error: unknown): void;
}

interface HostQueue {
  running: string | null;
  queued: Map<string, AgentPrefetchInput>;
}

export function createAgentPrefetcher(ports: AgentPrefetchPorts) {
  const hosts = new Map<string, HostQueue>();
  const completedAt = new Map<string, number>();

  const isFresh = (key: string) => {
    const at = completedAt.get(key);
    return at !== undefined && ports.now() - at < AGENT_PREFETCH_TTL_MS;
  };

  const run = async (input: AgentPrefetchInput) => {
    const { serverId, agentId } = input;
    await ports.prepareAgentRoute(serverId, agentId);
    await ports.prepareAgentTimeline(serverId, agentId);
    await ports.fetchAgentTimeline(serverId, agentId, {
      ...planTimelineTailFetch(),
      ...(input.anchor === "latest-prompt" && ports.supportsLatestPromptTail(serverId)
        ? { includeLatestPrompt: true }
        : {}),
    });
    completedAt.set(`${serverId}:${agentId}`, ports.now());
  };

  const drain = (serverId: string, host: HostQueue) => {
    if (host.running) return;
    const next = host.queued.entries().next();
    if (next.done) {
      hosts.delete(serverId);
      return;
    }
    const [key, input] = next.value;
    host.queued.delete(key);
    // The user may have opened the agent while it waited in the queue.
    if (ports.isAgentDesired(serverId, input.agentId) || isFresh(key)) {
      drain(serverId, host);
      return;
    }
    host.running = key;
    void run(input)
      .catch((error) => ports.reportError(error))
      .finally(() => {
        host.running = null;
        drain(serverId, host);
      });
  };

  return {
    prefetchAgent(input: AgentPrefetchInput): void {
      const key = `${input.serverId}:${input.agentId}`;
      if (ports.isAgentDesired(input.serverId, input.agentId) || isFresh(key)) return;
      const host = hosts.get(input.serverId) ?? { running: null, queued: new Map() };
      hosts.set(input.serverId, host);
      if (host.running === key) return;
      const queued = host.queued.get(key);
      host.queued.delete(key);
      host.queued.set(key, {
        ...input,
        anchor: queued?.anchor === "latest-prompt" ? "latest-prompt" : input.anchor,
      });
      // Hover moves fast: only the newest cards are worth warming.
      for (const oldest of host.queued.keys()) {
        if (host.queued.size <= AGENT_PREFETCH_QUEUE_LIMIT) break;
        host.queued.delete(oldest);
      }
      drain(input.serverId, host);
    },
  };
}

/** Reads the host session's viewed-timeline sync; a host without one holds no agent. */
export function isViewedAgentDesired(serverId: string, agentId: string): boolean {
  const sync = useSessionStore.getState().sessions[serverId]?.viewedTimelineSync;
  return sync?.isAgentDesired(agentId) ?? false;
}

const hostPrefetcher = createAgentPrefetcher({
  isAgentDesired: isViewedAgentDesired,
  supportsLatestPromptTail: (serverId) =>
    hostSupportsFeature(
      useSessionStore.getState().sessions[serverId]?.serverInfo,
      "timelineLatestPromptTail",
    ),
  prepareAgentRoute: (serverId, agentId) =>
    getHostRuntimeStore().prepareAgentRoute(serverId, agentId),
  prepareAgentTimeline: (serverId, agentId) =>
    getHostRuntimeStore().prepareAgentTimeline(serverId, agentId),
  fetchAgentTimeline: (serverId, agentId, request) =>
    getHostRuntimeStore().fetchAgentTimeline(serverId, agentId, request),
  now: () => Date.now(),
  reportError: (error) => console.debug("[AgentPrefetch] prefetch failed", error),
});

/** Warms an agent's directory row, replica cache, and tail page. Never shows UI. */
export function prefetchAgent(input: AgentPrefetchInput): void {
  hostPrefetcher.prefetchAgent(input);
}
