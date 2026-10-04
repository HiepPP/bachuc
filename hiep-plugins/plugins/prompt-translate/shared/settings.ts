import { defineSettings } from "@getpaseo/plugin";
import { z } from "zod";

// Fastest in the 2026-09-24 OpenRouter benchmark for both modes (total p50 935 ms translate,
// 858 ms enhance). The enhance model stays separately configurable.
export const DEFAULT_TRANSLATE_MODEL = "google/gemini-2.5-flash-lite";
export const DEFAULT_ENHANCE_MODEL = "google/gemini-2.5-flash-lite";

export const providerSchema = z.enum(["vercel", "openrouter"]);
export type Provider = z.output<typeof providerSchema>;
export const cavemanModeSchema = z.enum([
  "follow-agent",
  "lite",
  "full",
  "ultra",
  "wenyan-lite",
  "wenyan-full",
  "wenyan-ultra",
]);
export const rewriteModeSchema = z.enum(["automatic", "cmd-enter"]);
export type RewriteMode = z.output<typeof rewriteModeSchema>;
export const chineseScriptSchema = z.enum(["skill-default", "simplified"]);

export const translateSettings = defineSettings({
  id: "translate",
  scope: "host",
  version: 1,
  schema: z.object({
    translate: z.boolean().default(true),
    enhanceShortcut: z.boolean().default(true),
    rewriteMode: rewriteModeSchema.default("automatic"),
    matchReplyLanguage: z.boolean().default(true),
    cavemanMode: cavemanModeSchema.default("follow-agent"),
    chineseScript: chineseScriptSchema.default("skill-default"),
    provider: providerSchema.default("openrouter"),
    translateModel: z.string().trim().min(1).default(DEFAULT_TRANSLATE_MODEL),
    enhanceModel: z.string().trim().min(1).default(DEFAULT_ENHANCE_MODEL),
  }),
});
export type TranslateSettings = z.output<typeof translateSettings.schema>;

/**
 * Automatic rewrites on Enter and the send button; Cmd/Ctrl+Enter sends as typed.
 * Cmd+Enter mode rewrites only on Cmd/Ctrl+Enter. The button, dictation, and older hosts report no key.
 */
export function rewritesOnSendKey(
  sendKey: "enter" | "mod-enter" | undefined,
  mode: RewriteMode,
): boolean {
  return mode === "automatic" ? sendKey !== "mod-enter" : sendKey === "mod-enter";
}
