import {
  defineAttachmentSource,
  defineRpc,
  PluginAttachmentSearchPayloadSchema,
} from "@getpaseo/plugin";
import { z } from "zod";

export const taskSchema = z.object({
  id: z.string(),
  title: z.string(),
  status: z.string(),
  deps: z.string(),
  notes: z.string(),
  spec: z.string(),
  // The spec's `Class:` line; Watchtower reads a missing class as `risky`.
  taskClass: z.string().nullable(),
  brief: z.string().nullable(),
  blocker: z.string().nullable(),
  error: z.string().nullable(),
});
// An OPEN row of QUESTIONS.md whose Blocks cell names at least one TASK.
export const questionSchema = z.object({
  id: z.string(),
  question: z.string(),
  blocks: z.array(z.string()),
});
export const runEntrySchema = z.object({
  start: z.string(),
  end: z.string(),
  task: z.string(),
  result: z.string(),
  detail: z.string(),
});
export const runSchema = z.object({
  runner: z.string().nullable(),
  schedule: z.string().nullable(),
  profile: z.string().nullable(),
  started: z.string().nullable(),
  finished: z.string().nullable(),
  // Newest first, at most RUN_LOG_LIMIT rows.
  log: z.array(runEntrySchema),
  total: z.number().int(),
});
export const RUN_LOG_LIMIT = 5;
export const boardSchema = z.object({
  title: z.string(),
  tasks: z.array(taskSchema),
  message: z.string().nullable(),
  questions: z.array(questionSchema),
  run: runSchema.nullable(),
  proposedAdrs: z.number().int(),
  // Unreadable QUESTIONS.md, RUN.md, or DECISIONS.md; a missing file is not a warning.
  warnings: z.array(z.string()),
});
export type Board = z.infer<typeof boardSchema>;
export type Task = z.infer<typeof taskSchema>;
export type Question = z.infer<typeof questionSchema>;
export type Run = z.infer<typeof runSchema>;
export const readBoardRpc = defineRpc({
  name: "watchtower.read",
  input: z.object({ workspaceId: z.string().min(1).max(256) }),
  output: boardSchema,
});
// The new workspace screen has a project but no workspace yet.
export const readProjectBoardRpc = defineRpc({
  name: "watchtower.project.read",
  input: z.object({ projectId: z.string().min(1).max(256) }),
  output: boardSchema,
});
export const searchTasksRpc = defineRpc({
  name: "watchtower.search",
  input: z.object({ query: z.string().max(512) }),
  output: PluginAttachmentSearchPayloadSchema,
});
export const taskAttachments = defineAttachmentSource({
  id: "tasks",
  title: "Watchtower task",
  icon: "ListTodo",
  pickerTitle: "Attach Watchtower task",
  searchPlaceholder: "Search recent workspaces / tasks, or paste a board search key",
  search: searchTasksRpc,
});
export function attachmentKey(workspaceId: string, taskId: string): string {
  return `workspace:${workspaceId} ${taskId}`;
}
