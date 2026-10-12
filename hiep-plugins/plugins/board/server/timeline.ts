import type { BoardRun } from "../shared/board";
import { TIMELINE_WINDOW_MS, type TimelineThread } from "../shared/timeline";

/**
 * Runs that overlap the last 24 hours. `droppedEndedAt` is the end time of the oldest kept
 * finished run when the store is full, so older runs in the window may be gone (Q-024).
 */
export function timelineSnapshot(
  runs: readonly BoardRun[],
  droppedEndedAt: string | null,
  now = Date.now(),
) {
  const windowStart = now - TIMELINE_WINDOW_MS;
  const threads: TimelineThread[] = [];
  let missingStart = 0;
  for (const run of runs) {
    const ended = run.status === "running" ? now : Date.parse(run.endedAt ?? "");
    if (Number.isFinite(ended) && ended < windowStart) continue;
    if (!run.startedAt || !Number.isFinite(Date.parse(run.startedAt))) {
      missingStart++;
      continue;
    }
    if (Date.parse(run.startedAt) > now) continue;
    threads.push({
      agentId: run.agentId,
      parentAgentId: run.parentAgentId,
      title: run.title,
      project: run.project,
      projectKey: run.projectKey,
      projectId: run.projectId,
      status: run.status,
      startedAt: run.startedAt,
      endedAt: run.endedAt,
      needsInput: run.needsInput,
      taskId: run.taskId,
      workspaceId: run.workspaceId,
    });
  }
  const dropped = droppedEndedAt === null ? NaN : Date.parse(droppedEndedAt);
  return {
    threads,
    missingStart,
    olderDropped: Number.isFinite(dropped) && dropped > windowStart,
  };
}
