import type { Overview, OverviewTask } from "../shared/overview";
import { taskTitle } from "./dashboard";
import { buildTimeline, startedAt, type BarKind, type TimelineBar } from "./timeline";

export interface PlanTreeRow {
  id: string;
  title: string;
  group: string | null;
  kind: BarKind;
  // `4m` done, `12m / 9m` running, `~10:24` planned; null for blocked rows and rows with no data.
  time: string | null;
  // Running rows only: the elapsed minutes pass the average.
  over: boolean;
}
export interface PlanTreeWave {
  // 1-based position in the list.
  number: number;
  header: string;
  rows: PlanTreeRow[];
  // Every row is done; the view collapses the wave.
  done: boolean;
  // The wave holds the TASKs left in a dependency cycle, or waiting on one.
  cyclic: boolean;
}
export interface PlanTreePace {
  averageMinutes: number;
  // Estimated finish as `HH:MM`; null when the run has no start clock or nothing is left.
  finishClock: string | null;
  maxParallel: number;
  // The running TASK furthest over the average, with its minutes over.
  slowest: { id: string; minutes: number } | null;
}
export interface PlanTree {
  waves: PlanTreeWave[];
  pace: PlanTreePace;
}

type PlanTreeInput = Pick<Overview, "tasks" | "run" | "questions">;

function clock(origin: number, minute: number): string {
  const total = (((origin + minute) % 1440) + 1440) % 1440;
  return `${String(Math.floor(total / 60)).padStart(2, "0")}:${String(total % 60).padStart(2, "0")}`;
}

function names(ids: readonly string[], joiner: string): string {
  return ids.length <= 1
    ? ids.join("")
    : `${ids.slice(0, -1).join(", ")} ${joiner} ${ids[ids.length - 1]}`;
}

// Wave number per TASK ID, or null for a TASK that sits in or behind a cycle.
function assignWaves(tasks: readonly OverviewTask[], inPlan: Set<string>): Map<string, number> {
  const levels = new Map<string, number>();
  const waiting = new Map<string, number>();
  const dependents = new Map<string, string[]>();
  for (const item of tasks) {
    const deps = new Set(item.depIds.filter((id) => inPlan.has(id)));
    waiting.set(item.id, deps.size);
    for (const dep of deps) dependents.set(dep, [...(dependents.get(dep) ?? []), item.id]);
  }
  let ready = tasks.filter((item) => waiting.get(item.id) === 0).map((item) => item.id);
  for (const id of ready) levels.set(id, 1);
  while (ready.length) {
    const next: string[] = [];
    for (const id of ready) {
      for (const dependent of dependents.get(id) ?? []) {
        levels.set(dependent, Math.max(levels.get(dependent) ?? 1, (levels.get(id) ?? 1) + 1));
        const left = (waiting.get(dependent) ?? 0) - 1;
        waiting.set(dependent, left);
        if (left === 0) next.push(dependent);
      }
    }
    ready = next;
  }
  for (const [id, left] of waiting) if (left > 0) levels.delete(id);
  return levels;
}

function rowFor(
  item: OverviewTask,
  bar: TimelineBar | undefined,
  average: number,
  origin: number | null,
): PlanTreeRow {
  const kind: BarKind =
    item.status === "DONE"
      ? "done"
      : item.status === "IN PROGRESS"
        ? "running"
        : item.status === "BLOCKED"
          ? "blocked"
          : "planned";
  const base = {
    id: item.id,
    title: taskTitle(item),
    group: item.group,
    kind,
    time: null,
    over: false,
  };
  if (!bar) return base;
  if (kind === "done") return { ...base, time: `${bar.end - bar.start}m` };
  if (kind === "running") {
    const elapsed = bar.end - bar.start;
    return { ...base, time: `${elapsed}m / ${average}m`, over: elapsed > average };
  }
  if (kind === "planned" && origin !== null)
    return { ...base, time: `~${clock(origin, bar.start)}` };
  return base;
}

function headerFor(
  number: number,
  rows: readonly PlanTreeRow[],
  waveTasks: readonly OverviewTask[],
  cyclic: boolean,
  doneIds: ReadonlySet<string>,
  inPlan: ReadonlySet<string>,
): string {
  if (cyclic) return "waits on a dependency cycle";
  // A finished wave is history: its deps are done too, so "ready to start" would read as pending.
  if (rows.length > 0 && rows.every((item) => item.kind === "done")) return "done";
  const running = rows.some((item) => item.kind === "running");
  if (number === 1) {
    // A TASK with no group is a builder group of its own.
    const groups = new Set(rows.map((item) => item.group ?? `\0${item.id}`)).size;
    const parallel = `${groups} ${groups === 1 ? "group" : "groups"} in parallel`;
    return running ? `running, ${parallel}` : parallel;
  }
  const pending = new Set<string>();
  for (const item of waveTasks)
    for (const dep of item.depIds) if (inPlan.has(dep) && !doneIds.has(dep)) pending.add(dep);
  if (!pending.size) return running ? "running" : "ready to start";
  return `starts when ${names([...pending], "and")} ${pending.size === 1 ? "finishes" : "finish"}`;
}

// Waves are dependency levels of the Tracker. `started` is the run's `Started:` text, which gives
// planned rows and the finish estimate a clock; without it they carry no clock time.
export function buildPlanTree(input: PlanTreeInput, started: string | null | undefined): PlanTree {
  const seen = new Set<string>();
  const tasks = input.tasks.filter((item) => !seen.has(item.id) && seen.add(item.id));
  const inPlan = new Set(tasks.map((item) => item.id));
  const timeline = buildTimeline({ ...input, tasks });
  const bars = new Map(timeline.bars.map((bar) => [bar.id, bar]));
  const origin = startedAt(started)?.minutes ?? null;
  const average = timeline.stats.averageMinutes;

  const levels = assignWaves(tasks, inPlan);
  const lastLevel = Math.max(0, ...levels.values());
  const cycleTasks = tasks.filter((item) => !levels.has(item.id));
  const buckets: { tasks: OverviewTask[]; cyclic: boolean }[] = [];
  for (let level = 1; level <= lastLevel; level++) buckets.push({ tasks: [], cyclic: false });
  // Tracker order inside a wave comes from walking the tasks in order.
  for (const item of tasks) {
    const level = levels.get(item.id);
    if (level !== undefined) buckets[level - 1].tasks.push(item);
  }
  if (cycleTasks.length) buckets.push({ tasks: cycleTasks, cyclic: true });

  const doneIds = new Set(tasks.filter((item) => item.status === "DONE").map((item) => item.id));
  const waves = buckets.map((bucket, index): PlanTreeWave => {
    const rows = bucket.tasks.map((item) => rowFor(item, bars.get(item.id), average, origin));
    return {
      number: index + 1,
      header: headerFor(index + 1, rows, bucket.tasks, bucket.cyclic, doneIds, inPlan),
      rows,
      done: rows.every((item) => item.kind === "done"),
      cyclic: bucket.cyclic,
    };
  });

  return {
    waves,
    pace: {
      averageMinutes: average,
      finishClock:
        origin !== null && timeline.finishMinute !== null
          ? clock(origin, timeline.finishMinute)
          : null,
      maxParallel: timeline.stats.maxParallel,
      slowest: timeline.stats.overAverage,
    },
  };
}
