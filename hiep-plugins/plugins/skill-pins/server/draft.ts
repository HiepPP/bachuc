import type { PluginSessionOpenRequest } from "@getpaseo/plugin/server";
import type { SkillPins } from "./state";
import type { SkillId } from "../shared/catalog";
import { draftEnvKey, draftSkillsSchema } from "../shared/draft";

export async function seedDraftSkills(
  request: PluginSessionOpenRequest,
  pins: SkillPins,
  defaults: () => Promise<SkillId[]>,
): Promise<PluginSessionOpenRequest> {
  const env = { ...request.env };
  const raw = env[draftEnvKey];
  delete env[draftEnvKey];
  if (request.reason === "create" && request.purpose === "interactive") {
    const skills =
      raw === undefined ? await defaults() : draftSkillsSchema.parse(JSON.parse(raw)).skills;
    await pins.seed(request.agentId, skills);
  }
  return { ...request, env };
}
