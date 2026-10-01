import type { PaseoAgent } from "@getpaseo/client";
import type { CloserSettings } from "../shared/settings";

const MINUTE_MS = 60 * 1000;
const PARENT_LABEL = "paseo.parent-agent-id";

export type CloserAgent = Pick<
  PaseoAgent,
  | "id"
  | "status"
  | "updatedAt"
  | "lastUserMessageAt"
  | "pendingPermissions"
  | "archivedAt"
  | "labels"
>;

// The daemon stores last activity as `updatedAt`. An unparseable timestamp yields NaN,
// which never counts as idle.
export function lastActivityAt(agent: CloserAgent): number {
  const updated = Date.parse(agent.updatedAt);
  const lastMessage = agent.lastUserMessageAt ? Date.parse(agent.lastUserMessageAt) : -Infinity;
  return Math.max(updated, lastMessage);
}

function isActive(agent: CloserAgent): boolean {
  return agent.status === "running" || agent.status === "initializing";
}

/** Ids of every agent with an active descendant, following the parent label upward. */
function ancestorsOfActive(agents: readonly CloserAgent[]): Set<string> {
  const byId = new Map(agents.map((agent) => [agent.id, agent]));
  const busy = new Set<string>();
  for (const agent of agents) {
    if (!isActive(agent)) continue;
    let parentId: string | undefined = agent.labels?.[PARENT_LABEL];
    while (parentId && !busy.has(parentId)) {
      busy.add(parentId);
      parentId = byId.get(parentId)?.labels?.[PARENT_LABEL];
    }
  }
  return busy;
}

/**
 * Agents whose runtime can be released. `idle` and `error` are the only statuses that hold a
 * runtime without a turn; `closed` has none and `running`/`initializing` are in use.
 */
export function selectIdle<T extends CloserAgent>(
  agents: readonly T[],
  now: Date,
  settings: CloserSettings,
  openedAt: ReadonlyMap<string, number> = new Map(),
): T[] {
  if (!settings.enabled) return [];
  const busy = ancestorsOfActive(agents);
  const cutoff = now.getTime() - settings.idleMinutes * MINUTE_MS;
  return agents.filter(
    (agent) =>
      !agent.archivedAt &&
      (agent.status === "idle" || agent.status === "error") &&
      agent.pendingPermissions.length === 0 &&
      !busy.has(agent.id) &&
      // Resuming a thread keeps its stored activity time, so a thread opened to be read
      // would otherwise close on the next sweep.
      Math.max(lastActivityAt(agent), openedAt.get(agent.id) ?? -Infinity) < cutoff,
  );
}
