import { defineRpc } from "@getpaseo/plugin";
import { z } from "zod";

export const taskSchema = z.strictObject({
  id: z.string().regex(/^[a-zA-Z0-9_-]{1,64}$/),
  goal: z.string().min(8).max(6000),
  acceptance: z.string().min(8).max(3000),
  kind: z.enum(["research", "implementation", "review"]),
  files: z.array(z.string().min(1).max(300)).min(1).max(30),
  resources: z.array(z.string().min(1).max(100)).max(12).default([]),
  dependsOn: z
    .array(z.string().regex(/^[a-zA-Z0-9_-]{1,64}$/))
    .max(20)
    .default([]),
  checks: z
    .array(
      z.strictObject({
        argv: z.array(z.string().min(1).max(2000)).min(1).max(30),
        timeoutMs: z.number().int().min(100).max(120000).default(30000),
      }),
    )
    .max(8)
    .default([]),
  profileId: z.string().max(100).optional(),
  allowedProfileIds: z.array(z.string().max(100)).min(1).max(20),
  discovery: z.enum(["auto", "always", "never"]).default("auto"),
  review: z.enum(["auto", "always", "never"]).default("auto"),
  maxAttempts: z.number().int().min(1).max(3).default(2),
  maxDurationMs: z.number().int().min(10000).max(1800000).default(600000),
  shareWithJev: z.literal(true),
});
export type Task = z.infer<typeof taskSchema>;
export const submitSchema = z.strictObject({ tasks: z.array(taskSchema).min(1).max(12) });
export const scopeSchema = z.strictObject({ parentId: z.string().min(1).max(120) });
export const jobsRpc = defineRpc({
  name: "jobs.list",
  input: scopeSchema,
  output: z.object({ jobs: z.array(z.unknown()), history: z.array(z.unknown()) }),
});
export const submitRpc = defineRpc({
  name: "jobs.submit",
  input: scopeSchema.extend(submitSchema.shape),
  output: z.object({ ids: z.array(z.string()) }),
});
export const cancelRpc = defineRpc({
  name: "jobs.cancel",
  input: scopeSchema.extend({ id: z.string() }),
  output: z.object({ cancelled: z.boolean() }),
});
