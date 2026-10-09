import { defineRpc } from "@getpaseo/plugin";
import { z } from "zod";

export const runSchema = z.object({
  id: z.string(),
  agentId: z.string(),
  parentAgentId: z.string().nullable().optional(),
  title: z.string(),
  starred: z.boolean(),
  needsInput: z.boolean().optional(),
  // Host read state of a finished run; absent from older hosts. Never persisted.
  unread: z.boolean().optional(),
  // The thread's edit-diffs.mode label is off. Read from the host; never persisted.
  diffsOff: z.boolean().optional(),
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
// Releases the processes of a removed thread and all its subagents. Runs on the thread's host.
export const closeRuntimeRpc = defineRpc({
  name: "board.close-runtime",
  input: z.object({ agentId: z.string().min(1) }),
  output: z.object({ closed: z.number(), kept: z.number(), failed: z.number() }),
});
// The edit-diffs plugin reads this agent label and leaves that thread's edits as Paseo's own rows.
export const EDIT_DIFFS_MODE_LABEL = "edit-diffs.mode";
// Turns edit diffs on or off for one thread. Runs on the thread's host.
export const editDiffsModeRpc = defineRpc({
  name: "board.edit-diffs.set",
  input: z.object({ agentId: z.string().min(1), off: z.boolean() }),
  output: z.object({}),
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

export const ACTIVE_BOARD_POLL_MS = 2_000;
export const IDLE_BOARD_POLL_MS = 15_000;

// Running cards show live elapsed time and change state soon; finished cards rarely change.
export function boardPollInterval(
  snapshot: { runs: readonly Pick<BoardRun, "status">[] } | undefined,
) {
  return snapshot?.runs.some((run) => run.status === "running")
    ? ACTIVE_BOARD_POLL_MS
    : IDLE_BOARD_POLL_MS;
}

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

export const boardHostRpc = defineRpc({
  name: "board.host",
  input: z.object({}),
  output: z.object({ serverId: z.string() }),
});
