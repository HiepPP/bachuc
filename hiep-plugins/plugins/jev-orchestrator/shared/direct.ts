import { defineRpc } from "@getpaseo/plugin";
import { z } from "zod";

export const effortSchema = z.enum(["low", "medium", "high", "xhigh", "max"]);
export const modelAllowlistSchema = z
  .array(
    z.strictObject({
      provider: z.string().min(1).max(100),
      model: z.string().min(1).max(150),
      effortIds: z
        .array(effortSchema)
        .min(1)
        .max(5)
        .refine((ids) => new Set(ids).size === ids.length, "Duplicate effort."),
    }),
  )
  .min(1)
  .max(8)
  .refine(
    (entries) =>
      new Set(entries.map((e) => JSON.stringify([e.provider, e.model]))).size === entries.length,
    "Duplicate model.",
  );
export type ModelAllowlist = z.infer<typeof modelAllowlistSchema>;
export function isLunaModel(model: string) {
  return /(^|[/-])luna([/-]|$)/i.test(model);
}

export const directInput = z.strictObject({
  workspaceId: z.string().min(1).max(120),
  requestId: z.uuid(),
  prompt: z
    .string()
    .min(8)
    .max(12000)
    .refine(
      (value) => value.trim().length >= 8,
      "Task must contain at least 8 non-padding characters.",
    ),
  allowedProfileIds: z.array(z.string().min(1).max(100)).min(1).max(8),
  allowedModels: modelAllowlistSchema,
  shareWithJev: z.literal(true),
});
export type DirectInput = z.infer<typeof directInput>;
export const directProfilesRpc = defineRpc({
  name: "direct.profiles",
  input: z.strictObject({ workspaceId: z.string().min(1) }),
  output: z.object({
    profiles: z.array(
      z.object({
        id: z.string(),
        name: z.string(),
        provider: z.string(),
        model: z.string(),
        modeId: z.string().optional(),
        notes: z.string().optional(),
        effortIds: z.array(effortSchema),
      }),
    ),
  }),
});
export const directRunRpc = defineRpc({
  name: "direct.run",
  input: directInput,
  output: z.object({
    agentId: z.string(),
    selection: z.object({
      id: z.string(),
      name: z.string(),
      provider: z.string(),
      model: z.string(),
      modeId: z.string().optional(),
      thinkingOptionId: z.string(),
      featureValues: z.record(z.string(), z.unknown()).optional(),
    }),
    routingMs: z.number(),
    usage: z.unknown(),
  }),
});
export const directStatusRpc = defineRpc({
  name: "direct.status",
  input: z.strictObject({ workspaceId: z.string().min(1), requestId: z.uuid().optional() }),
  output: z.object({ records: z.array(z.unknown()) }),
});
