import { type PluginButtonLabelProps, useRpc } from "@getpaseo/plugin/client";
import { useQuery } from "@tanstack/react-query";
import { readBoardRpc } from "../shared/board";
import { pillLabel } from "./pill-label";

// The plugin's rpc and query providers wrap the pill trigger (PluginRuntimeBoundary in the host's
// ButtonEnvironment), so the label can read the board. The New workspace draft has no workspace.
export function WatchtowerPillLabel({ host, ...context }: PluginButtonLabelProps) {
  const read = useRpc(readBoardRpc);
  const workspaceId = context.context === "draft" ? null : context.workspaceId;
  const board = useQuery({
    queryKey: ["watchtower-board", host.id, workspaceId],
    queryFn: () => read({ workspaceId: workspaceId ?? "" }),
    enabled: workspaceId !== null,
    staleTime: 30_000,
    retry: false,
  });
  return workspaceId === null ? "Watchtower" : pillLabel(board.data);
}
