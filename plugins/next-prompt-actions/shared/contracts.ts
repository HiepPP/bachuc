import { defineRpc } from "@getpaseo/plugin";
import { z } from "zod";
import { selectionSchema } from "./next-prompts";

export const scopeSchema = z.object({
  serverId: z.string(),
  agentId: z.string().min(1),
  workspaceId: z.string().min(1),
});
export type Scope = z.infer<typeof scopeSchema>;
export const candidateSchema = z.object({
  key: z.string(),
  block: z.string(),
  text: z.string(),
  why: z.string().optional(),
  source: z.string(),
  timestamp: z.number(),
  state: z.enum(["ready", "sending", "sent", "unknown"]),
  selection: selectionSchema.optional(),
});
export const snapshotSchema = z.object({
  enabled: z.boolean(),
  busy: z.boolean(),
  note: z.string(),
  warning: z.string().optional(),
  candidates: z.array(candidateSchema),
});
export type Snapshot = z.infer<typeof snapshotSchema>;
export type Candidate = z.infer<typeof candidateSchema>;
export const hostRpc = defineRpc({
  name: "prompts.host",
  input: z.object({}),
  output: z.object({ serverId: z.string() }),
});
export const inspectRpc = defineRpc({
  name: "prompts.inspect",
  input: scopeSchema,
  output: snapshotSchema,
});
export const sendRpc = defineRpc({
  name: "prompts.send",
  input: scopeSchema.extend({
    key: z.union([z.string(), z.array(z.string()).min(1)]),
  }),
  // Only the outcome: the client rereads state after every action, and skipping that read here
  // lets a sent prompt return to the Board one timeline read sooner.
  output: z.object({ sent: z.boolean() }),
});
export const toggleRpc = defineRpc({
  name: "prompts.toggle",
  input: scopeSchema.extend({ enabled: z.boolean() }),
  output: snapshotSchema,
});
