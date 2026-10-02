import type { PaseoApi } from "@getpaseo/client";
import { getParentAgentIdFromLabels } from "./agent-labels";
import type { ActiveAgent } from "./store";

export async function listBoardAgents(paseo: Pick<PaseoApi, "agents">, signal: AbortSignal) {
  const agents: ActiveAgent[] = [];
  const visibleAgentIds = new Set<string>();
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
  return { agents, visibleAgentIds };
}
