import type { PluginClientContext } from "@getpaseo/plugin/client";

export type BoardHostClient = Pick<PluginClientContext, "rpc" | "openNewWorkspace"> & {
  // Opens the Watchtower page of watchtower-board on this host, at a workspace and TASK.
  openWatchtower?: (input: { workspaceId: string; task?: string }) => void;
};

// Only the active host's client bundle loads. Borrow its RPC capability with an explicit
// target instead of requiring every host's Board bundle to execute in this window.
export function createBoardHostClient(
  client: Pick<PluginClientContext, "rpc" | "openNewWorkspace"> &
    Partial<Pick<PluginClientContext, "openSurface">>,
  serverId: string,
): BoardHostClient {
  const openSurface = client.openSurface?.bind(client);
  return {
    rpc: (contract, input) => client.rpc(contract, input, { serverId }),
    openNewWorkspace: (input) => client.openNewWorkspace({ ...input, serverId }),
    openWatchtower: openSurface
      ? ({ workspaceId, task }) =>
          openSurface("watchtower", {
            pluginId: "watchtower-board",
            serverId,
            presentation: "overlay",
            params: task ? { workspaceId, task } : { workspaceId },
          })
      : undefined,
  };
}
