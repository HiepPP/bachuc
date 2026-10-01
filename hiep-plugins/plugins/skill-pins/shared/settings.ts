import { defineSettings } from "@getpaseo/plugin";
import { z } from "zod";
import { savedSkillsSchema } from "./catalog";

export const pinSettings = defineSettings({
  id: "skill-pins",
  scope: "host",
  version: 1,
  schema: z.object({ defaults: savedSkillsSchema.default([]) }),
});
