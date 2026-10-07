import assert from "node:assert/strict";
import { test } from "node:test";
import { performance } from "node:perf_hooks";
import { lifecyclePhases, needsYou } from "../client/lifecycle";
import type { Overview, OverviewTask } from "../shared/overview";

function task(id: string, status: string, group: string | null = "A"): OverviewTask {
  return {
    id,
    title: id,
    status,
    deps: "-",
    notes: "",
    spec: "",
    taskClass: null,
    blocker: null,
    error: null,
    depIds: [],
    group,
  };
}

function overview(patch: Partial<Overview> = {}): Overview {
  return {
    plan: { title: "Plan", slug: "20261007-plan", status: "ACTIVE", updated: "2026-10-07" },
    tasks: [task("TASK-001", "DONE"), task("TASK-002", "IN PROGRESS", "B")],
    planVerify: ["npm test", "npm run lint"],
    manualChecks: [],
    run: { runner: "loop", started: "2026-10-07 12:32", finished: null, log: [], nowMinute: 10 },
    questions: [],
    decisions: [],
    history: [],
    message: null,
    warnings: [],
    ...patch,
  };
}

function states(input: Overview) {
  return lifecyclePhases(input).map((phase) => `${phase.name}:${phase.state}`);
}

test("open tasks put the plan in Implement", () => {
  assert.deepEqual(states(overview()), [
    "Plan:done",
    "Implement:current",
    "Verify:todo",
    "Review:todo",
    "Archive:todo",
  ]);
  const phases = lifecyclePhases(overview());
  assert.equal(phases[0].meta, "2 tasks in 2 groups");
  assert.equal(phases[1].meta, "1 of 2 done, autorun live");
  assert.equal(phases[2].meta, "2 checks and 0 manual checks");
  assert.equal(phases[3].meta, "After the run");
});

test("every task done without a finish puts the plan in Verify", () => {
  const input = overview({ tasks: [task("TASK-001", "DONE"), task("TASK-002", "DONE")] });
  assert.deepEqual(states(input).slice(0, 3), ["Plan:done", "Implement:done", "Verify:current"]);
});

test("a finished run puts the plan in Review", () => {
  const input = overview({
    run: {
      runner: "loop",
      started: "2026-10-07 12:32",
      finished: "2026-10-07 15:00",
      log: [],
      nowMinute: 148,
    },
  });
  assert.deepEqual(states(input), [
    "Plan:done",
    "Implement:done",
    "Verify:done",
    "Review:current",
    "Archive:todo",
  ]);
  assert.equal(lifecyclePhases(input)[3].meta, "Draft PR open");
  assert.equal(lifecyclePhases(input)[1].meta, "1 of 2 done");
});

test("an archived plan puts the plan in Archive", () => {
  const input = overview({
    plan: { title: "Plan", slug: null, status: "ARCHIVED", updated: null },
  });
  assert.equal(states(input).at(-1), "Archive:current");
  assert.ok(
    states(input)
      .slice(0, 4)
      .every((state) => state.endsWith(":done")),
  );
});

test("no plan keeps Plan current with No plan yet", () => {
  const input = overview({ tasks: [], run: null, planVerify: [] });
  assert.deepEqual(states(input), [
    "Plan:current",
    "Implement:todo",
    "Verify:todo",
    "Review:todo",
    "Archive:todo",
  ]);
  assert.equal(lifecyclePhases(input)[0].meta, "No plan yet");
});

test("tasks without a Group column count tasks only", () => {
  const input = overview({ tasks: [task("TASK-001", "TODO", null)] });
  assert.equal(lifecyclePhases(input)[0].meta, "1 task");
});

test("Needs you orders blocking questions, defaults, ADRs, then checks", () => {
  const rows = needsYou(
    overview({
      questions: [
        {
          id: "Q-010",
          tasks: ["TASK-010"],
          blocks: [],
          question: "Native scroll?",
          default: "Open the agent",
          status: "DEFAULTED",
        },
        {
          id: "Q-012",
          tasks: ["TASK-011"],
          blocks: [],
          question: "Which color?",
          default: "none",
          status: "OPEN",
        },
        {
          id: "Q-008",
          tasks: ["TASK-008"],
          blocks: ["TASK-009"],
          question: "Network access?",
          default: "No network",
          status: "OPEN",
        },
      ],
      decisions: [
        { id: "ADR-0001", date: "2026-09-30", title: "Active host", status: "accepted" },
        { id: "ADR-0003", date: "2026-10-07", title: "Chip-only sources", status: "proposed" },
      ],
      manualChecks: [
        { text: "Stop a parent (TASK-001).", tasks: ["TASK-001"] },
        { text: "Queued message (TASK-003, TASK-004).", tasks: ["TASK-003", "TASK-004"] },
      ],
    }),
  );
  assert.deepEqual(
    rows.map((row) => [row.kind, row.id, row.tone, row.meta]),
    [
      ["question", "Q-008", "danger", "Q-008 blocks TASK-009"],
      ["question", "Q-010", "warning", "Q-010, has a default"],
      ["question", "Q-012", "neutral", "Q-012, open"],
      ["adr", "ADR-0003", "neutral", "ADR-0003, proposed"],
      ["checks", "manual-checks", "neutral", "From TASK-001, TASK-003, TASK-004"],
    ],
  );
  assert.equal(rows[4].title, "2 manual checks pending");
  assert.deepEqual(needsYou(overview()), []);
});

test("the model stays cheap on a 200-task plan", (t) => {
  const tasks = Array.from({ length: 200 }, (_, index) =>
    task(
      `TASK-${String(index + 1).padStart(3, "0")}`,
      index % 3 ? "TODO" : "DONE",
      `G${index % 9}`,
    ),
  );
  const input = overview({
    tasks,
    questions: Array.from({ length: 40 }, (_, index) => ({
      id: `Q-${index}`,
      tasks: [],
      blocks: index % 2 ? [`TASK-${index}`] : [],
      question: "Why?",
      default: "Yes",
      status: "OPEN",
    })),
    manualChecks: Array.from({ length: 50 }, (_, index) => ({
      text: "Check",
      tasks: [`TASK-${index}`],
    })),
  });
  const runs = 100;
  const start = performance.now();
  for (let index = 0; index < runs; index++) {
    lifecyclePhases(input);
    needsYou(input);
  }
  const average = (performance.now() - start) / runs;
  t.diagnostic(`lifecycle and needs-you average ms: ${average.toFixed(4)}`);
  assert.ok(average <= 5, `average ${average} ms`);
});
