import { defineSettings } from "@getpaseo/plugin";
import { z } from "zod";

export const IDLE_HOURS_DEFAULT = 24;
export const IDLE_HOURS_MIN = 1;

export const janitorSettings = defineSettings({
  id: "janitor",
  scope: "host",
  version: 1,
  schema: z.object({
    enabled: z.boolean().default(true),
    keepRecent: z.number().int().min(0).default(7),
    idleHours: z.number().min(IDLE_HOURS_MIN).default(IDLE_HOURS_DEFAULT),
  }),
});

export type JanitorSettings = z.output<typeof janitorSettings.schema>;
