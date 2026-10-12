import type { PaseoApi } from "@getpaseo/client";
import { EDIT_DIFFS_MODE_LABEL, WATCHTOWER_TASK_LABEL } from "../shared/board";
import { getParentAgentIdFromLabels } from "./agent-labels";
import type { ActiveAgent } from "./store";

export async function listBoardAgents(paseo: Pick<PaseoApi, "agents">, signal: AbortSignal) {
  const agents: ActiveAgent[] = [];
  const visibleAgentIds = new Set<string>();
  // The host flags a thread when a turn finishes or fails and clears it when a client opens it.
  const unreadAgentIds = new Set<string>();
  const diffsOffAgentIds = new Set<string>();
  // Each agent's TASK chip and the workspace that chip opens Watchtower on.
  const tasks = new Map<string, { taskId?: string; workspaceId: string | null }>();
  const cursors = new Set<string>();
  let cursor: string | undefined;
  do {
    if (signal.aborted) throw new Error("Board stopped.");
    const page = await paseo.agents.list({
      filter: { includeArchived: false },
      page: { limit: 200, ...(cursor ? { cursor } : {}) },
    });
    for (const { agent, project } of page.entries) {
      visibleAgentIds.add(agent.id);
      if (
        agent.requiresAttention &&
        (agent.attentionReason === "finished" || agent.attentionReason === "error")
      )
        unreadAgentIds.add(agent.id);
      if (agent.labels?.[EDIT_DIFFS_MODE_LABEL] === "off") diffsOffAgentIds.add(agent.id);
      tasks.set(agent.id, {
        taskId: agent.labels?.[WATCHTOWER_TASK_LABEL]?.match(/\bTASK-\d+\b/)?.[0],
        workspaceId: agent.workspaceId ?? null,
      });
      if (agent.status !== "running") continue;
      agents.push({
        ...agent,
        workspaceId: agent.workspaceId ?? null,
        parentAgentId: getParentAgentIdFromLabels(agent.labels),
        model: agent.model,
        effort: agent.effectiveThinkingOptionId ?? agent.thinkingOptionId ?? null,
        project: project?.projectName,
        projectKey: project?.projectKey,
      });
    }
    if (!page.pageInfo.hasMore) break;
    cursor = page.pageInfo.nextCursor ?? undefined;
    if (!cursor || cursors.has(cursor)) throw new Error("Agent list changed. Refresh to retry.");
    cursors.add(cursor);
  } while (cursor);
  return { agents, visibleAgentIds, unreadAgentIds, diffsOffAgentIds, tasks };
}
