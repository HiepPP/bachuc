import { defineRpc } from "@getpaseo/plugin";
import { z } from "zod";
import { taskSchema } from "./board";

// The overview shows no brief bodies; dropping them keeps a 10-task plan near 8 KB instead of 46 KB.
export const overviewTaskSchema = taskSchema.omit({ brief: true }).extend({
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
// Minutes count from the run's `Started:` time. Null means the time is `now`, `-`, or unreadable.
export const overviewRunEntrySchema = z.object({
  task: z.string(),
  result: z.string(),
  detail: z.string(),
  start: z.string(),
  end: z.string(),
  startMinute: z.number().int().nullable(),
  endMinute: z.number().int().nullable(),
});
export const overviewRunSchema = z.object({
  runner: z.string().nullable(),
  started: z.string().nullable(),
  finished: z.string().nullable(),
  // Every log row, oldest first.
  log: z.array(overviewRunEntrySchema),
  // Minutes from `Started:` to the read time, or to `Finished:` once the run ends. A stopped run
  // keeps the minute of its last log activity.
  nowMinute: z.number().int().nullable(),
  // No `Finished:` line and no log activity for more than two hours.
  stopped: z.boolean(),
});
// An OPEN or DEFAULTED row of QUESTIONS.md.
export const overviewQuestionSchema = z.object({
  id: z.string(),
  tasks: z.array(z.string()),
  blocks: z.array(z.string()),
  question: z.string(),
  default: z.string(),
  status: z.string(),
});
export const decisionSchema = z.object({
  id: z.string(),
  date: z.string(),
  title: z.string(),
  status: z.string(),
});
// One folder of watchtower/archive/, newest first.
export const historySchema = z.object({
  slug: z.string(),
  date: z.string().nullable(),
  title: z.string(),
  hasLearn: z.boolean(),
});
export const HISTORY_LIMIT = 20;
export const overviewSchema = z.object({
  plan: overviewPlanSchema,
  tasks: z.array(overviewTaskSchema),
  planVerify: z.array(z.string()),
  manualChecks: z.array(manualCheckSchema),
  run: overviewRunSchema.nullable(),
  questions: z.array(overviewQuestionSchema),
  decisions: z.array(decisionSchema),
  history: z.array(historySchema),
  message: z.string().nullable(),
  warnings: z.array(z.string()),
});
export type Overview = z.infer<typeof overviewSchema>;
export type OverviewTask = z.infer<typeof overviewTaskSchema>;
export type ManualCheck = z.infer<typeof manualCheckSchema>;
export type OverviewRun = z.infer<typeof overviewRunSchema>;
export type OverviewRunEntry = z.infer<typeof overviewRunEntrySchema>;
export type OverviewQuestion = z.infer<typeof overviewQuestionSchema>;
export type Decision = z.infer<typeof decisionSchema>;
export type History = z.infer<typeof historySchema>;
export const readOverviewRpc = defineRpc({
  name: "watchtower.overview.read",
  input: z.object({ workspaceId: z.string().min(1).max(256) }),
  output: overviewSchema,
});

// The ID of a DECISIONS.md row. The server matches it against file names, never joins it into a path.
export const DECISION_ID = /^ADR-\d{1,6}$/;
export const decisionDetailSchema = z.object({
  id: z.string(),
  title: z.string(),
  status: z.string(),
  // The ADR file, relative to the workspace root.
  file: z.string(),
  markdown: z.string(),
});
export type DecisionDetail = z.infer<typeof decisionDetailSchema>;
export const readDecisionRpc = defineRpc({
  name: "watchtower.decision.read",
  input: z.object({ workspaceId: z.string().min(1).max(256), id: z.string().regex(DECISION_ID) }),
  output: decisionDetailSchema,
});
