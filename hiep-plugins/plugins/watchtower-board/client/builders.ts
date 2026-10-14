import type { PaseoAgent } from "@getpaseo/client";
import type { Overview } from "../shared/overview";

export const TASK_LABEL = "watchtower.task";
const PARENT_LABEL = "paseo.parent-agent-id";
const TASK_ID = /^TASK-\d+$/;

// The snapshot carries the parent only as a label. The direct fields are read first, in case a
// host adds them.
export type BuilderCandidate = Pick<
  PaseoAgent,
  "id" | "title" | "status" | "workspaceId" | "archivedAt" | "updatedAt" | "labels" | "provider"
> &
  Partial<Pick<PaseoAgent, "requiresAttention" | "attentionReason">> & {
    parentAgentId?: string | null;
    lastActivityAt?: string | null;
  };

export type BuilderTask = Pick<Overview["tasks"][number], "id" | "group">;

export type BuilderState = "running" | "waiting" | "failed" | "idle";

export interface RunAgent {
  id: string;
  title: string | null;
  state: BuilderState;
  // Milliseconds since the last activity; null when the time is unreadable.
  activeAgo: number | null;
}

export interface BuilderMatch {
  // Keyed by TASK ID. A Group builder appears under every TASK of its group.
  byTask: Record<string, RunAgent & { title: string }>;
  // The parent that most matched builders name, or null (Q-036 default).
  lead: RunAgent | null;
}

function stateOf(agent: BuilderCandidate): BuilderState {
  // A permission wait comes first: the agent still reports `running` while it waits.
  if (agent.requiresAttention && agent.attentionReason === "permission") return "waiting";
  if (agent.status === "error") return "failed";
  if (agent.status === "running") return "running";
  return "idle";
}

function activeAgo(agent: BuilderCandidate, now: number): number | null {
  const at = Date.parse(agent.lastActivityAt ?? agent.updatedAt);
  return Number.isNaN(at) ? null : Math.max(0, now - at);
}

function parentOf(agent: BuilderCandidate): string | null {
  const parent = agent.parentAgentId?.trim() || agent.labels?.[PARENT_LABEL]?.trim();
  return parent || null;
}

function tasksOf(label: string, tasks: readonly BuilderTask[]): string[] {
  if (TASK_ID.test(label)) {
    const task = tasks.find((candidate) => candidate.id === label);
    if (!task) return [];
    // A builder labelled with the first TASK ID of its group works on the whole group.
    return task.group
      ? tasks.filter((candidate) => candidate.group === task.group).map((candidate) => candidate.id)
      : [task.id];
  }
  return tasks.filter((task) => task.group === label).map((task) => task.id);
}

export function matchBuilders(
  agents: readonly BuilderCandidate[],
  tasks: readonly BuilderTask[],
  now: number = Date.now(),
): BuilderMatch {
  const byTask: BuilderMatch["byTask"] = {};
  const winners = new Map<string, BuilderCandidate>();
  const updated = (agent: BuilderCandidate) => Date.parse(agent.updatedAt) || 0;

  // Newest first, so the first agent to claim a TASK is the one to keep.
  const sorted = agents
    .filter(
      (agent) =>
        !agent.archivedAt && !agent.provider?.startsWith("mock") && agent.labels?.[TASK_LABEL],
    )
    .sort((a, b) => updated(b) - updated(a));
  for (const agent of sorted) {
    const label = (agent.labels?.[TASK_LABEL] ?? "").trim();
    if (!label) continue;
    for (const taskId of tasksOf(label, tasks)) {
      if (byTask[taskId]) continue;
      byTask[taskId] = {
        id: agent.id,
        title: agent.title?.trim() || agent.id,
        state: stateOf(agent),
        activeAgo: activeAgo(agent, now),
      };
      winners.set(agent.id, agent);
    }
  }

  // Each builder agent votes once, however many TASKs it covers.
  const votes = new Map<string, number>();
  for (const agent of winners.values()) {
    const parent = parentOf(agent);
    if (parent) votes.set(parent, (votes.get(parent) ?? 0) + 1);
  }
  let leadId: string | null = null;
  for (const [parent, count] of votes) {
    if (leadId === null || count > (votes.get(leadId) ?? 0)) leadId = parent;
  }
  const found = leadId ? agents.find((agent) => agent.id === leadId) : undefined;
  const lead: RunAgent | null = leadId
    ? found
      ? {
          id: found.id,
          title: found.title?.trim() || null,
          state: stateOf(found),
          activeAgo: activeAgo(found, now),
        }
      : { id: leadId, title: null, state: "idle", activeAgo: null }
    : null;
  return { byTask, lead };
}
