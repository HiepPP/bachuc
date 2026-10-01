import { defineRpc } from "@getpaseo/plugin";
import { z } from "zod";

import { cavemanModeSchema } from "./settings";

export const MAX_TEXT = 20_000;
const text = z.string().min(1).max(MAX_TEXT);

export const translateRpc = defineRpc({
  name: "translate.translate",
  input: z.object({ text, cacheOnly: z.boolean().default(false) }),
  output: z.object({ translation: z.string().nullable() }),
});
export const enhanceRpc = defineRpc({
  name: "translate.enhance",
  input: z.object({ text, deferCaveman: z.boolean().default(false) }),
  output: z.object({ prompt: z.string() }),
});
export const originalRpc = defineRpc({
  name: "translate.original",
  input: z.object({ text }),
  output: z.object({ original: z.string().nullable() }),
});

const agentId = z.string().uuid();
const prefs = z.object({ mode: cavemanModeSchema, rewrite: z.boolean() });
export const modeReadRpc = defineRpc({
  name: "translate.mode.read",
  input: z.object({ agentId }),
  output: prefs,
});
export const modeWriteRpc = defineRpc({
  name: "translate.mode.write",
  input: z.object({ agentId, mode: cavemanModeSchema.optional(), rewrite: z.boolean().optional() }),
  output: prefs,
});
