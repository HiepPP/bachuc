import type { PluginServerContext } from "@getpaseo/plugin/server";
import {
  loadProjectBoard,
  loadWorkspaceBoard,
  loadWorkspaceDecision,
  loadWorkspaceOverview,
  searchTaskAttachments,
} from "./server/handlers";
import { readBoardRpc, readProjectBoardRpc, searchTasksRpc } from "./shared/board";
import { readDecisionRpc, readOverviewRpc } from "./shared/overview";

export default function contribute(server: PluginServerContext) {
  server.handle(readBoardRpc, ({ workspaceId }, { paseo }) =>
    loadWorkspaceBoard(workspaceId, paseo),
  );
  server.handle(readProjectBoardRpc, ({ projectId }, { paseo }) =>
    loadProjectBoard(projectId, paseo),
  );
  server.handle(readOverviewRpc, ({ workspaceId }, { paseo }) =>
    loadWorkspaceOverview(workspaceId, paseo),
  );
  server.handle(readDecisionRpc, ({ workspaceId, id }, { paseo }) =>
    loadWorkspaceDecision(workspaceId, id, paseo),
  );
  server.handle(searchTasksRpc, ({ query }, { paseo }) => searchTaskAttachments(query, paseo));
  return () => {};
}
