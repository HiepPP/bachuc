import assert from "node:assert/strict";
import { test } from "node:test";
import { runningLogLine } from "../client/run-log";

test("a running task in a live run shows now and its minutes", () => {
  assert.deepEqual(runningLogLine({ id: "TASK-006", minutes: 37 }, false), {
    cells: ["", "now", "006", "running"],
    tail: "37m",
    tone: "active",
  });
});

test("a running task in a stopped run shows stalled with no minutes", () => {
  assert.deepEqual(runningLogLine({ id: "TASK-006", minutes: 0 }, true), {
    cells: ["", "-", "006", "stalled"],
    tail: "",
    tone: "warning",
  });
});
