import assert from "node:assert/strict";
import { test } from "node:test";
import {
  BAR_HEIGHT,
  connector,
  laneCenter,
  PAD_X,
  parallelCaption,
  timelineGeometry,
  viewCount,
} from "../client/timeline-geometry";
import { buildTimeline, type Timeline } from "../client/timeline";
import type { OverviewRunEntry, OverviewTask } from "../shared/overview";

function task(id: string, status: string, depIds: string[] = []): OverviewTask {
  return {
    id,
    title: id,
    status,
    deps: depIds.join(", ") || "-",
    notes: "",
    spec: "",
    taskClass: null,
    blocker: null,
    error: null,
    depIds,
    group: null,
  };
}

function row(taskId: string, start: number, end: number): OverviewRunEntry {
  return {
    task: taskId,
    result: "DONE",
    detail: "",
    start: "",
    end: "",
    startMinute: start,
    endMinute: end,
  };
}

// The mid-run state of the t3code plan used in the design mock.
const mock = buildTimeline({
  tasks: [
    task("TASK-001", "DONE"),
    task("TASK-002", "DONE", ["TASK-001"]),
    task("TASK-003", "DONE", ["TASK-001"]),
    task("TASK-004", "DONE", ["TASK-003"]),
    task("TASK-005", "DONE"),
    task("TASK-006", "IN PROGRESS"),
    task("TASK-010", "TODO", ["TASK-006"]),
    task("TASK-007", "TODO", ["TASK-006", "TASK-010"]),
    task("TASK-008", "IN PROGRESS"),
    task("TASK-009", "BLOCKED", ["TASK-008"]),
  ],
  run: {
    runner: "loop",
    started: "2026-10-06 22:52",
    finished: null,
    log: [
      row("TASK-001", 0, 14),
      row("TASK-002", 17, 36),
      row("TASK-003", 39, 67),
      row("TASK-004", 70, 89),
      row("TASK-005", 92, 100),
    ],
    nowMinute: 140,
    stopped: false,
  },
  questions: [],
});

test("a fork between lanes is 3 segments and a same-lane link is 1", () => {
  const fork = connector(10, 0, 50, 1, "done");
  assert.equal(fork.length, 3);
  assert.deepEqual(fork[1], {
    x: 29,
    y: laneCenter(0),
    width: 2,
    height: laneCenter(1) - laneCenter(0),
    tone: "done",
  });
  assert.equal(fork[2].x + fork[2].width, 50);
  assert.equal(connector(10, 1, 50, 1, "active").length, 1);
  assert.equal(connector(10, 1, 50, 0, "pending")[1].y, laneCenter(0));
});

test("a label that would touch the previous one in its lane moves below the bar", () => {
  const model: Timeline = {
    ...mock,
    bars: [
      { id: "TASK-001", kind: "done", start: 0, end: 5, expectedEnd: null, lane: 0 },
      { id: "TASK-002", kind: "done", start: 10, end: 20, expectedEnd: null, lane: 0 },
      { id: "TASK-003", kind: "done", start: 80, end: 90, expectedEnd: null, lane: 0 },
    ],
    edges: [],
    laneCount: 1,
    axis: { start: 0, end: 100, ticks: [] },
  };
  // 100 minutes over 100 usable px puts the first two labels 10 px apart.
  const geometry = timelineGeometry(model, 100 + PAD_X * 2, null);
  const labels = Object.fromEntries(geometry.labels.map((label) => [label.id, label]));
  assert.equal(labels["TASK-002"].x - labels["TASK-001"].x, 10);
  assert.equal(labels["TASK-001"].below, false);
  assert.equal(labels["TASK-002"].below, true);
  assert.equal(labels["TASK-002"].y, geometry.bars[1].y + BAR_HEIGHT + 1);
  assert.equal(labels["TASK-003"].below, false);
  assert.equal(labels["TASK-003"].text, "003");
});

test("a label that would cross the now line moves off it", () => {
  const model: Timeline = {
    ...mock,
    bars: [
      { id: "TASK-001", kind: "done", start: 0, end: 50, expectedEnd: null, lane: 0 },
      { id: "TASK-010", kind: "planned", start: 49, end: 60, expectedEnd: null, lane: 1 },
      { id: "TASK-011", kind: "planned", start: 98, end: 100, expectedEnd: null, lane: 0 },
    ],
    edges: [],
    laneCount: 2,
    axis: { start: 0, end: 100, ticks: [] },
  };
  // One minute is one px, so now at minute 50 sits at x 54, inside the "010" label at x 53.
  const geometry = timelineGeometry(model, 100 + PAD_X * 2, 50);
  const labels = Object.fromEntries(geometry.labels.map((label) => [label.id, label]));
  assert.equal(geometry.nowX, 54);
  assert.equal(labels["TASK-010"].x, 60);
  assert.equal(labels["TASK-001"].x, PAD_X);
  // Near the right edge there is no room after the line, so the label moves before it.
  const late = timelineGeometry(model, 100 + PAD_X * 2, 98);
  const end = Object.fromEntries(late.labels.map((label) => [label.id, label]));
  assert.ok(end["TASK-011"].x + 3 * 6.2 <= late.nowX! - 6 + 0.001);
});

test("the stat caption is singular for one branch", () => {
  assert.equal(parallelCaption(1), "branch at once");
  assert.equal(parallelCaption(2), "branches at once");
  assert.equal(parallelCaption(0), "branches at once");
});

test("the now line sits at the x of now", () => {
  const width = 400;
  const geometry = timelineGeometry(mock, width, 140);
  const expected = PAD_X + (140 / mock.axis.end) * (width - PAD_X * 2);
  assert.equal(geometry.nowX, expected);
  assert.equal(timelineGeometry(mock, width, null).nowX, null);
});

test("running bars get a dashed part only past now", () => {
  const geometry = timelineGeometry(mock, 400, 140);
  const running = geometry.bars.filter((bar) => bar.kind === "running");
  assert.equal(running.length, 2);
  // Both running tasks are already past the 18-minute average, so nothing is left to draw.
  assert.ok(running.every((bar) => bar.expected === null));
  assert.equal(geometry.ticks.length, 4);
  assert.equal(geometry.height, geometry.axisY + 18);
});

test("the timeline stays light on the mock data", (t) => {
  const geometry = timelineGeometry(mock, 640, 140);
  const perEdge = mock.edges.map(
    (edge) => connector(edge.fromMinute, edge.fromLane, edge.toMinute, edge.toLane, "done").length,
  );
  assert.ok(perEdge.every((count) => count <= 3));
  assert.equal(
    geometry.segments.length,
    perEdge.reduce((sum, value) => sum + value, 0),
  );
  const count = viewCount(geometry);
  const limit = 4 * mock.bars.length + 24;
  t.diagnostic(`positioned views: ${count}, limit ${limit}`);
  assert.ok(count <= limit, `${count} views`);
});
