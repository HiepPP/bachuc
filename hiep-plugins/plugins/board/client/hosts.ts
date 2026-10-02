import type { PluginClientContext } from "@getpaseo/plugin/client";

export type BoardHostClient = Pick<PluginClientContext, "rpc" | "openNewWorkspace">;

// Only the active host's client bundle loads. Borrow its RPC capability with an explicit
// target instead of requiring every host's Board bundle to execute in this window.
export function createBoardHostClient(client: BoardHostClient, serverId: string): BoardHostClient {
  return {
    rpc: (contract, input) => client.rpc(contract, input, { serverId }),
    openNewWorkspace: (input) => client.openNewWorkspace({ ...input, serverId }),
  };
}
