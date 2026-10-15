import { type PluginButtonLabelProps, useRpc } from "@getpaseo/plugin/client";
import { useQuery } from "@tanstack/react-query";
import { useMemo } from "react";
import { readBoardRpc } from "../shared/board";
import { readOverviewRpc } from "../shared/overview";
import { liveTaskIds, matchBuilders } from "./builders";
import { pillLabel } from "./pill-label";
import { useRunAgents } from "./run-agents";

// The plugin's rpc and query providers wrap the pill trigger (PluginRuntimeBoundary in the host's
// ButtonEnvironment), so the label can read the board. The New workspace draft has no workspace.
export function WatchtowerPillLabel({ host, ...context }: PluginButtonLabelProps) {
  const read = useRpc(readBoardRpc);
  const readOverview = useRpc(readOverviewRpc);
  const workspaceId = context.context === "draft" ? null : context.workspaceId;
  const board = useQuery({
    queryKey: ["watchtower-board", host.id, workspaceId],
    queryFn: () => read({ workspaceId: workspaceId ?? "" }),
    enabled: workspaceId !== null,
    staleTime: 30_000,
    retry: false,
  });
  // Builders match TASKs through the overview's groups, as in the plan tree. The queries share
  // their keys with the Watchtower page, so every pill of a workspace reads one copy.
  const overview = useQuery({
    queryKey: ["watchtower-overview", host.id, workspaceId],
    queryFn: () => readOverview({ workspaceId: workspaceId ?? "" }),
    enabled: workspaceId !== null,
    staleTime: 30_000,
    retry: false,
  });
  const runAgents = useRunAgents(host.id, workspaceId ?? "");
  const liveKey = useMemo(
    () => liveTaskIds(matchBuilders(runAgents, overview.data?.tasks ?? [])).join(","),
    [runAgents, overview.data],
  );
  const live = useMemo(() => new Set(liveKey ? liveKey.split(",") : []), [liveKey]);
  return workspaceId === null ? "Watchtower" : pillLabel(board.data, live);
}
