import type { PluginServerContext } from "@getpaseo/plugin/server";
import { readFileSync, existsSync } from "node:fs";
import { homedir } from "node:os";
import path from "node:path";

export default function contribute(server: PluginServerContext) {
  const configPath = path.join(
    process.env.PASEO_HOME || path.join(homedir(), ".paseo"),
    "config.json",
  );
  const config = JSON.parse(readFileSync(configPath, "utf8"));
  // Directory installs keep the stdio entry at a stable path across plugin reloads.
  const pluginPath: unknown = config.plugins?.["jev-evaluator"]?.path;
  if (typeof pluginPath !== "string" || !path.isAbsolute(pluginPath)) {
    throw new Error("Install this directory plugin with the ID jev-evaluator.");
  }
  const entry = path.join(pluginPath, "server", "mcp.mjs");
  if (!existsSync(entry)) throw new Error("Jev MCP entry is missing. Reinstall the plugin source.");

  server.before("agent.create", ({ request }) => {
    if (request.config.provider !== "codex" && request.config.provider !== "claude") return request;
    if (request.config.mcpServers?.jev) return request;
    return {
      ...request,
      config: {
        ...request.config,
        mcpServers: {
          ...request.config.mcpServers,
          jev: {
            type: "stdio",
            command: process.execPath,
            args: [entry],
            env: { ELECTRON_RUN_AS_NODE: "1", PASEO_JEV_CONFIG_PATH: configPath },
            alwaysLoad: true,
          },
        },
      },
    };
  });
  return () => {};
}
