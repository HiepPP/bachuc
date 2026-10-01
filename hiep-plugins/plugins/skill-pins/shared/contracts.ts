import { defineRpc } from "@getpaseo/plugin";
import { z } from "zod";

import { skillsSchema } from "./catalog";

const agentId = z.string().uuid();
const skillsOutput = z.object({ skills: skillsSchema });

export const skillsReadRpc = defineRpc({
  name: "skill-pins.read",
  input: z.object({ agentId }),
  output: skillsOutput,
});
export const skillsWriteRpc = defineRpc({
  name: "skill-pins.write",
  input: z.object({ agentId, skills: skillsSchema }),
  output: skillsOutput,
});
