import type { Overview } from "../shared/overview";

export const DEFAULT_TASK_MINUTES = 20;

export type BarKind = "done" | "running" | "planned" | "blocked";
export interface TimelineBar {
  id: string;
  kind: BarKind;
  start: number;
  // Done bars end at their log end; running bars end now; planned bars end at start + average.
  end: number;
  // Running bars only: start plus the average duration.
  expectedEnd: number | null;
  lane: number;
}
export interface TimelineEdge {
  from: string;
  to: string;
  fromLane: number;
  toLane: number;
  fromMinute: number;
  toMinute: number;
}
export interface TimelineTick {
  minute: number;
  label: string;
}
export interface TimelineStats {
  averageMinutes: number;
  longest: { id: string; minutes: number } | null;
  // The running bar furthest over the average, with its minutes over.
  overAverage: { id: string; minutes: number } | null;
  maxParallel: number;
}
export interface Timeline {
  bars: TimelineBar[];
  edges: TimelineEdge[];
  laneCount: number;
  axis: { start: number; end: number; ticks: TimelineTick[] };
  stats: TimelineStats;
  finishMinute: number | null;
  deadlines: { questionId: string; minute: number }[];
}

type TimelineInput = Pick<Overview, "tasks" | "run" | "questions">;

// The minute a bar stops taking room: a running bar holds its lane until its expected end.
export function barEnd(bar: TimelineBar): number {
  return Math.max(bar.end, bar.expectedEnd ?? bar.end);
}

function clockOrigin(started: string | null | undefined): number | null {
  const match = started?.match(/\d{4}-\d{2}-\d{2}[ T](\d{2}):(\d{2})/);
  return match ? Number(match[1]) * 60 + Number(match[2]) : null;
}

function hourTicks(started: string | null | undefined, end: number): TimelineTick[] {
  const origin = clockOrigin(started);
  const ticks: TimelineTick[] = [];
  const first = origin === null ? 60 : (60 - (origin % 60)) % 60;
  for (let minute = first; minute <= end; minute += 60) {
    const label =
      origin === null
        ? `+${minute / 60}h`
        : `${String(Math.floor((origin + minute) / 60) % 24).padStart(2, "0")}:00`;
    ticks.push({ minute, label });
  }
  return ticks;
}

function maxParallel(bars: readonly TimelineBar[]): number {
  const points = bars.flatMap((bar) => [
    [bar.start, 1],
    [barEnd(bar), -1],
  ]);
  // An end and a start at the same minute do not overlap, so ends sort first.
  points.sort((a, b) => a[0] - b[0] || a[1] - b[1]);
  let open = 0;
  let max = 0;
  for (const [, change] of points) {
    open += change;
    max = Math.max(max, open);
  }
  return max;
}

// Bars go into lanes by start time. A task takes its first dep's lane when that lane is free.
function packLanes(bars: TimelineBar[], deps: Map<string, string[]>): number {
  const byId = new Map(bars.map((bar) => [bar.id, bar]));
  const laneEnds: number[] = [];
  const order = [...bars].sort((a, b) => a.start - b.start);
  for (const bar of order) {
    const first = deps
      .get(bar.id)
      ?.map((id) => byId.get(id))
      .find((dep) => dep !== undefined);
    const free = (lane: number) => laneEnds[lane] <= bar.start;
    let lane =
      first && free(first.lane) ? first.lane : laneEnds.findIndex((_, index) => free(index));
    if (lane < 0) lane = laneEnds.length;
    bar.lane = lane;
    laneEnds[lane] = barEnd(bar);
  }
  return laneEnds.length;
}

export function buildTimeline({ tasks, run, questions }: TimelineInput): Timeline {
  const log = run?.log ?? [];
  const now = run?.nowMinute ?? null;
  const lastDone = new Map<string, { start: number; end: number }>();
  for (const row of log) {
    if (row.result.toUpperCase() !== "DONE" || row.startMinute === null || row.endMinute === null)
      continue;
    lastDone.set(row.task, { start: row.startMinute, end: row.endMinute });
  }
  const bars: TimelineBar[] = [];
  for (const task of tasks) {
    const span = lastDone.get(task.id);
    if (task.status === "DONE" && span)
      bars.push({ id: task.id, kind: "done", ...span, expectedEnd: null, lane: 0 });
  }
  const doneLengths = bars.map((bar) => bar.end - bar.start);
  const average = doneLengths.length
    ? Math.round(doneLengths.reduce((sum, value) => sum + value, 0) / doneLengths.length)
    : DEFAULT_TASK_MINUTES;

  // A running task starts where the latest log row ended, because the run builds one at a time.
  const lastEnd = [...log].reverse().find((row) => row.endMinute !== null)?.endMinute ?? 0;
  for (const task of tasks.filter((candidate) => candidate.status === "IN PROGRESS")) {
    const start = Math.min(lastEnd, now ?? lastEnd);
    bars.push({
      id: task.id,
      kind: "running",
      start,
      end: Math.max(start, now ?? start),
      expectedEnd: start + average,
      lane: 0,
    });
  }

  const ends = new Map(bars.map((bar) => [bar.id, barEnd(bar)]));
  let cursor = Math.max(now ?? 0, ...bars.filter((bar) => bar.kind === "running").map(barEnd));
  for (const task of tasks) {
    if (task.status !== "TODO" && task.status !== "BLOCKED") continue;
    const depEnd = Math.max(0, ...task.depIds.map((id) => ends.get(id) ?? 0));
    const start = Math.max(cursor, depEnd);
    const bar: TimelineBar = {
      id: task.id,
      kind: task.status === "BLOCKED" ? "blocked" : "planned",
      start,
      end: start + average,
      expectedEnd: null,
      lane: 0,
    };
    bars.push(bar);
    ends.set(bar.id, bar.end);
    cursor = bar.end;
  }

  const deps = new Map(tasks.map((task) => [task.id, task.depIds]));
  const laneCount = packLanes(bars, deps);
  const byId = new Map(bars.map((bar) => [bar.id, bar]));
  const edges: TimelineEdge[] = [];
  for (const bar of bars) {
    for (const depId of deps.get(bar.id) ?? []) {
      const dep = byId.get(depId);
      if (!dep) continue;
      edges.push({
        from: dep.id,
        to: bar.id,
        fromLane: dep.lane,
        toLane: bar.lane,
        fromMinute: barEnd(dep),
        toMinute: bar.start,
      });
    }
  }

  const open = bars.filter((bar) => bar.kind === "planned" || bar.kind === "blocked");
  const running = bars.filter((bar) => bar.kind === "running");
  const finishMinute = open.length
    ? Math.max(...open.map((bar) => bar.end))
    : running.length
      ? Math.max(...running.map(barEnd))
      : null;
  const deadlines: Timeline["deadlines"] = [];
  for (const question of questions) {
    if (question.status !== "OPEN" || question.blocks.length === 0) continue;
    const starts = question.blocks
      .map((id) => byId.get(id))
      .filter((bar) => bar && (bar.kind === "planned" || bar.kind === "blocked"))
      .map((bar) => bar!.start);
    if (starts.length) deadlines.push({ questionId: question.id, minute: Math.min(...starts) });
  }

  const done = bars.filter((bar) => bar.kind === "done");
  const longest = done.reduce<TimelineStats["longest"]>(
    (best, bar) =>
      !best || bar.end - bar.start > best.minutes
        ? { id: bar.id, minutes: bar.end - bar.start }
        : best,
    null,
  );
  const overAverage = running.reduce<TimelineStats["overAverage"]>((worst, bar) => {
    const over = bar.end - bar.start - average;
    return over > 0 && (!worst || over > worst.minutes) ? { id: bar.id, minutes: over } : worst;
  }, null);
  const axisEnd = Math.max(now ?? 0, 0, ...bars.map(barEnd));
  return {
    bars,
    edges,
    laneCount,
    axis: { start: 0, end: axisEnd, ticks: hourTicks(run?.started, axisEnd) },
    stats: { averageMinutes: average, longest, overAverage, maxParallel: maxParallel(bars) },
    finishMinute,
    deadlines,
  };
}
