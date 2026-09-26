import { existsSync } from "node:fs";
import path from "node:path";
import type { PluginSessionOpenRequest } from "@getpaseo/plugin/server";

// caveman-hook.cjs owns Caveman mode for Paseo Claude agents. The native Claude Caveman
// plugin injects its default level at SessionStart and every turn, overriding the chosen
// mode, so launch it with default "off". The bridge restores the saved user default.
export function withBridgeCavemanEnv(
  request: PluginSessionOpenRequest,
  dataDir: string,
  inherited: NodeJS.ProcessEnv = process.env,
): PluginSessionOpenRequest | void {
  if (request.provider !== "claude" || !existsSync(path.join(dataDir, "hook-runtime.json"))) return;
  const env = request.env ?? {};
  return {
    ...request,
    env: {
      ...env,
      PROMPT_TRANSLATE_CAVEMAN_DEFAULT_MODE:
        env.PROMPT_TRANSLATE_CAVEMAN_DEFAULT_MODE ??
        env.CAVEMAN_DEFAULT_MODE ??
        inherited.CAVEMAN_DEFAULT_MODE ??
        "",
      CAVEMAN_DEFAULT_MODE: "off",
    },
  };
}
