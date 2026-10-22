import { defineSettings } from "@getpaseo/plugin";
import { z } from "zod";

// Same default as prompt-translate DEFAULT_TRANSLATE_MODEL (2026-09-24 OpenRouter benchmark).
export const DEFAULT_TRANSLATE_MODEL = "google/gemini-2.5-flash-lite";

export const providerSchema = z.enum(["vercel", "openrouter"]);
export type Provider = z.output<typeof providerSchema>;

export const translateSettings = defineSettings({
  id: "translate",
  scope: "host",
  version: 1,
  schema: z.object({
    provider: providerSchema.default("openrouter"),
    model: z.string().trim().min(1).default(DEFAULT_TRANSLATE_MODEL),
  }),
});
export type TranslateSettings = z.output<typeof translateSettings.schema>;
