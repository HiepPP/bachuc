import assert from "node:assert/strict";
import { test } from "node:test";
import { dashboardSummary, groupTasks, taskTitle } from "../client/dashboard";
import type { Task } from "../shared/board";

function task(id: string, status: string): Task {
  return {
    id,
    title: `${id} Example task`,
    status,
    deps: "-",
    notes: "",
    spec: "",
    brief: "Do the work.",
    blocker: null,
    error: null,
  };
}

test("summarizes completion and keeps zero progress empty", () => {
  assert.deepEqual(dashboardSummary([]), {
    counts: { active: 0, blocked: 0, todo: 0, done: 0, unknown: 0 },
    percentage: 0,
    total: 0,
  });
  assert.deepEqual(
    dashboardSummary([
      task("TASK-001", "DONE"),
      task("TASK-002", "IN PROGRESS"),
      task("TASK-003", "BLOCKED"),
      task("TASK-004", "TODO"),
    ]),
    {
      counts: { active: 1, blocked: 1, todo: 1, done: 1, unknown: 0 },
      percentage: 25,
      total: 4,
    },
  );
});

test("keeps unknown statuses separate from Todo", () => {
  const invalid = task("TASK-005", "MAYBE");
  const summary = dashboardSummary([invalid]);
  const groups = groupTasks([invalid]);
  assert.equal(summary.counts.todo, 0);
  assert.equal(summary.counts.unknown, 1);
  assert.deepEqual(groups.todo, []);
  assert.deepEqual(
    groups.unknown.map(({ id }) => id),
    ["TASK-005"],
  );
});

test("groups dashboard tasks and separates IDs from titles", () => {
  const tasks = [
    task("TASK-001", "IN PROGRESS"),
    task("TASK-002", "BLOCKED"),
    task("TASK-003", "TODO"),
    task("TASK-004", "DONE"),
  ];
  const groups = groupTasks(tasks);
  assert.deepEqual(
    groups.active.map(({ id }) => id),
    ["TASK-001"],
  );
  assert.deepEqual(
    groups.blocked.map(({ id }) => id),
    ["TASK-002"],
  );
  assert.deepEqual(
    groups.todo.map(({ id }) => id),
    ["TASK-003"],
  );
  assert.deepEqual(
    groups.done.map(({ id }) => id),
    ["TASK-004"],
  );
  assert.equal(taskTitle(tasks[0]), "Example task");
});
