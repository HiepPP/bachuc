import assert from "node:assert/strict";
import test from "node:test";
import type { PluginHookAgent } from "@getpaseo/plugin/server";

import { createRunStore } from "../server/store";
test("parent relationship survives bootstrap, hooks, completion and new turns", () => {
  const store = createRunStore();
  const child = { ...agent, parentAgentId: "parent" };
  store.reconcile([child], store.revision);
  assert.equal(store.snapshot().runs[0].parentAgentId, "parent");
  store.start(child, "first");
  store.end(child, "first", { kind: "completed" });
  assert.equal(store.snapshot().runs[0].parentAgentId, "parent");
  store.start(child, "second");
  assert.equal(store.snapshot().runs[0].parentAgentId, "parent");
  store.reconcile([{ ...child, parentAgentId: "updated" }], store.revision);
  assert.equal(store.snapshot().runs[0].parentAgentId, "updated");
});

const agent = {
  id: "demo",
  workspaceId: "workspace",
  parentAgentId: null,
  provider: "codex",
  cwd: "/demo/project",
  title: "Example task",
};
function setup() {
  let tick = 0;
  return createRunStore(() => new Date(1_700_000_000_000 + tick++ * 1000).toISOString());
}
test("directory reconciliation hides archived cards, preserving idle agents and live children", () => {
  const s = setup();
  s.end(agent, "finished", { kind: "completed" });
  s.end({ ...agent, id: "idle" }, "idle-turn", { kind: "completed" });
  s.start({ ...agent, id: "child", parentAgentId: agent.id }, "child-turn");
  s.reconcile(
    [{ ...agent, id: "child", parentAgentId: agent.id }],
    s.revision,
    new Set(["idle", "child"]),
  );
  assert.deepEqual(
    s
      .snapshot()
      .runs.map((run) => run.agentId)
      .sort(),
    ["child", "idle"],
  );
  assert.equal(s.snapshot().runs.find((run) => run.agentId === "idle")?.status, "completed");
  s.end(agent, "finished", { kind: "completed" });
  assert.equal(
    s.snapshot().runs.some((run) => run.agentId === agent.id),
    false,
  );
  const restored = setup();
  restored.restore(structuredClone(s.exportState()));
  assert.equal(
    restored.snapshot().runs.some((run) => run.agentId === agent.id),
    false,
  );
});

test("restored running cards archived while offline disappear; stale directory reads cannot hide a new turn", () => {
  const s = setup();
  s.start(agent, "first");
  const restored = setup();
  restored.restore(structuredClone(s.exportState()));
  restored.reconcile([], restored.revision, new Set());
  assert.deepEqual(restored.snapshot().runs, []);
  const revision = s.revision;
  s.start({ ...agent, id: "new" }, "new-turn");
  s.reconcile([], revision, new Set());
  assert.equal(s.snapshot().runs.length, 2);
});
test("known conversation titles survive new turns with untitled lifecycle payloads", () => {
  const s = setup();
  s.start(agent, "first");
  s.end(agent, "first", { kind: "completed" });
  s.start({ ...agent, title: null }, "second");
  s.end({ ...agent, title: null }, "second", { kind: "completed" });
  assert.equal(s.snapshot().runs.length, 1);
  assert.equal(s.snapshot().runs[0].title, agent.title);
});

test("host titles distinguish finished conversations without changing their outcomes", () => {
  const s = setup();
  s.end({ ...agent, title: null }, "first", { kind: "completed" });
  s.end({ ...agent, id: "other", title: null }, "second", { kind: "completed" });
  s.updateTitle(agent.id, "First conversation");
  s.updateTitle("other", "Second conversation");
  s.updateTitle(agent.id, null);
  assert.deepEqual(
    s.snapshot().runs.map((run) => [run.agentId, run.title, run.status]),
    [
      ["other", "Second conversation", "completed"],
      [agent.id, "First conversation", "completed"],
    ],
  );
});
test("remove hides only finished cards, survives duplicate events, and permits future runs", () => {
  const s = setup();
  s.start(agent, "t");
  const { observingSince, runs } = s.snapshot();
  let endedAt: string | null = null;
  assert.equal(
    s.removeFinished(runs[0].id, observingSince, s.snapshot().runs[0]?.endedAt ?? endedAt),
    false,
  );
  s.end(agent, "t", { kind: "completed" });
  endedAt = s.snapshot().runs[0].endedAt;
  assert.equal(s.removeFinished(runs[0].id, "previous plugin instance", endedAt), false);
  assert.equal(
    s.removeFinished(runs[0].id, observingSince, s.snapshot().runs[0]?.endedAt ?? endedAt),
    true,
  );
  assert.equal(
    s.removeFinished(runs[0].id, observingSince, s.snapshot().runs[0]?.endedAt ?? endedAt),
    true,
  );
  s.end(agent, "t", { kind: "completed" });
  s.reconcile([{ ...agent, activeTurn: { turnId: "t", startedAt: null } }], s.revision);
  assert.equal(s.snapshot().runs.length, 0);
  s.start(agent, "t");
  s.end(agent, "t", { kind: "completed" });
  assert.equal(s.snapshot().runs.length, 1);
  assert.equal(s.snapshot().runs[0].id, runs[0].id);
  assert.equal(s.removeFinished(runs[0].id, observingSince, endedAt), false);
});
test("one card per conversation across repeated and reused turn IDs", () => {
  const s = setup();
  s.start(agent, "turn");
  s.start(agent, "turn");
  assert.equal(s.snapshot().runs.length, 1);
  s.end(agent, "turn", { kind: "completed" });
  s.end(agent, "turn", { kind: "completed" });
  assert.equal(s.snapshot().runs.length, 1);
  const oldId = s.snapshot().runs[0].id;
  s.start(agent, "turn");
  s.end(agent, "turn", { kind: "failed", error: { message: "test" } });
  assert.equal(s.snapshot().runs.length, 1);
  assert.equal(s.snapshot().runs[0].id, oldId);
  assert.equal(s.snapshot().runs[0].status, "failed");
});
test("bootstrap start timestamp survives a matching start hook and terminal event", () => {
  const s = setup();
  const startedAt = "2026-09-20T00:00:00.000Z";
  s.reconcile(
    [{ ...agent, project: "Display name", activeTurn: { turnId: "t", startedAt } }],
    s.revision,
  );
  s.start(agent, "t");
  s.end(agent, "t", { kind: "completed" });
  assert.equal(s.snapshot().runs.length, 1);
  assert.equal(s.snapshot().runs[0].startedAt, startedAt);
  assert.equal(s.snapshot().runs[0].project, "Display name");
});
test("idle reconciliation never invents success; a late result can resolve unknown", () => {
  const s = setup();
  s.start(agent, "t");
  s.reconcile([], s.revision);
  assert.equal(s.snapshot().runs[0].status, "unknown");
  s.end(agent, "t", { kind: "completed" });
  assert.equal(s.snapshot().runs[0].status, "completed");
  assert.equal(s.snapshot().runs.length, 1);
});
test("snapshot racing an end event cannot resurrect a completed run", () => {
  const s = setup();
  s.start(agent, "t");
  const revision = s.revision;
  const rows = [{ ...agent, activeTurn: { turnId: "t", startedAt: null } }];
  s.end(agent, "t", { kind: "completed" });
  s.reconcile(rows, revision);
  s.reconcile(rows, s.revision);
  assert.deepEqual(
    s.snapshot().runs.map((r) => r.status),
    ["completed"],
  );
});
test("stale terminal does not close a newer run; changed active identity records missing outcome", () => {
  const s = setup();
  s.start(agent, "old");
  s.reconcile([{ ...agent, activeTurn: { turnId: "new", startedAt: null } }], s.revision);
  s.end(agent, "old", { kind: "completed" });
  assert.deepEqual(
    s.snapshot().runs.map((r) => r.status),
    ["running"],
  );
  s.end(agent, "new", { kind: "canceled", reason: "test" });
  assert.equal(s.snapshot().runs[0].status, "cancelled");
});
test("null IDs and missing starts preserve confirmed outcome without inventing duration", () => {
  const s = setup();
  s.end(agent, null, { kind: "completed" });
  assert.equal(s.snapshot().runs[0].startedAt, null);
  s.start(agent, null);
  s.end(agent, null, { kind: "canceled", reason: "test" });
  assert.equal(s.snapshot().runs.length, 1);
  assert.equal(s.snapshot().runs[0].status, "cancelled");
});
test("keep latest 50 finished runs and all active runs; cleanup clears both", () => {
  const s = setup();
  for (let i = 0; i < 55; i++) {
    const other = { ...agent, id: `agent-${i}` };
    s.start(other, String(i));
    s.end(other, String(i), { kind: "completed" });
  }
  s.start({ ...agent, id: "active" }, "running");
  assert.equal(s.snapshot().runs.length, 51);
  assert.equal(s.snapshot().runs[0].status, "running");
  assert.equal(s.snapshot().runs[1].id, "agent-54");
  s.clear();
  assert.equal(s.snapshot().runs.length, 0);
});

test("a late confirmed outcome moves into recent history before retention trims", () => {
  const s = setup();
  s.start(agent, "missing");
  s.reconcile([], s.revision);
  for (let i = 0; i < 49; i++) {
    const other = { ...agent, id: `agent-${i}` };
    s.start(other, String(i));
    s.end(other, String(i), { kind: "completed" });
  }
  s.end(agent, "missing", { kind: "completed" });
  const latest = { ...agent, id: "latest" };
  s.start(latest, "latest");
  s.end(latest, "latest", { kind: "completed" });
  assert.equal(s.snapshot().runs.length, 50);
  assert.equal(s.snapshot().runs[1].id, "demo");
});

test("stars survive column changes and new turns; unstar and stale requests are handled", () => {
  const s = setup();
  s.start(agent, "first");
  const scope = s.snapshot().observingSince;
  assert.equal(s.snapshot().runs[0].starred, false);
  assert.equal(s.setStarred(agent.id, "stale", true), false);
  assert.equal(s.setStarred("missing", scope, true), false);
  assert.equal(s.setStarred(agent.id, scope, true), true);
  s.end(agent, "first", { kind: "completed" });
  assert.equal(s.snapshot().runs[0].starred, true);
  s.start(agent, "second");
  assert.equal(s.snapshot().runs[0].starred, true);
  s.reconcile([{ ...agent, activeTurn: { turnId: "third", startedAt: null } }], s.revision);
  assert.equal(s.snapshot().runs[0].starred, true);
  s.end(agent, "third", { kind: "completed" });
  assert.equal(s.setStarred(agent.id, scope, false), true);
  assert.equal(s.snapshot().runs[0].starred, false);
  s.removeFinished(agent.id, scope, s.snapshot().runs[0].endedAt);
  assert.equal(s.setStarred(agent.id, scope, true), false);
});

test("input badge follows pending requests and clears after response or turn end", () => {
  const s = setup();
  s.reconcile([{ ...agent, pendingPermissions: [{}] }], s.revision);
  assert.equal(s.snapshot().runs[0].needsInput, true);
  s.reconcile([{ ...agent, pendingPermissions: [] }], s.revision);
  assert.equal(s.snapshot().runs[0].needsInput, false);
  s.reconcile([{ ...agent, attentionReason: "permission" }], s.revision);
  assert.equal(s.snapshot().runs[0].needsInput, true);
  s.end(agent, null, { kind: "completed" });
  assert.equal(s.snapshot().runs[0].needsInput, false);
});

test("finished and error attention do not show the input badge", () => {
  const s = setup();
  for (const attentionReason of ["finished", "error"] as const) {
    s.reconcile([{ ...agent, attentionReason }], s.revision);
    assert.equal(s.snapshot().runs[0].needsInput, false);
  }
});

const family = {
  parent: { ...agent, id: "parent" },
  child: { ...agent, id: "child", parentAgentId: "parent" },
  grandchild: { ...agent, id: "grandchild", parentAgentId: "child" },
};
function finish(s: ReturnType<typeof setup>, ...members: PluginHookAgent[]) {
  for (const member of members) {
    s.start(member, "t");
    s.end(member, "t", { kind: "completed" });
  }
}

test("removing a parent also removes its finished subagents; guards remove nothing", () => {
  const s = setup();
  finish(s, family.grandchild, family.child, family.parent);
  const { observingSince, runs } = s.snapshot();
  const endedAt = runs.find((run) => run.id === "parent")!.endedAt;
  assert.equal(s.removeFinished("parent", "stale", endedAt), false);
  assert.equal(s.removeFinished("parent", observingSince, "wrong"), false);
  assert.equal(s.snapshot().runs.length, 3);
  assert.equal(s.removeFinished("parent", observingSince, endedAt), true);
  assert.deepEqual(s.snapshot().runs, []);
});

test("a running subagent and its own subtree stay visible when the parent is removed", () => {
  const s = setup();
  finish(s, family.parent, family.grandchild);
  s.start(family.child, "t");
  const { observingSince, runs } = s.snapshot();
  const endedAt = runs.find((run) => run.id === "parent")!.endedAt;
  assert.equal(s.removeFinished("parent", observingSince, endedAt), true);
  assert.deepEqual(
    s
      .snapshot()
      .runs.map((run) => [run.id, run.status])
      .sort(),
    [
      ["child", "running"],
      ["grandchild", "completed"],
    ],
  );
  s.end(family.child, "t", { kind: "completed" });
  assert.deepEqual(
    s
      .snapshot()
      .runs.map((run) => run.id)
      .sort(),
    ["child", "grandchild"],
  );
});

test("removing a subagent keeps its parent and removes only its own subtree", () => {
  const s = setup();
  finish(s, family.parent, family.child, family.grandchild);
  const { observingSince, runs } = s.snapshot();
  const endedAt = runs.find((run) => run.id === "child")!.endedAt;
  assert.equal(s.removeFinished("child", observingSince, endedAt), true);
  assert.deepEqual(
    s.snapshot().runs.map((run) => run.id),
    ["parent"],
  );
});

test("malformed parent cycles cannot make removal loop", () => {
  const s = setup();
  finish(s, { ...agent, id: "a", parentAgentId: "b" }, { ...agent, id: "b", parentAgentId: "a" });
  const { observingSince, runs } = s.snapshot();
  const endedAt = runs.find((run) => run.id === "a")!.endedAt;
  assert.equal(s.removeFinished("a", observingSince, endedAt), true);
  assert.deepEqual(s.snapshot().runs, []);
});

test("finished runs are unread until the host reports them opened after they ended", () => {
  const s = setup();
  s.end(agent, "first", { kind: "completed" });
  const unread = () => s.snapshot().runs.find((run) => run.agentId === agent.id)?.unread;
  // No host read yet.
  assert.equal(unread(), true);
  const endedAt = s.snapshot().runs[0].endedAt!;
  const after = new Date(Date.parse(endedAt) + 500).toISOString();
  s.setUnread(new Set([agent.id]), after);
  assert.equal(unread(), true);
  s.setUnread(new Set(), after);
  assert.equal(unread(), false);
  // A list that started before the run ended cannot know it was opened.
  s.setUnread(new Set(), new Date(Date.parse(endedAt) - 500).toISOString());
  assert.equal(unread(), true);
  s.start(agent, "second");
  assert.equal(unread(), false);
});
