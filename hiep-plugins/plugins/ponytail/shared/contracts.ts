import { defineRpc } from "@getpaseo/plugin";
import { z } from "zod";

import { ponytailModeSchema } from "./modes";

const agentId = z.string().uuid();
const saved = z.object({ mode: ponytailModeSchema });

export const modeReadRpc = defineRpc({
  name: "ponytail.mode.read",
  input: z.object({ agentId }),
  output: saved,
});
export const modeWriteRpc = defineRpc({
  name: "ponytail.mode.write",
  input: z.object({ agentId, mode: ponytailModeSchema }),
  output: saved,
});
