import { defineSettings } from "@getpaseo/plugin";
import { z } from "zod";

export const displaySettings = defineSettings({
  id: "display",
  scope: "host",
  version: 1,
  schema: z.object({
    // Off by default: expanded rows each locate and highlight, which slows long threads.
    autoExpand: z.boolean().default(false),
  }),
});
