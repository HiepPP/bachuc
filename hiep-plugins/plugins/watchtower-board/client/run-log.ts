// The run log row for a task that is IN PROGRESS. In a stopped run nothing is building it, so it
// reads "stalled" with no minutes instead of "running 0m".
export interface RunningLogLine {
  cells: readonly [string, string, string, string];
  tail: string;
  tone: "active" | "warning";
}

export function runningLogLine(
  task: { id: string; minutes: number },
  stopped: boolean,
): RunningLogLine {
  const id = task.id.replace(/^TASK-/, "");
  return stopped
    ? { cells: ["", "-", id, "stalled"], tail: "", tone: "warning" }
    : { cells: ["", "now", id, "running"], tail: `${task.minutes}m`, tone: "active" };
}
