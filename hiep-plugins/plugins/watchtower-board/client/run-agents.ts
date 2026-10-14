import { usePaseo } from "@getpaseo/plugin/client";
import { useQuery } from "@tanstack/react-query";
import { useCallback } from "react";
import type { BuilderCandidate } from "./builders";

// Same bounds as agent-target.ts: 200 agents per page, newest first, so a run's builders sit on
// the first page.
const MAX_PAGES = 3;
const REFRESH_MS = 15_000;
const NO_AGENTS: BuilderCandidate[] = [];

// The agents of one workspace for `matchBuilders`. Empty until the first lookup succeeds.
export function useRunAgents(hostId: string, workspaceId: string): BuilderCandidate[] {
  const paseo = usePaseo();
  const find = useCallback(async (): Promise<BuilderCandidate[]> => {
    const found: BuilderCandidate[] = [];
    let cursor: string | undefined;
    for (let page = 0; page < MAX_PAGES; page++) {
      const result = await paseo.agents.list({
        filter: { includeArchived: false },
        sort: [{ key: "updated_at", direction: "desc" }],
        page: { limit: 200, ...(cursor ? { cursor } : {}) },
      });
      found.push(...result.entries.map((entry) => entry.agent));
      if (!result.pageInfo.hasMore || !result.pageInfo.nextCursor) break;
      cursor = result.pageInfo.nextCursor;
    }
    return found.filter((agent) => agent.workspaceId === workspaceId);
  }, [paseo, workspaceId]);
  const { data } = useQuery({
    queryKey: ["watchtower-run-agents", hostId, workspaceId],
    queryFn: find,
    refetchInterval: REFRESH_MS,
    retry: false,
  });
  return data ?? NO_AGENTS;
}
