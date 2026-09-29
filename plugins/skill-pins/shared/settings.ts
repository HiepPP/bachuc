import { defineSettings } from "@getpaseo/plugin";
import { z } from "zod";
import { savedSkillsSchema } from "./catalog";

// Agents created through Paseo receive the initial set in this env variable. The hook reads it
// until the agent has its own skills.json.
export const INITIAL_ENV = "SKILL_PINS_INITIAL";

export const pinSettings = defineSettings({
  id: "skill-pins",
  scope: "host",
  version: 1,
  schema: z.object({ defaults: savedSkillsSchema.default([]) }),
});
