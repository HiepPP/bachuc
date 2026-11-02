import type { PluginSessionOpenRequest } from "@getpaseo/plugin/server";
import type { AgentModes } from "./modes";
import { DEFAULT_MODE, draftEnvKey, draftPrefsSchema } from "../shared/modes";

export async function seedDraftPrefs(
  request: PluginSessionOpenRequest,
  modes: AgentModes,
): Promise<PluginSessionOpenRequest> {
  const env = { ...request.env };
  const raw = env[draftEnvKey];
  delete env[draftEnvKey];
  if (
    request.reason === "create" &&
    request.purpose === "interactive" &&
    (await modes.find(request.agentId)) === null
  ) {
    const mode = raw === undefined ? DEFAULT_MODE : draftPrefsSchema.parse(JSON.parse(raw)).mode;
    await modes.set(request.agentId, mode);
  }
  return { ...request, env };
}
