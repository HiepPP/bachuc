import { defineRpc } from "@getpaseo/plugin";
import { z } from "zod";

export const runSchema = z.object({
  id: z.string(),
  agentId: z.string(),
  parentAgentId: z.string().nullable().optional(),
  title: z.string(),
  starred: z.boolean(),
  needsInput: z.boolean().optional(),
  project: z.string(),
  projectKey: z.string(),
  projectId: z.string().optional(),
  cwd: z.string().optional(),
  provider: z.string(),
  // Raw ids in the store; the snapshot RPC replaces them with provider labels.
  model: z.string().nullable().optional(),
  effort: z.string().nullable().optional(),
  status: z.enum(["running", "completed", "failed", "cancelled", "unknown"]),
  startedAt: z.string().nullable(),
  endedAt: z.string().nullable(),
});
export type BoardRun = z.infer<typeof runSchema>;
export const removeRunRpc = defineRpc({
  name: "board.remove-finished",
  input: z.object({ id: z.string(), observingSince: z.string(), endedAt: z.string().nullable() }),
  output: z.object({ removed: z.boolean() }),
});
export const boardRpc = defineRpc({
  name: "board.snapshot",
  input: z.object({}),
  output: z.object({
    runs: z.array(runSchema),
    observingSince: z.string(),
  }),
});

export const starRunRpc = defineRpc({
  name: "board.set-starred",
  input: z.object({ id: z.string(), observingSince: z.string(), starred: z.boolean() }),
  output: z.object({ updated: z.boolean() }),
});

export function starredFirst(left: Pick<BoardRun, "starred">, right: Pick<BoardRun, "starred">) {
  return Number(right.starred) - Number(left.starred);
}

export function groupRuns(runs: readonly BoardRun[]) {
  const projects = new Map<string, { key: string; name: string; runs: BoardRun[] }>();
  for (const run of runs) {
    let group = projects.get(run.projectKey);
    if (!group) {
      group = { key: run.projectKey, name: run.project, runs: [] };
      projects.set(run.projectKey, group);
    }
    group.runs.push(run);
  }
  for (const group of projects.values()) group.runs.sort(starredFirst);
  return { projects: [...projects.values()] };
}
