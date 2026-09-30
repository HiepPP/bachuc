import {
  defineAttachmentSource,
  defineRpc,
  PluginAttachmentSearchPayloadSchema,
} from "@getpaseo/plugin";
import { z } from "zod";

export const threadSchema = z.object({
  id: z.string(),
  title: z.string(),
  provider: z.string(),
  cwd: z.string(),
  workspaceId: z.string().nullable(),
  status: z.string(),
  lastActivityAt: z.string(),
});
export type Thread = z.infer<typeof threadSchema>;

export const listThreadsRpc = defineRpc({
  name: "thread-context.list-threads",
  input: z.object({
    currentAgentId: z.string().min(1).max(256).optional(),
    currentWorkspaceId: z.string().min(1).max(256).optional(),
    query: z.string().max(512).optional(),
    limit: z.number().int().positive().max(200).optional(),
  }),
  output: z.object({ threads: z.array(threadSchema) }),
});

export const threadSnapshotSchema = z.object({
  agentId: z.string(),
  title: z.string(),
  provider: z.string(),
  cwd: z.string(),
  hasReply: z.boolean(),
  truncated: z.boolean(),
  text: z.string(),
});
export type ThreadSnapshot = z.infer<typeof threadSnapshotSchema>;

export const getThreadSnapshotRpc = defineRpc({
  name: "thread-context.get-thread-snapshot",
  input: z.object({ agentId: z.string().min(1).max(256) }),
  output: threadSnapshotSchema,
});

export const searchThreadsRpc = defineRpc({
  name: "thread-context.search",
  input: z.object({ query: z.string().max(512) }),
  output: PluginAttachmentSearchPayloadSchema,
});

export const threadAttachments = defineAttachmentSource({
  id: "thread",
  title: "Thread",
  icon: "MessagesSquare",
  pickerTitle: "Attach another thread's last reply",
  searchPlaceholder: "Search threads by title, provider, or folder",
  search: searchThreadsRpc,
});
