import type { PluginBeforeRequests } from "@getpaseo/plugin/server";
import path from "node:path";

export function injectPreflight(
  request: PluginBeforeRequests["agent.create"],
  pluginPath: string,
  workspace: string,
) {
  if (
    !["codex", "claude"].includes(request.config.provider) ||
    request.config.mcpServers?.workspace_preflight
  )
    return request;
  if (!path.isAbsolute(workspace) || !path.isAbsolute(pluginPath))
    throw new Error("Absolute MCP paths required.");
  return {
    ...request,
    config: {
      ...request.config,
      mcpServers: {
        ...request.config.mcpServers,
        workspace_preflight: {
          type: "stdio" as const,
          command: process.execPath,
          args: [
            "--import",
            path.join(pluginPath, "node_modules/tsx/dist/loader.mjs"),
            path.join(pluginPath, "server/mcp.ts"),
          ],
          env: {
            ELECTRON_RUN_AS_NODE: "1",
            PATH: process.env.PATH ?? "",
            PASEO_PREFLIGHT_WORKSPACE: workspace,
            NODE_OPTIONS: "",
          },
          alwaysLoad: true,
        },
      },
    },
  };
}
