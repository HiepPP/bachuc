import type { PluginSessionOpenRequest } from "@getpaseo/plugin/server";
import type { AgentModes } from "./modes";
import type { TranslateSettings } from "../shared/settings";
import { draftEnvKey, draftPrefsSchema } from "../shared/draft";

export async function seedDraftPrefs(
  request: PluginSessionOpenRequest,
  modes: AgentModes,
  defaults: () => Promise<TranslateSettings>,
): Promise<PluginSessionOpenRequest> {
  const env = { ...request.env };
  const raw = env[draftEnvKey];
  delete env[draftEnvKey];
  if (
    request.reason === "create" &&
    request.purpose === "interactive" &&
    (await modes.find(request.agentId)) === null
  ) {
    const mode =
      raw === undefined
        ? (await defaults()).cavemanMode
        : draftPrefsSchema.parse(JSON.parse(raw)).mode;
    await modes.set(request.agentId, mode);
  }
  return { ...request, env };
}
