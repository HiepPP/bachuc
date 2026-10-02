import { z } from "zod";
import { cavemanModeSchema } from "./settings";

export const draftEnvKey = "PASEO_PLUGIN_COMPOSER_prompt-translate";
export const draftPrefsSchema = z.object({ mode: cavemanModeSchema, rewrite: z.boolean() });
