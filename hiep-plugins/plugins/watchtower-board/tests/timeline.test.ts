import assert from "node:assert/strict";
import { test } from "node:test";
import { performance } from "node:perf_hooks";
import { buildTimeline } from "../client/timeline";
import type { Overview, OverviewRunEntry, OverviewTask } from "../shared/overview";

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

function row(taskId: string, start: number, end: number | null, result = "DONE"): OverviewRunEntry {
  return {
    task: taskId,
    result,
    detail: "",
    start: "",
    end: "",
    startMinute: start,
    endMinute: end,
  };
}

type Input = Pick<Overview, "tasks" | "run" | "questions">;

// The mid-run state of the t3code plan used in the design mock.
function midRun(): Input {
  return {
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
    questions: [
      {
        id: "Q-008",
        tasks: ["TASK-008"],
        blocks: ["TASK-009"],
        question: "Network access?",
        default: "No network",
        status: "OPEN",
      },
    ],
  };
}

test("a sequential done log packs into lane 0 and running tasks fork", () => {
  const timeline = buildTimeline(midRun());
  const bars = Object.fromEntries(timeline.bars.map((bar) => [bar.id, bar]));
  for (const id of ["TASK-001", "TASK-002", "TASK-003", "TASK-004", "TASK-005"]) {
    assert.equal(bars[id].kind, "done");
    assert.equal(bars[id].lane, 0);
  }
  assert.equal(timeline.stats.averageMinutes, 18);
  assert.deepEqual(
    ["TASK-006", "TASK-008"].map((id) => [
      bars[id].kind,
      bars[id].start,
      bars[id].end,
      bars[id].expectedEnd,
      bars[id].lane,
    ]),
    [
      ["running", 100, 140, 118, 0],
      ["running", 100, 140, 118, 1],
    ],
  );
  assert.equal(timeline.laneCount, 2);
  assert.equal(timeline.stats.maxParallel, 2);
  assert.deepEqual(timeline.stats.overAverage, { id: "TASK-006", minutes: 22 });
  assert.deepEqual(timeline.stats.longest, { id: "TASK-003", minutes: 28 });
});

test("planned tasks run one at a time after their deps", () => {
  const timeline = buildTimeline(midRun());
  const bars = Object.fromEntries(timeline.bars.map((bar) => [bar.id, bar]));
  assert.deepEqual(
    ["TASK-010", "TASK-007", "TASK-009"].map((id) => [
      bars[id].kind,
      bars[id].start,
      bars[id].end,
      bars[id].lane,
    ]),
    [
      ["planned", 140, 158, 0],
      ["planned", 158, 176, 0],
      ["blocked", 176, 194, 1],
    ],
  );
  // TASK-007 has two deps and starts after the later one ends.
  assert.ok(bars["TASK-007"].start >= bars["TASK-010"].end);
  assert.equal(timeline.finishMinute, 194);
  assert.deepEqual(timeline.deadlines, [{ questionId: "Q-008", minute: 176 }]);
  const edge = timeline.edges.find((candidate) => candidate.to === "TASK-009");
  assert.deepEqual(edge, {
    from: "TASK-008",
    to: "TASK-009",
    fromLane: 1,
    toLane: 1,
    fromMinute: 140,
    toMinute: 176,
  });
  assert.equal(timeline.edges.filter((candidate) => candidate.to === "TASK-007").length, 2);
});

test("the axis ticks fall on whole clock hours", () => {
  const timeline = buildTimeline(midRun());
  assert.equal(timeline.axis.end, 194);
  assert.deepEqual(timeline.axis.ticks, [
    { minute: 8, label: "23:00" },
    { minute: 68, label: "00:00" },
    { minute: 128, label: "01:00" },
    { minute: 188, label: "02:00" },
  ]);
});

test("a dep lane that is busy sends the task to another lane", () => {
  const input = midRun();
  input.tasks = [task("TASK-001", "IN PROGRESS"), task("TASK-002", "IN PROGRESS", ["TASK-001"])];
  input.run!.log = [];
  input.questions = [];
  const timeline = buildTimeline(input);
  assert.deepEqual(
    timeline.bars.map((bar) => [bar.id, bar.start, bar.lane]),
    [
      ["TASK-001", 0, 0],
      ["TASK-002", 0, 1],
    ],
  );
  assert.equal(timeline.stats.averageMinutes, 20);
  assert.deepEqual(
    timeline.edges.map((edge) => [edge.fromLane, edge.toLane]),
    [[0, 1]],
  );
});

test("no done bars gives the 20-minute default and no run gives relative ticks", () => {
  const timeline = buildTimeline({
    tasks: [task("TASK-001", "TODO"), task("TASK-002", "TODO", ["TASK-001"])],
    run: null,
    questions: [],
  });
  assert.equal(timeline.stats.averageMinutes, 20);
  assert.deepEqual(
    timeline.bars.map((bar) => [bar.id, bar.start, bar.end]),
    [
      ["TASK-001", 0, 20],
      ["TASK-002", 20, 40],
    ],
  );
  assert.equal(timeline.finishMinute, 40);
  assert.deepEqual(timeline.axis.ticks, []);
  assert.equal(buildTimeline({ ...midRun(), run: null }).axis.ticks[0]?.label, "+1h");
});

test("every task done gives no finish, and a done task without a log row has no bar", () => {
  const input = midRun();
  input.tasks = input.tasks.map((candidate) => ({ ...candidate, status: "DONE" }));
  const timeline = buildTimeline(input);
  assert.equal(timeline.finishMinute, null);
  assert.equal(timeline.bars.length, 5);
  assert.ok(!timeline.bars.some((bar) => bar.id === "TASK-006"));
  assert.deepEqual(timeline.deadlines, []);
});

test("the model stays cheap on 200 tasks and 200 log rows", (t) => {
  const tasks: OverviewTask[] = [];
  const log: OverviewRunEntry[] = [];
  for (let index = 0; index < 200; index++) {
    const id = `TASK-${String(index + 1).padStart(3, "0")}`;
    const deps = index > 0 ? [`TASK-${String(index).padStart(3, "0")}`] : [];
    const status = index < 160 ? "DONE" : index < 165 ? "IN PROGRESS" : "TODO";
    tasks.push(task(id, status, deps));
    log.push(row(id, index * 20, index * 20 + 15));
  }
  const input: Input = {
    tasks,
    run: {
      runner: "loop",
      started: "2026-10-06 22:52",
      finished: null,
      log,
      nowMinute: 4000,
      stopped: false,
    },
    questions: [],
  };
  const runs = 50;
  const start = performance.now();
  for (let index = 0; index < runs; index++) buildTimeline(input);
  const average = (performance.now() - start) / runs;
  t.diagnostic(`buildTimeline average ms: ${average.toFixed(4)}`);
  assert.ok(average <= 10, `average ${average} ms`);
});
