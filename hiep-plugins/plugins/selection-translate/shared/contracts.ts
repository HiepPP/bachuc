import { defineRpc } from "@getpaseo/plugin";
import { z } from "zod";

// Same limit as Cite MAX_QUOTE_LENGTH. The server checks the trimmed length, so the schema
// only bounds the raw payload.
export const MAX_SELECTION_LENGTH = 4000;

export const translateRpc = defineRpc({
  name: "selection.translate",
  input: z.object({ text: z.string() }),
  output: z.object({ translation: z.string() }),
});
