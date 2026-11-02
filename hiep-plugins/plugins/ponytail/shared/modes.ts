import { z } from "zod";

export const ponytailModeSchema = z.enum(["off", "lite", "full", "ultra"]);
export type PonytailMode = z.infer<typeof ponytailModeSchema>;

/** The mode of an agent with no saved mode. */
export const DEFAULT_MODE: PonytailMode = "full";

export const draftEnvKey = "PASEO_PLUGIN_COMPOSER_ponytail";
export const draftPrefsSchema = z.object({ mode: ponytailModeSchema });
