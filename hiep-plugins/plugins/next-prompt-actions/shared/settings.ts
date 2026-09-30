import { defineSettings } from "@getpaseo/plugin";
import { z } from "zod";

export const sendSettings = defineSettings({
  id: "send",
  scope: "host",
  version: 1,
  schema: z.object({
    backToBoard: z.boolean().default(false),
  }),
});
