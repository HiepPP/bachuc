import { usePaseo } from "@getpaseo/plugin/client";
import { useToast } from "@getpaseo/plugin/client/react-native";
import { useQuery } from "@tanstack/react-query";
import { useCallback } from "react";
import { type AgentCandidate, type AgentTarget, chooseAgent } from "./actions";

// 200 agents per page, newest first; the target is almost always on the first page.
const MAX_PAGES = 3;
const REFRESH_MS = 15_000;

export interface AgentSender {
  // Null until a lookup succeeds. Send looks again, so it still works and reports the error.
  target: AgentTarget | null;
  // Sends the prompt and shows a toast. Resolves true when the agent took it.
  send(prompt: string): Promise<boolean>;
}

// `enabled` false skips the periodic agent lookup, for a page with nothing to send.
export function useAgentSender(hostId: string, workspaceId: string, enabled: boolean): AgentSender {
  const paseo = usePaseo();
  const toast = useToast();
  const find = useCallback(async (): Promise<AgentTarget> => {
    const found: AgentCandidate[] = [];
    let cursor: string | undefined;
    for (let page = 0; page < MAX_PAGES; page++) {
      const result = await paseo.agents.list({
        filter: { includeArchived: false },
        sort: [{ key: "updated_at", direction: "desc" }],
        page: { limit: 200, ...(cursor ? { cursor } : {}) },
      });
      found.push(...result.entries.map((entry) => entry.agent));
      if (chooseAgent(found, workspaceId).state === "ready") break;
      if (!result.pageInfo.hasMore || !result.pageInfo.nextCursor) break;
      cursor = result.pageInfo.nextCursor;
    }
    return chooseAgent(found, workspaceId);
  }, [paseo, workspaceId]);
  const { data, refetch } = useQuery({
    queryKey: ["watchtower-agent", hostId, workspaceId],
    queryFn: find,
    enabled,
    refetchInterval: REFRESH_MS,
    retry: false,
  });

  const send = useCallback(
    async (prompt: string) => {
      try {
        // Look again: the agent may have started a turn since the last refresh.
        const target = await find();
        if (target.state === "none") throw new Error("No agent in this workspace.");
        if (target.state === "busy") throw new Error("Agent busy.");
        await paseo.agents.ref(target.id).send(prompt);
        toast.show(`Sent to ${target.name}`, { variant: "success" });
        void refetch();
        return true;
      } catch (error) {
        toast.error(error instanceof Error ? error.message : "Could not send the prompt.");
        return false;
      }
    },
    [find, paseo, toast, refetch],
  );
  return { target: data ?? null, send };
}
