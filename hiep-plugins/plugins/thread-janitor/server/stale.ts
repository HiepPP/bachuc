import type { PaseoAgent, PaseoWorkspace } from "@getpaseo/client";
import type { JanitorSettings } from "../shared/settings";

const HOUR_MS = 60 * 60 * 1000;
const PARENT_LABEL = "paseo.parent-agent-id";

export type JanitorAgent = Pick<
  PaseoAgent,
  | "id"
  | "title"
  | "status"
  | "updatedAt"
  | "lastUserMessageAt"
  | "pendingPermissions"
  | "archivedAt"
  | "labels"
  | "workspaceId"
  | "cwd"
>;

// The agent snapshot has no `lastActivityAt`; the daemon stores lastActivityAt as the agent's
// `updatedAt`. An unparseable timestamp yields NaN, which never counts as stale.
export function lastActivityAt(agent: JanitorAgent): number {
  const updated = Date.parse(agent.updatedAt);
  const lastMessage = agent.lastUserMessageAt ? Date.parse(agent.lastUserMessageAt) : -Infinity;
  return Math.max(updated, lastMessage);
}

function isActive(agent: JanitorAgent): boolean {
  return agent.status === "running" || agent.status === "initializing";
}

/** Ids of every agent with an active descendant, following the parent label upward. */
function ancestorsOfActive(agents: readonly JanitorAgent[]): Set<string> {
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
 * Threads whose runtime can be closed. `idle` and `error` are the only statuses that hold a
 * runtime without a turn; `closed` has none and `running`/`initializing` are in use.
 */
export function selectStale<T extends JanitorAgent>(
  agents: readonly T[],
  now: Date,
  settings: JanitorSettings,
): T[] {
  if (!settings.enabled) return [];
  const busy = ancestorsOfActive(agents);
  const cutoff = now.getTime() - settings.idleHours * HOUR_MS;
  return agents.filter(
    (agent) =>
      !agent.archivedAt &&
      (agent.status === "idle" || agent.status === "error") &&
      agent.pendingPermissions.length === 0 &&
      !busy.has(agent.id) &&
      lastActivityAt(agent) < cutoff,
  );
}

export type JanitorWorkspace = Pick<
  PaseoWorkspace,
  | "id"
  | "workspaceKind"
  | "workspaceDirectory"
  | "projectRootPath"
  | "pinnedAt"
  | "archivingAt"
  | "status"
  | "activityAt"
  | "statusEnteredAt"
>;

/** Workspaces still holding an unarchived agent, by id or by directory when the id is missing. */
export interface LiveWorkspaces {
  ids: ReadonlySet<string>;
  directories: ReadonlySet<string>;
}

export function liveWorkspaces(
  agents: readonly Pick<PaseoAgent, "workspaceId" | "cwd">[],
): LiveWorkspaces {
  const ids = new Set<string>();
  const directories = new Set<string>();
  for (const agent of agents) {
    if (agent.workspaceId) ids.add(agent.workspaceId);
    else directories.add(agent.cwd);
  }
  return { ids, directories };
}

// The sidebar lists workspaces, so archived agents alone leave their rows behind.
// Paseo-owned worktrees are never selected: archiving one can remove its directory.
export function selectStaleWorkspaces<T extends JanitorWorkspace>(
  workspaces: readonly T[],
  live: LiveWorkspaces,
  now: Date,
  settings: JanitorSettings,
): T[] {
  if (!settings.enabled) return [];
  const cutoff = now.getTime() - settings.idleHours * HOUR_MS;
  return workspaces.filter((workspace) => {
    const activity = Date.parse(workspace.activityAt ?? workspace.statusEnteredAt ?? "");
    return (
      workspace.workspaceKind !== "worktree" &&
      !workspace.pinnedAt &&
      !workspace.archivingAt &&
      workspace.status !== "running" &&
      workspace.status !== "needs_input" &&
      !live.ids.has(workspace.id) &&
      !live.directories.has(workspace.workspaceDirectory ?? workspace.projectRootPath) &&
      activity < cutoff
    );
  });
}
