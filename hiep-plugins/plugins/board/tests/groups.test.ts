import assert from "node:assert/strict";
import test from "node:test";
import { groupRuns, type BoardRun } from "../shared/board";
import { createRunStore } from "../server/store";

const card = (id: string, projectKey: string, starred = false): BoardRun => ({
  id,
  agentId: id,
  title: id,
  project: "Same name",
  projectKey,
  starred,
  provider: "codex",
  status: "running",
  startedAt: null,
  endedAt: null,
});

test("groups by identity and preserves first appearance and card order without duplicates", () => {
  const runs = [card("a", "one"), card("b", "two"), card("c", "one"), card("s", "two", true)];
  const result = groupRuns(runs);
  assert.deepEqual(
    result.projects.map((g) => [g.key, g.runs.map((r) => r.id)]),
    [
      ["one", ["a", "c"]],
      ["two", ["s", "b"]],
    ],
  );
  assert.deepEqual(
    runs.map((r) => r.id),
    ["a", "b", "c", "s"],
  );
  runs[3].starred = false;
  assert.deepEqual(
    groupRuns(runs).projects[1].runs.map((r) => r.id),
    ["b", "s"],
  );
});

test("empty and all-starred columns have no empty project groups", () => {
  assert.deepEqual(groupRuns([]), { projects: [] });
  const runs = [card("a", "one", true), card("b", "two", true)];
  assert.deepEqual(
    groupRuns(runs).projects.map((g) => [g.key, g.runs.map((r) => r.id)]),
    [
      ["one", ["a"]],
      ["two", ["b"]],
    ],
  );
});

test("host project identity and stars survive completion and a new hook-only turn", () => {
  const store = createRunStore();
  const agent = {
    id: "a",
    workspaceId: "w",
    parentAgentId: null,
    cwd: "/work/checkout",
    provider: "codex",
    title: "Example",
  };
  store.reconcile([{ ...agent, project: "Atlas", projectKey: "atlas" }], store.revision);
  const scope = store.snapshot().observingSince;
  store.setStarred("a", scope, true);
  store.end(agent, null, { kind: "completed" });
  let runs = store.snapshot().runs;
  assert.equal(runs.length, 1);
  assert.equal(runs[0].status, "completed");
  assert.equal(groupRuns(runs).projects[0].runs[0].projectKey, "project:atlas");
  store.start(agent, "next");
  store.setStarred("a", scope, false);
  runs = store.snapshot().runs;
  assert.equal(runs.length, 1);
  assert.equal(runs[0].status, "running");
  assert.equal(groupRuns(runs).projects[0].key, "project:atlas");
  assert.equal(groupRuns(runs).projects[0].name, "Atlas");
});

test("hook-only finished cards resolve project placement; cwd fallback separates same-name folders", () => {
  const store = createRunStore();
  const agent = {
    id: "a",
    workspaceId: null,
    parentAgentId: null,
    cwd: "/one/app",
    provider: "codex",
    title: "Example",
  };
  store.end(agent, null, { kind: "completed" });
  store.end({ ...agent, id: "b", cwd: "/two/app" }, null, { kind: "completed" });
  assert.equal(groupRuns(store.snapshot().runs).projects.length, 2);
  assert.equal(store.unresolvedProjects().length, 2);
  store.updateProject("a", "/wrong/cwd", { projectName: "Atlas", projectKey: "atlas" });
  assert.equal(store.unresolvedProjects().length, 2);
  store.updateProject("a", agent.cwd, { projectName: "Atlas", projectKey: "atlas" });
  store.updateProject("b", "/two/app", { projectName: "Atlas", projectKey: "atlas" });
  assert.equal(groupRuns(store.snapshot().runs).projects.length, 1);
  assert.equal(store.unresolvedProjects().length, 0);
});
