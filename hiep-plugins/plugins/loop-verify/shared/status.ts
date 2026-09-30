import { z } from "zod";

export const STATUS_KIND = "loop-verify-status";
export const STATUS_VERSION = 1;
export const STATUS_OUTPUT_TAIL = 1500;

export const loopStatusSchema = z.object({
  rootId: z.string(),
  agentId: z.string(),
  round: z.number().int(),
  maxRounds: z.number().int(),
  verify: z.string(),
  exitCode: z.number().nullable(),
  status: z.enum(["running", "paused", "passed", "exhausted", "stopped"]),
  output: z.string(),
  nextAgentId: z.string().nullable(),
});

export type LoopStatus = z.output<typeof loopStatusSchema>;
