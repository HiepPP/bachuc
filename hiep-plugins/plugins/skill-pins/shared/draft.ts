import { z } from "zod";
import { skillsSchema } from "./catalog";

export const draftEnvKey = "PASEO_PLUGIN_COMPOSER_skill-pins";
export const draftSkillsSchema = z.object({ skills: skillsSchema });
