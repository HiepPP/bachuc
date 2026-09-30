import type { PluginServerContext } from "@getpaseo/plugin/server";
import { readFileSync, existsSync } from "node:fs";
import { realpath } from "node:fs/promises";
import { homedir } from "node:os";
import path from "node:path";
import { injectPreflight } from "./server/injection";
import { runPreflight } from "./server/preflight";
import { preflightRpc } from "./shared/preflight";

export default function contribute(server: PluginServerContext) {
  server.handle(preflightRpc, async ({ workspaceId }, { paseo }) => {
    const workspace = await paseo.workspaces.ref(workspaceId).refresh();
    if (!workspace) throw new Error("Workspace unavailable on this host.");
    return runPreflight(workspace.workspaceDirectory);
  });
  const configPath = path.join(
    process.env.PASEO_HOME || path.join(homedir(), ".paseo"),
    "config.json",
  );
  const config = JSON.parse(readFileSync(configPath, "utf8"));
  const pluginPath: unknown = config.plugins?.["workspace-preflight"]?.path;
  if (
    typeof pluginPath !== "string" ||
    !path.isAbsolute(pluginPath) ||
    !existsSync(path.join(pluginPath, "server/mcp.ts"))
  )
    throw new Error(
      "Install this directory plugin under ID workspace-preflight with dependencies.",
    );
  const remove = server.before("agent.create", async ({ request }) => {
    if (
      !["codex", "claude"].includes(request.config.provider) ||
      request.config.mcpServers?.workspace_preflight
    )
      return request;
    // Paseo binds config.cwd to the selected workspace before this hook; plugins cannot change it.
    return injectPreflight(request, pluginPath, await realpath(request.config.cwd));
  });
  return remove;
}
