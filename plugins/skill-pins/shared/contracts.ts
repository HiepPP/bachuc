import { defineRpc } from "@getpaseo/plugin";
import { z } from "zod";

import { skillsSchema } from "./catalog";

export const MAX_TEXT = 200_000;
const agentId = z.string().uuid();
const token = z.string().min(1);
const queueId = z.string().min(1);
const skillsOutput = z.object({ skills: skillsSchema });

export const hostRpc = defineRpc({
  name: "skill-pins.host",
  input: z.object({}),
  output: z.object({ serverId: z.string().min(1) }),
});
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
export const prepareSkillsRpc = defineRpc({
  name: "skill-pins.prepare",
  input: z.object({ agentId, text: z.string().min(1).max(MAX_TEXT), skills: skillsSchema }),
  output: z.object({ token }),
});
export const cancelSkillsRpc = defineRpc({
  name: "skill-pins.cancel",
  input: z.object({ agentId, token }),
  output: z.object({}),
});
export const bindQueueSkillsRpc = defineRpc({
  name: "skill-pins.bind-queue",
  input: z.object({ agentId, token, queueId }),
  output: z.object({}),
});
export const cancelQueueSkillsRpc = defineRpc({
  name: "skill-pins.cancel-queue",
  input: z.object({ agentId, queueId }),
  output: z.object({}),
});
// New-thread composers have no agent ID yet. They share one host-level draft.
export const draftReadRpc = defineRpc({
  name: "skill-pins.draft-read",
  input: z.object({}),
  output: skillsOutput,
});
export const draftWriteRpc = defineRpc({
  name: "skill-pins.draft-write",
  input: z.object({ skills: skillsSchema }),
  output: skillsOutput,
});
