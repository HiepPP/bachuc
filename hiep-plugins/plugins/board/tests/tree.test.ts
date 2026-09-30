import assert from "node:assert/strict";
import test from "node:test";
import type { BoardRun } from "../shared/board";
import { boardColumns, buildRunTrees } from "../shared/tree";

const run = (
  id: string,
  parentAgentId: string | null = null,
  extra: Partial<BoardRun> = {},
): BoardRun => ({
  id,
  agentId: id,
  parentAgentId,
  title: id,
  project: "App",
  projectKey: "app",
  provider: "codex",
  status: "completed",
  starred: false,
  startedAt: null,
  endedAt: "2026-09-22T00:00:00Z",
  ...extra,
});

test("children arriving before parents form one nested cluster without duplicate cards", () => {
  const input = [run("child", "parent"), run("grandchild", "child"), run("parent"), run("other")];
  const trees = buildRunTrees(input);
  assert.deepEqual(
    trees.map((tree) => tree.run.id),
    ["parent", "other"],
  );
  assert.equal(trees[0].count, 3);
  assert.equal(trees[0].children[0].run.id, "child");
  assert.equal(trees[0].children[0].children[0].run.id, "grandchild");
  assert.equal(input.length, 4);
});

test("a running descendant keeps its finished parent in Running and preserves attention and stars", () => {
  const rows = [
    run("parent"),
    run("child", "parent", { status: "running", needsInput: true, starred: true }),
  ];
  let columns = boardColumns(rows);
  assert.equal(columns.running.length, 1);
  assert.equal(columns.finished.length, 0);
  assert.equal(columns.running[0].run.status, "completed");
  assert.equal(columns.running[0].starred, true);
  assert.equal(columns.running[0].needsInput, true);
  assert.equal(columns.running[0].run.starred, false);
  rows[1].status = "completed";
  columns = boardColumns(rows);
  assert.equal(columns.running.length, 0);
  assert.equal(columns.finished[0].count, 2);
});

test("missing or removed parents leave children visible; unrelated projects never imply parentage", () => {
  const trees = buildRunTrees([run("child", "removed"), run("other"), run("self", "self")]);
  assert.deepEqual(
    trees.map((tree) => tree.run.id),
    ["child", "other", "self"],
  );
  assert.ok(trees.every((tree) => tree.count === 1));
});

test("malformed cycles cannot hide cards or cause infinite recursion", () => {
  const trees = buildRunTrees([run("a", "b"), run("b", "a"), run("c", "a")]);
  assert.deepEqual(
    trees.map((tree) => tree.run.id),
    ["a", "b", "c"],
  );
});

test("stars and latest descendant completion order whole clusters", () => {
  const { finished } = boardColumns([
    run("old"),
    run("parent"),
    run("child", "parent", { endedAt: "2026-09-22T01:00:00Z" }),
    run("starred", null, { starred: true }),
  ]);
  assert.deepEqual(
    finished.map((tree) => tree.run.id),
    ["starred", "parent", "old"],
  );
  assert.equal(
    finished.reduce((count, tree) => count + tree.count, 0),
    4,
  );
});
