import type { PluginServerContext } from "@getpaseo/plugin/server";
import { loadWorkspaceBoard, searchTaskAttachments } from "./server/handlers";
import { readBoardRpc, searchTasksRpc } from "./shared/board";

export default function contribute(server: PluginServerContext) {
  server.handle(readBoardRpc, ({ workspaceId }, { paseo }) =>
    loadWorkspaceBoard(workspaceId, paseo),
  );
  server.handle(searchTasksRpc, ({ query }, { paseo }) => searchTaskAttachments(query, paseo));
  return () => {};
}
