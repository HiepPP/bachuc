import type { PaseoAgent, PaseoWorkspace } from "@getpaseo/client";
import type { JanitorSettings } from "../shared/settings";

const HOUR_MS = 60 * 60 * 1000;

export type JanitorAgent = Pick<
  PaseoAgent,
  | "id"
  | "title"
  | "status"
  | "updatedAt"
  | "lastUserMessageAt"
  | "pendingPermissions"
  | "archivedAt"
>;

// The agent snapshot has no `lastActivityAt`; the daemon stores lastActivityAt as the agent's
// `updatedAt`. An unparseable timestamp yields NaN, which never counts as stale.
export function lastActivityAt(agent: JanitorAgent): number {
  const updated = Date.parse(agent.updatedAt);
  const lastMessage = agent.lastUserMessageAt ? Date.parse(agent.lastUserMessageAt) : -Infinity;
  return Math.max(updated, lastMessage);
}

export function selectStale<T extends JanitorAgent>(
  agents: readonly T[],
  now: Date,
  settings: JanitorSettings,
): T[] {
  if (!settings.enabled) return [];
  const retained = new Set(
    agents
      .filter((agent) => !agent.archivedAt)
      .sort((a, b) => {
        // Unknown activity is protected rather than treated as old.
        const timeA = Number.isFinite(lastActivityAt(a)) ? lastActivityAt(a) : Infinity;
        const timeB = Number.isFinite(lastActivityAt(b)) ? lastActivityAt(b) : Infinity;
        return timeB - timeA || a.id.localeCompare(b.id);
      })
      .slice(0, settings.keepRecent)
      .map((agent) => agent.id),
  );
  const cutoff = now.getTime() - settings.idleHours * HOUR_MS;
  return agents.filter(
    (agent) =>
      !agent.archivedAt &&
      !retained.has(agent.id) &&
      agent.status !== "running" &&
      agent.pendingPermissions.length === 0 &&
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
