import assert from "node:assert/strict";
import { performance } from "node:perf_hooks";
import { test } from "node:test";
import { buildPlanTree } from "../client/plan-tree";
import { buildTimeline } from "../client/timeline";
import type { Overview, OverviewRunEntry, OverviewTask } from "../shared/overview";

function task(
  id: string,
  status: string,
  depIds: string[] = [],
  group: string | null = null,
): OverviewTask {
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
    group,
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

function run(log: OverviewRunEntry[], nowMinute: number): Input["run"] {
  return {
    runner: "loop",
    started: "2026-10-14 10:00",
    finished: null,
    log,
    nowMinute,
    stopped: false,
  };
}

const STARTED = "2026-10-14 10:00";

// The Fleet map plan: 090 waits on 086..089, 091 waits on 090.
function fleetMap(status = "TODO"): Input {
  return {
    tasks: [
      task("TASK-086", status, [], "A"),
      task("TASK-087", status, [], "B"),
      task("TASK-088", status, [], "C"),
      task("TASK-089", status, [], "D"),
      task("TASK-090", status, ["TASK-086", "TASK-087", "TASK-088", "TASK-089"], "E"),
      task("TASK-091", status, ["TASK-090"], "F"),
    ],
    run: null,
    questions: [],
  };
}

test("the Fleet map plan gives three waves of 4, 1, and 1 rows", () => {
  const { waves } = buildPlanTree(fleetMap(), STARTED);
  assert.deepEqual(
    waves.map((wave) => wave.rows.map((item) => item.id)),
    [["TASK-086", "TASK-087", "TASK-088", "TASK-089"], ["TASK-090"], ["TASK-091"]],
  );
  assert.deepEqual(
    waves.map((wave) => wave.number),
    [1, 2, 3],
  );
  assert.equal(
    waves.some((wave) => wave.cyclic),
    false,
  );
});

test("wave headers name the groups in parallel and the deps still to finish", () => {
  const { waves } = buildPlanTree(fleetMap(), STARTED);
  assert.equal(waves[0].header, "4 groups in parallel");
  assert.equal(waves[1].header, "starts when TASK-086, TASK-087, TASK-088 and TASK-089 finish");
  assert.equal(waves[2].header, "starts when TASK-090 finishes");
});

test("two TASKs of one group count as one group; a TASK with no group counts alone", () => {
  const tasks = [
    task("TASK-001", "TODO", [], "A"),
    task("TASK-002", "TODO", [], "A"),
    task("TASK-003", "TODO", [], null),
    task("TASK-004", "TODO", [], null),
  ];
  const { waves } = buildPlanTree({ tasks, run: null, questions: [] }, STARTED);
  assert.equal(waves[0].header, "3 groups in parallel");
  const single = buildPlanTree(
    { tasks: [task("TASK-001", "TODO")], run: null, questions: [] },
    STARTED,
  );
  assert.equal(single.waves[0].header, "1 group in parallel");
});

test("a later wave names only the deps that are not done", () => {
  const tasks = [
    task("TASK-001", "DONE"),
    task("TASK-002", "TODO"),
    task("TASK-003", "TODO", ["TASK-001", "TASK-002"]),
  ];
  const { waves } = buildPlanTree({ tasks, run: null, questions: [] }, STARTED);
  assert.equal(waves[1].header, "starts when TASK-002 finishes");
  const ready = buildPlanTree(
    {
      tasks: [task("TASK-001", "DONE"), task("TASK-003", "TODO", ["TASK-001"])],
      run: null,
      questions: [],
    },
    STARTED,
  );
  assert.equal(ready.waves[1].header, "ready to start");
});

test("a row title drops the leading TASK ID, as the task list does", () => {
  const item = { ...task("TASK-095", "DONE"), title: "TASK-095 Plan tree model" };
  const { waves } = buildPlanTree({ tasks: [item], run: null, questions: [] }, STARTED);
  assert.equal(waves[0].rows[0].title, "Plan tree model");
});

test("a wave with a running TASK says running", () => {
  const { waves } = buildPlanTree(
    {
      tasks: [
        task("TASK-001", "IN PROGRESS"),
        task("TASK-002", "DONE"),
        task("TASK-003", "IN PROGRESS", ["TASK-002"]),
      ],
      run: null,
      questions: [],
    },
    STARTED,
  );
  assert.match(waves[0].header, /^running, \d+ groups? in parallel$/);
  assert.equal(waves[1].header, "running");
});

test("a finished wave says done instead of a start condition", () => {
  const { waves } = buildPlanTree(
    {
      tasks: [task("TASK-001", "DONE"), task("TASK-002", "DONE", ["TASK-001"])],
      run: null,
      questions: [],
    },
    STARTED,
  );
  assert.equal(waves[0].header, "done");
  assert.equal(waves[1].header, "done");
});

test("a dep outside the plan does not count", () => {
  const tasks = [
    task("TASK-010", "TODO", ["TASK-999"]),
    task("TASK-011", "TODO", ["TASK-010", "TASK-999"]),
  ];
  const { waves } = buildPlanTree({ tasks, run: null, questions: [] }, STARTED);
  assert.deepEqual(
    waves.map((wave) => wave.rows.map((item) => item.id)),
    [["TASK-010"], ["TASK-011"]],
  );
  assert.equal(waves[1].header, "starts when TASK-010 finishes");
});

test("a TASK sits one wave after its highest dep, and keeps Tracker order inside a wave", () => {
  const tasks = [
    task("TASK-004", "TODO", ["TASK-002", "TASK-001"]),
    task("TASK-003", "TODO"),
    task("TASK-002", "TODO", ["TASK-001"]),
    task("TASK-001", "TODO"),
  ];
  const { waves } = buildPlanTree({ tasks, run: null, questions: [] }, STARTED);
  assert.deepEqual(
    waves.map((wave) => wave.rows.map((item) => item.id)),
    [["TASK-003", "TASK-001"], ["TASK-002"], ["TASK-004"]],
  );
});

test("a cycle ends in one last wave marked cyclic and does not throw", () => {
  const tasks = [
    task("TASK-001", "TODO"),
    task("TASK-002", "TODO", ["TASK-001", "TASK-003"]),
    task("TASK-003", "TODO", ["TASK-002"]),
    task("TASK-004", "TODO", ["TASK-003"]),
    task("TASK-005", "TODO", ["TASK-005"]),
  ];
  const { waves } = buildPlanTree({ tasks, run: null, questions: [] }, STARTED);
  assert.equal(waves.length, 2);
  assert.equal(waves[0].cyclic, false);
  assert.deepEqual(
    waves[1].rows.map((item) => item.id),
    ["TASK-002", "TASK-003", "TASK-004", "TASK-005"],
  );
  assert.equal(waves[1].cyclic, true);
  assert.equal(waves[1].number, 2);
  assert.equal(waves[1].done, false);
});

test("done, running, planned, and blocked rows get their time text", () => {
  const input: Input = {
    tasks: [
      task("TASK-001", "DONE"),
      task("TASK-002", "DONE"),
      task("TASK-003", "IN PROGRESS"),
      task("TASK-004", "TODO", ["TASK-003"]),
      task("TASK-005", "BLOCKED"),
      task("TASK-006", "DONE"),
    ],
    run: run([row("TASK-001", 0, 4), row("TASK-002", 5, 9)], 17),
    questions: [],
  };
  const { waves } = buildPlanTree(input, STARTED);
  const byId = new Map(waves.flatMap((wave) => wave.rows).map((item) => [item.id, item]));
  const view = (id: string) => {
    const item = byId.get(id);
    return item && { kind: item.kind, time: item.time, over: item.over };
  };
  assert.deepEqual(view("TASK-001"), { kind: "done", time: "4m", over: false });
  // Started at minute 9, now 17: 8 minutes against the average of 4.
  assert.deepEqual(view("TASK-003"), { kind: "running", time: "8m / 4m", over: true });
  // Expected start is minute 17 after 10:00.
  assert.deepEqual(view("TASK-004"), { kind: "planned", time: "~10:17", over: false });
  assert.deepEqual(view("TASK-005"), { kind: "blocked", time: null, over: false });
  // Done in the Tracker but with no log row: no data, no time.
  assert.deepEqual(view("TASK-006"), { kind: "done", time: null, over: false });
});

test("a running row under the average is not over", () => {
  const input: Input = {
    tasks: [task("TASK-001", "DONE"), task("TASK-002", "IN PROGRESS")],
    run: run([row("TASK-001", 0, 4)], 6),
    questions: [],
  };
  const item = buildPlanTree(input, STARTED).waves[0].rows[1];
  assert.deepEqual({ time: item.time, over: item.over }, { time: "2m / 4m", over: false });
});

test("a planned row has no time when Started has no clock", () => {
  const input: Input = {
    tasks: [task("TASK-001", "TODO")],
    run: run([], 0),
    questions: [],
  };
  assert.equal(buildPlanTree(input, null).waves[0].rows[0].time, null);
  assert.equal(buildPlanTree(input, "2026-10-14").waves[0].rows[0].time, "~00:00");
});

test("a wave is done when every row is done", () => {
  const input: Input = {
    tasks: [
      task("TASK-001", "DONE"),
      task("TASK-002", "DONE"),
      task("TASK-003", "DONE", ["TASK-001"]),
      task("TASK-004", "TODO", ["TASK-003"]),
    ],
    run: null,
    questions: [],
  };
  assert.deepEqual(
    buildPlanTree(input, STARTED).waves.map((wave) => wave.done),
    [true, true, false],
  );
});

test("an empty plan has no waves", () => {
  const tree = buildPlanTree({ tasks: [], run: null, questions: [] }, STARTED);
  assert.deepEqual(tree.waves, []);
  assert.deepEqual(tree.pace, {
    averageMinutes: 20,
    finishClock: null,
    maxParallel: 0,
    slowest: null,
  });
});

test("the pace line matches the timeline stats for the same input", () => {
  const input: Input = {
    tasks: [
      task("TASK-001", "DONE"),
      task("TASK-002", "DONE"),
      task("TASK-003", "IN PROGRESS"),
      task("TASK-004", "TODO", ["TASK-003"]),
      task("TASK-005", "BLOCKED"),
    ],
    run: run([row("TASK-001", 0, 4), row("TASK-002", 5, 9)], 17),
    questions: [],
  };
  const timeline = buildTimeline(input);
  const { pace } = buildPlanTree(input, STARTED);
  assert.equal(pace.averageMinutes, timeline.stats.averageMinutes);
  assert.equal(pace.maxParallel, timeline.stats.maxParallel);
  assert.deepEqual(pace.slowest, timeline.stats.overAverage);
  assert.deepEqual(pace.slowest, { id: "TASK-003", minutes: 4 });
  // The last open bar ends at minute 25: 10:25.
  assert.equal(timeline.finishMinute, 25);
  assert.equal(pace.finishClock, "10:25");
});

test("the finish clock wraps past midnight", () => {
  const input: Input = {
    tasks: [task("TASK-001", "TODO")],
    run: run([], 0),
    questions: [],
  };
  assert.equal(buildPlanTree(input, "2026-10-14 23:50").pace.finishClock, "00:10");
});

test("200 TASKs with chained deps build in under 20 ms", () => {
  const tasks = Array.from({ length: 200 }, (_, index) =>
    task(
      `TASK-${String(index + 1).padStart(3, "0")}`,
      index < 100 ? "DONE" : "TODO",
      index === 0 ? [] : [`TASK-${String(index).padStart(3, "0")}`],
    ),
  );
  const log = tasks.slice(0, 100).map((item, index) => row(item.id, index * 5, index * 5 + 4));
  const input: Input = { tasks, run: run(log, 500), questions: [] };
  buildPlanTree(input, STARTED);
  const begin = performance.now();
  const { waves } = buildPlanTree(input, STARTED);
  const elapsed = performance.now() - begin;
  assert.equal(waves.length, 200);
  assert.ok(elapsed < 20, `built in ${elapsed.toFixed(1)} ms`);
});
