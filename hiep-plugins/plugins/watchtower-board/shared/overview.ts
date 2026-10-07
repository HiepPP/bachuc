import { defineRpc } from "@getpaseo/plugin";
import { z } from "zod";
import { taskSchema } from "./board";

export const overviewTaskSchema = taskSchema.extend({
  // TASK IDs named in the Deps cell; `-` gives an empty list.
  depIds: z.array(z.string()),
  // The Tracker Group cell, or null when the Tracker has no Group column.
  group: z.string().nullable(),
});
export const overviewPlanSchema = z.object({
  title: z.string(),
  slug: z.string().nullable(),
  status: z.string().nullable(),
  updated: z.string().nullable(),
});
// A `## Handoff` bullet that starts with `Manual check pending:`.
export const manualCheckSchema = z.object({
  text: z.string(),
  tasks: z.array(z.string()),
});
export const overviewSchema = z.object({
  plan: overviewPlanSchema,
  tasks: z.array(overviewTaskSchema),
  planVerify: z.array(z.string()),
  manualChecks: z.array(manualCheckSchema),
  message: z.string().nullable(),
  warnings: z.array(z.string()),
});
export type Overview = z.infer<typeof overviewSchema>;
export type OverviewTask = z.infer<typeof overviewTaskSchema>;
export type ManualCheck = z.infer<typeof manualCheckSchema>;
export const readOverviewRpc = defineRpc({
  name: "watchtower.overview.read",
  input: z.object({ workspaceId: z.string().min(1).max(256) }),
  output: overviewSchema,
});
