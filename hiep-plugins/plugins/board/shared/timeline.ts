import { defineRpc } from "@getpaseo/plugin";
import { z } from "zod";
import { runSchema } from "./board";

export const TIMELINE_WINDOW_MS = 24 * 60 * 60 * 1000;
const MINUTE = 60_000;
const HOUR = 60 * MINUTE;

export const timelineThreadSchema = z.object({
  agentId: z.string(),
  parentAgentId: z.string().nullable().optional(),
  title: z.string(),
  project: z.string(),
  projectKey: z.string(),
  projectId: z.string().optional(),
  status: runSchema.shape.status,
  startedAt: z.string(),
  endedAt: z.string().nullable(),
  needsInput: z.boolean().optional(),
  taskId: z.string().optional(),
  workspaceId: z.string().nullable().optional(),
});
export type TimelineThread = z.infer<typeof timelineThreadSchema>;

export const timelineRpc = defineRpc({
  name: "board.timeline",
  input: z.object({}),
  output: z.object({
    threads: z.array(timelineThreadSchema),
    // Runs in the window without a start time. They are left out of the chart.
    missingStart: z.number().int().min(0),
    // The store dropped finished runs that may still fall in the window (Q-024).
    olderDropped: z.boolean(),
  }),
});

// Same rule as the watchtower-board timeline: an idle gap longer than max(2 h, 4 x the average
// thread) draws as a fixed-width break. Plugins never import each other, so the rule is copied.
export const BREAK_WIDTH = 48;
export const MIN_BREAK_MINUTES = 120;
const PAD_X = 4;
const MIN_BAR_WIDTH = 4;
// A tick label is about 36 px wide. Keep every nth hour so neighbouring labels never touch.
const MIN_TICK_GAP = 44;

export type TimelineKind = "running" | "needs" | "done" | "failed" | "cancelled";
export interface TimelineBar {
  agentId: string;
  title: string;
  kind: TimelineKind;
  taskId?: string;
  workspaceId?: string | null;
  start: number;
  end: number;
  x: number;
  width: number;
}
export interface TimelineRow {
  agentId: string;
  project: string;
  projectKey: string;
  projectId?: string;
  bar: TimelineBar;
  // A thread's subagents share one row under it.
  subagents: TimelineBar[];
}
export interface TimelineHostInput {
  hostId: string;
  threads: readonly TimelineThread[];
}
export interface TimelineModel {
  windowStart: number;
  now: number;
  width: number;
  nowX: number;
  rows: Map<string, TimelineRow[]>;
  breaks: { x: number; width: number; minutes: number; label: string }[];
  ticks: { x: number; label: string }[];
  // Top-level threads running at once, per clock hour.
  load: { x: number; width: number; count: number; running: boolean }[];
  peak: { count: number; label: string } | null;
  threads: number;
  agentMinutes: number;
  idleMinutes: number;
}

export function durationText(milliseconds: number): string {
  const seconds = Math.max(0, Math.round(milliseconds / 1000));
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return `${minutes}m`;
  const hours = Math.floor(minutes / 60);
  return minutes % 60 ? `${hours}h ${minutes % 60}m` : `${hours}h`;
}

export function clockText(time: number): string {
  const date = new Date(time);
  return `${String(date.getHours()).padStart(2, "0")}:${String(date.getMinutes()).padStart(2, "0")}`;
}

/** Notes under a host's header once its timeline has loaded. */
export function timelineNotes(
  result: { missingStart: number; olderDropped: boolean },
  threads: number,
): string[] {
  const notes: string[] = [];
  if (!threads) notes.push("Nothing ran in the last 24 hours.");
  if (result.olderDropped) notes.push("Older threads are not kept.");
  if (result.missingStart === 1) notes.push("1 thread without a start time is left out.");
  else if (result.missingStart)
    notes.push(`${result.missingStart} threads without a start time are left out.`);
  return notes;
}

export function timelineKind(thread: Pick<TimelineThread, "status" | "needsInput">): TimelineKind {
  if (thread.status === "running") return thread.needsInput ? "needs" : "running";
  if (thread.status === "failed") return "failed";
  if (thread.status === "cancelled") return "cancelled";
  return "done";
}

function span(thread: TimelineThread, windowStart: number, now: number) {
  const started = Date.parse(thread.startedAt);
  const start = Math.min(Math.max(started, windowStart), now);
  const ended = thread.status === "running" ? now : Date.parse(thread.endedAt ?? "");
  const end = Number.isFinite(ended) ? Math.min(Math.max(ended, start), now) : start;
  return { start, end };
}

// The top-level thread each thread belongs to: its oldest ancestor on the same host.
function roots(threads: readonly TimelineThread[]) {
  const byId = new Map(threads.map((thread) => [thread.agentId, thread]));
  const root = new Map<string, string>();
  for (const thread of threads) {
    let current = thread;
    const seen = new Set([current.agentId]);
    while (current.parentAgentId) {
      const parent = byId.get(current.parentAgentId);
      if (!parent || seen.has(parent.agentId)) break;
      seen.add(parent.agentId);
      current = parent;
    }
    root.set(thread.agentId, current.agentId);
  }
  return root;
}

function idleGaps(
  spans: readonly { start: number; end: number }[],
  windowStart: number,
  now: number,
) {
  // An empty window has nothing to fold toward.
  if (!spans.length) return [];
  const average = spans.reduce((sum, item) => sum + item.end - item.start, 0) / spans.length;
  const threshold = Math.max(MIN_BREAK_MINUTES * MINUTE, 4 * average);
  // The window start and now count as busy points, so a long quiet stretch at either end folds.
  const points = [
    { start: windowStart, end: windowStart },
    ...spans,
    { start: now, end: now },
  ].sort((a, b) => a.start - b.start);
  const gaps: { start: number; end: number }[] = [];
  let busyEnd = points[0].end;
  for (const point of points.slice(1)) {
    if (point.start - busyEnd > threshold) gaps.push({ start: busyEnd, end: point.start });
    busyEnd = Math.max(busyEnd, point.end);
  }
  return gaps;
}

// Most intervals overlapping at one moment inside [from, to).
function concurrency(spans: readonly { start: number; end: number }[], from: number, to: number) {
  const inside = spans.filter((item) => item.start < to && item.end >= from);
  if (!inside.length) return 0;
  const events = inside.flatMap((item) => [
    { time: Math.max(item.start, from), delta: 1 },
    { time: Math.min(item.end, to), delta: -1 },
  ]);
  // An end and a start at the same moment do not overlap.
  events.sort((a, b) => a.time - b.time || a.delta - b.delta);
  let count = 0;
  let peak = 1;
  for (const event of events) {
    count += event.delta;
    peak = Math.max(peak, count);
  }
  return peak;
}

export function timelineModel(
  hosts: readonly TimelineHostInput[],
  now: number,
  width: number,
): TimelineModel {
  const windowStart = now - TIMELINE_WINDOW_MS;
  const top: { start: number; end: number; running: boolean }[] = [];
  const grouped = hosts.map(({ hostId, threads }) => {
    const root = roots(threads);
    const rows = new Map<string, { thread: TimelineThread; children: TimelineThread[] }>();
    for (const thread of threads) {
      if (root.get(thread.agentId) === thread.agentId)
        rows.set(thread.agentId, { thread, children: [] });
    }
    for (const thread of threads) {
      const owner = root.get(thread.agentId)!;
      if (owner !== thread.agentId) rows.get(owner)?.children.push(thread);
    }
    for (const { thread } of rows.values()) {
      top.push({ ...span(thread, windowStart, now), running: thread.status === "running" });
    }
    return { hostId, rows: [...rows.values()] };
  });

  const gaps = idleGaps(top, windowStart, now);
  const hidden = gaps.reduce((sum, gap) => sum + gap.end - gap.start, 0);
  const scaled = Math.max(now - windowStart - hidden, 1);
  const usable = Math.max(width - PAD_X * 2 - gaps.length * BREAK_WIDTH, 1);
  // Time before a gap scales normally; a gap takes BREAK_WIDTH whatever its length.
  const x = (time: number) => {
    let offset = PAD_X;
    let from = windowStart;
    for (const gap of gaps) {
      if (time <= gap.start) break;
      offset += ((gap.start - from) / scaled) * usable;
      if (time < gap.end)
        return offset + ((time - gap.start) / (gap.end - gap.start)) * BREAK_WIDTH;
      offset += BREAK_WIDTH;
      from = gap.end;
    }
    return offset + ((time - from) / scaled) * usable;
  };
  const inGap = (time: number) => gaps.find((gap) => gap.start < time && time < gap.end);

  const bar = (thread: TimelineThread): TimelineBar => {
    const { start, end } = span(thread, windowStart, now);
    const left = x(start);
    return {
      agentId: thread.agentId,
      title: thread.title,
      kind: timelineKind(thread),
      taskId: thread.taskId,
      workspaceId: thread.workspaceId,
      start,
      end,
      x: left,
      width: Math.max(x(end) - left, MIN_BAR_WIDTH),
    };
  };
  const rows = new Map<string, TimelineRow[]>();
  for (const host of grouped) {
    const built = host.rows.map(({ thread, children }) => ({
      agentId: thread.agentId,
      project: thread.project,
      projectKey: thread.projectKey,
      projectId: thread.projectId,
      bar: bar(thread),
      subagents: children.map(bar).sort((a, b) => a.start - b.start),
    }));
    // Projects in order of their first thread, threads by start inside a project.
    const firstStart = new Map<string, number>();
    for (const row of built) {
      firstStart.set(
        row.projectKey,
        Math.min(firstStart.get(row.projectKey) ?? Infinity, row.bar.start),
      );
    }
    built.sort(
      (a, b) =>
        firstStart.get(a.projectKey)! - firstStart.get(b.projectKey)! ||
        a.projectKey.localeCompare(b.projectKey) ||
        a.bar.start - b.bar.start,
    );
    rows.set(host.hostId, built);
  }

  const firstHour = Math.floor(windowStart / HOUR) * HOUR;
  const hourStarts: number[] = [];
  for (let hour = firstHour; hour < now; hour += HOUR) hourStarts.push(hour);

  const ticks: TimelineModel["ticks"] = [];
  for (const hour of hourStarts) {
    if (hour < windowStart || inGap(hour)) continue;
    const tickX = x(hour);
    if (ticks.length && tickX - ticks.at(-1)!.x < MIN_TICK_GAP) continue;
    ticks.push({ x: tickX, label: clockText(hour) });
  }

  const load: TimelineModel["load"] = [];
  const counts: number[] = [];
  for (const hour of hourStarts) {
    const from = Math.max(hour, windowStart);
    const to = Math.min(hour + HOUR, now);
    const count = concurrency(top, from, to);
    counts.push(count);
    const startGap = inGap(from);
    const endGap = inGap(to);
    const left = x(startGap ? startGap.end : from);
    const right = x(endGap ? endGap.start : to);
    if (!count || right <= left) continue;
    load.push({
      x: left,
      width: right - left,
      count,
      running: top.some((item) => item.running && item.start < to && item.end >= from),
    });
  }
  const most = Math.max(0, ...counts);
  let peak: TimelineModel["peak"] = null;
  if (most > 0) {
    const first = counts.indexOf(most);
    let last = first;
    while (counts[last + 1] === most) last++;
    const to = Math.min(hourStarts[last] + HOUR, now);
    peak = { count: most, label: `${clockText(hourStarts[first])}–${clockText(to)}` };
  }

  return {
    windowStart,
    now,
    width,
    nowX: x(now),
    rows,
    breaks: gaps.map((gap) => ({
      x: x(gap.start),
      width: BREAK_WIDTH,
      minutes: Math.round((gap.end - gap.start) / MINUTE),
      label: durationText(gap.end - gap.start),
    })),
    ticks,
    load,
    peak,
    threads: top.length,
    agentMinutes: Math.round(top.reduce((sum, item) => sum + item.end - item.start, 0) / MINUTE),
    idleMinutes: Math.round(hidden / MINUTE),
  };
}
