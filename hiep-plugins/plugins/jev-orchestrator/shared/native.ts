import { z } from "zod";

export const nativePrepareSchema = z.strictObject({
  requestId: z.string().regex(/^[a-zA-Z0-9_-]{1,64}$/),
  task: z.string().trim().min(1).max(16000),
  sourceRole: z.string().trim().min(1).max(100),
  forkTurns: z.literal("none"),
  shareWithJev: z.literal(true),
});
