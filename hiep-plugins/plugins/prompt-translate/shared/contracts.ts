import { defineRpc } from "@getpaseo/plugin";
import { z } from "zod";

import { cavemanModeSchema } from "./settings";

export const MAX_TEXT = 20_000;
const text = z.string().min(1).max(MAX_TEXT);

export const hostRpc = defineRpc({
  name: "translate.host",
  input: z.object({}),
  output: z.object({ serverId: z.string().min(1) }),
});

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
export const modeReadRpc = defineRpc({
  name: "translate.mode.read",
  input: z.object({ agentId }),
  output: z.object({ mode: cavemanModeSchema }),
});
export const modeWriteRpc = defineRpc({
  name: "translate.mode.write",
  input: z.object({ agentId, mode: cavemanModeSchema }),
  output: z.object({ mode: cavemanModeSchema }),
});
export const prepareModeRpc = defineRpc({
  name: "translate.mode.prepare",
  input: z.object({ agentId, text, source: text, mode: cavemanModeSchema }),
  output: z.object({ token: z.string() }),
});
export const cancelModeRpc = defineRpc({
  name: "translate.mode.cancel",
  input: z.object({ agentId, token: z.string() }),
  output: z.object({}),
});

export const bindQueueModeRpc = defineRpc({
  name: "translate.mode.bind-queue",
  input: z.object({ agentId, token: z.string(), queueId: z.string().min(1) }),
  output: z.object({}),
});
export const cancelQueueModeRpc = defineRpc({
  name: "translate.mode.cancel-queue",
  input: z.object({ agentId, queueId: z.string().min(1) }),
  output: z.object({}),
});
