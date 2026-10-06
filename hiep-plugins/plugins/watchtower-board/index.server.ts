import type { PluginServerContext } from "@getpaseo/plugin/server";
import { loadProjectBoard, loadWorkspaceBoard, searchTaskAttachments } from "./server/handlers";
import { readBoardRpc, readProjectBoardRpc, searchTasksRpc } from "./shared/board";

export default function contribute(server: PluginServerContext) {
  server.handle(readBoardRpc, ({ workspaceId }, { paseo }) =>
    loadWorkspaceBoard(workspaceId, paseo),
  );
  server.handle(readProjectBoardRpc, ({ projectId }, { paseo }) =>
    loadProjectBoard(projectId, paseo),
  );
  server.handle(searchTasksRpc, ({ query }, { paseo }) => searchTaskAttachments(query, paseo));
  return () => {};
}
