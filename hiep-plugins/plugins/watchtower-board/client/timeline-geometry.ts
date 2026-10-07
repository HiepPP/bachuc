import { type BarKind, barEnd, type Timeline } from "./timeline";

// Pixel layout of the branch timeline. Kept apart from the view so tests can run without React.
export const PAD_X = 4;
export const LANE_HEIGHT = 34;
export const LABEL_HEIGHT = 14;
export const BAR_HEIGHT = 10;
export const AXIS_HEIGHT = 18;
export const LINE = 2;
// The 10 px mono label font is about 6.2 px per character.
const CHAR_WIDTH = 6.2;
const LABEL_GAP = 3;

export type SegmentTone = "done" | "active" | "pending";
export interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}
export interface BarRect extends Rect {
  id: string;
  kind: BarKind;
  // Running bars only: the dashed part from now to the expected end.
  expected: Rect | null;
}
export interface LabelPosition {
  id: string;
  text: string;
  kind: BarKind;
  x: number;
  y: number;
  below: boolean;
}
export interface Segment extends Rect {
  tone: SegmentTone;
}
export interface TimelineGeometry {
  width: number;
  height: number;
  bars: BarRect[];
  labels: LabelPosition[];
  segments: Segment[];
  nowX: number | null;
  ticks: { x: number; label: string }[];
  axisY: number;
}

export function laneTop(lane: number): number {
  return lane * LANE_HEIGHT;
}

export function laneCenter(lane: number): number {
  return laneTop(lane) + LABEL_HEIGHT + BAR_HEIGHT / 2;
}

export function shortId(id: string): string {
  return id.replace(/^TASK-/, "");
}

// The clock time of a timeline minute, from the run's `Started:` value.
export function clockAt(started: string | null, minute: number): string | null {
  const match = started?.match(/\d{4}-\d{2}-\d{2}[ T](\d{2}):(\d{2})/);
  if (!match) return null;
  const total = (Number(match[1]) * 60 + Number(match[2]) + Math.round(minute)) % (24 * 60);
  return `${String(Math.floor(total / 60)).padStart(2, "0")}:${String(total % 60).padStart(2, "0")}`;
}

function horizontal(x1: number, x2: number, y: number, tone: SegmentTone): Segment {
  return { x: Math.min(x1, x2), y: y - LINE / 2, width: Math.abs(x2 - x1), height: LINE, tone };
}

// A link in one lane is one line. A link between lanes is across, then down or up, then across.
export function connector(
  x1: number,
  lane1: number,
  x2: number,
  lane2: number,
  tone: SegmentTone,
): Segment[] {
  const y1 = laneCenter(lane1);
  const y2 = laneCenter(lane2);
  if (lane1 === lane2) return [horizontal(x1, x2, y1, tone)];
  const middle = (x1 + x2) / 2;
  return [
    horizontal(x1, middle, y1, tone),
    { x: middle - LINE / 2, y: Math.min(y1, y2), width: LINE, height: Math.abs(y2 - y1), tone },
    horizontal(middle, x2, y2, tone),
  ];
}

export function timelineGeometry(
  model: Timeline,
  width: number,
  nowMinute: number | null,
): TimelineGeometry {
  const span = Math.max(model.axis.end - model.axis.start, 1);
  const usable = Math.max(width - PAD_X * 2, 1);
  const x = (minute: number) => PAD_X + ((minute - model.axis.start) / span) * usable;
  const byId = new Map(model.bars.map((bar) => [bar.id, bar]));

  const bars: BarRect[] = model.bars.map((bar) => {
    const left = x(bar.start);
    const right = Math.max(x(bar.end), left + LINE);
    const expectedRight = bar.kind === "running" ? x(barEnd(bar)) : right;
    return {
      id: bar.id,
      kind: bar.kind,
      x: left,
      y: laneTop(bar.lane) + LABEL_HEIGHT,
      width: right - left,
      height: BAR_HEIGHT,
      expected:
        expectedRight > right
          ? {
              x: right,
              y: laneTop(bar.lane) + LABEL_HEIGHT,
              width: expectedRight - right,
              height: BAR_HEIGHT,
            }
          : null,
    };
  });

  // A label that would touch the previous label in its lane moves below its bar.
  const labelEnds = new Map<number, number>();
  const labels: LabelPosition[] = [];
  for (const rect of [...bars].sort((a, b) => a.x - b.x)) {
    const lane = byId.get(rect.id)?.lane ?? 0;
    const text = shortId(rect.id);
    const below = rect.x < (labelEnds.get(lane) ?? -Infinity) + LABEL_GAP;
    labels.push({
      id: rect.id,
      text,
      kind: rect.kind,
      x: rect.x,
      y: below ? rect.y + BAR_HEIGHT + 1 : laneTop(lane),
      below,
    });
    if (!below) labelEnds.set(lane, rect.x + text.length * CHAR_WIDTH);
  }

  const segments = model.edges.flatMap((edge) => {
    const kind = byId.get(edge.from)?.kind;
    const tone: SegmentTone = kind === "done" ? "done" : kind === "running" ? "active" : "pending";
    return connector(x(edge.fromMinute), edge.fromLane, x(edge.toMinute), edge.toLane, tone);
  });

  const axisY = laneTop(Math.max(model.laneCount, 1)) + 2;
  return {
    width,
    height: axisY + AXIS_HEIGHT,
    bars,
    labels,
    segments,
    nowX: nowMinute === null ? null : x(nowMinute),
    ticks: model.axis.ticks.map((tick) => ({ x: x(tick.minute), label: tick.label })),
    axisY,
  };
}

// Every absolutely positioned View the timeline draws: bars, expected parts, labels, segments,
// ticks, the now line, and the axis line.
export function viewCount(geometry: TimelineGeometry): number {
  return (
    geometry.bars.length +
    geometry.bars.filter((bar) => bar.expected).length +
    geometry.labels.length +
    geometry.segments.length +
    geometry.ticks.length +
    (geometry.nowX === null ? 0 : 1) +
    1
  );
}
