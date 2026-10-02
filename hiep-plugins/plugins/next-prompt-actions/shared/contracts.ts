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
  // Timestamp of the user message the reply answers. A reply watched while it streamed keeps the
  // live chunk timestamp, which differs from the stored row timestamp, so identity uses this bound.
  after: z.number().optional(),
  state: z.enum(["ready", "sending", "sent", "unknown"]),
  thread: z.literal("new").optional(),
  // Display only: the agent recommends this prompt. Set only when true.
  suggestion: z.boolean().optional(),
  goal: z.literal("done").optional(),
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
// Timeline rows know only the agent; the daemon resolves its workspace.
export const scopeRpc = defineRpc({
  name: "prompts.scope",
  input: z.object({ agentId: z.string().min(1) }),
  output: scopeSchema,
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
export const startRpc = defineRpc({
  name: "prompts.start",
  input: scopeSchema.extend({ key: z.string() }),
  output: z.object({ started: z.boolean() }),
});
export const toggleRpc = defineRpc({
  name: "prompts.toggle",
  input: scopeSchema.extend({ enabled: z.boolean() }),
  output: snapshotSchema,
});
