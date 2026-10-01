import { defineSettings } from "@getpaseo/plugin";
import { z } from "zod";

export const closerSettings = defineSettings({
  id: "closer",
  scope: "host",
  version: 1,
  schema: z.object({
    enabled: z.boolean().default(true),
    idleMinutes: z.number().min(1).default(30),
  }),
});

export type CloserSettings = z.output<typeof closerSettings.schema>;
