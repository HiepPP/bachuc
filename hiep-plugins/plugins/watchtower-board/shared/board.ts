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
  brief: z.string().nullable(),
  blocker: z.string().nullable(),
  error: z.string().nullable(),
});
export const boardSchema = z.object({
  title: z.string(),
  tasks: z.array(taskSchema),
  message: z.string().nullable(),
});
export type Board = z.infer<typeof boardSchema>;
export type Task = z.infer<typeof taskSchema>;
export const readBoardRpc = defineRpc({
  name: "watchtower.read",
  input: z.object({ workspaceId: z.string().min(1).max(256) }),
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
