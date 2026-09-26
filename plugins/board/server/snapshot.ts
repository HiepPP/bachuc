import { getParentAgentIdFromLabels } from "@getpaseo/protocol/agent-labels";
import type { PaseoApi } from "@getpaseo/client";
import type { ActiveAgent } from "./store";

export async function listRunning(paseo: Pick<PaseoApi, "agents">, signal: AbortSignal) {
  const agents: ActiveAgent[] = [];
  const cursors = new Set<string>();
  let cursor: string | undefined;
  do {
    if (signal.aborted) throw new Error("Board stopped.");
    const page = await paseo.agents.list({
      filter: { statuses: ["running"], includeArchived: false },
      page: { limit: 200, ...(cursor ? { cursor } : {}) },
    });
    for (const { agent, project } of page.entries) {
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
  return agents;
}
