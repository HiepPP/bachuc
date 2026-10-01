import assert from "node:assert/strict";
import test from "node:test";
import { createRunStore, runStateSchema } from "../server/store";

const agent = {
  id: "parent",
  workspaceId: "workspace",
  parentAgentId: null,
  provider: "claude",
  cwd: "/demo/project",
  title: "Parent",
};
const child = { ...agent, id: "child", parentAgentId: "parent", title: "Child" };
const grandchild = { ...agent, id: "grandchild", parentAgentId: "child", title: "Grandchild" };
const visible = (store: ReturnType<typeof createRunStore>) =>
  store
    .snapshot()
    .runs.map((run) => run.agentId)
    .sort();

test("board_remove removes finished threads and descendants, never other threads", () => {
  const store = createRunStore();
  store.end(agent, "p1", { kind: "completed" });
  store.end(child, "c1", { kind: "completed" });
  store.end(grandchild, "g1", { kind: "completed" });
  store.end({ ...agent, id: "other" }, "o1", { kind: "completed" });

  assert.deepEqual(store.removeByAgent("child", "parent"), { result: "forbidden" });
  assert.deepEqual(store.removeByAgent("other", "grandchild"), { result: "forbidden" });
  assert.deepEqual(store.removeByAgent("parent", "missing"), { result: "not_found" });
  assert.deepEqual(store.removeByAgent("parent", "child"), { result: "removed", count: 2 });
  assert.deepEqual(visible(store), ["other", "parent"]);
  assert.deepEqual(store.removeByAgent("parent", "child"), { result: "not_found" });
  assert.deepEqual(store.removeByAgent("parent", "parent"), { result: "removed", count: 1 });
  assert.deepEqual(visible(store), ["other"]);
});

test("a running thread is removed only when its turn completes", () => {
  const store = createRunStore();
  store.start(agent, "t1");
  assert.deepEqual(store.removeByAgent("parent", "parent"), { result: "scheduled" });
  assert.deepEqual(visible(store), ["parent"]);
  assert.equal(store.snapshot().runs[0].hasOwnProperty("removeOnFinish"), false);
  // The flag survives a save and restore.
  const restored = createRunStore();
  restored.restore(runStateSchema.parse(JSON.parse(JSON.stringify(store.exportState()))));
  restored.end(agent, "t1", { kind: "completed" });
  assert.deepEqual(visible(restored), []);

  // Failed turns stay visible and drop the request.
  store.end(agent, "t1", { kind: "failed", error: { message: "boom" } });
  assert.deepEqual(visible(store), ["parent"]);
  store.start(agent, "t2");
  store.end(agent, "t2", { kind: "completed" });
  assert.deepEqual(visible(store), ["parent"]);

  // A new turn clears a pending request, and a removed thread returns on its next turn.
  store.start(agent, "t3");
  store.removeByAgent("parent", "parent");
  store.start(agent, "t4");
  store.end(agent, "t4", { kind: "completed" });
  assert.deepEqual(visible(store), ["parent"]);
  store.removeByAgent("parent", "parent");
  store.start(agent, "t5");
  assert.deepEqual(visible(store), ["parent"]);
});
