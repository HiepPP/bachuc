import assert from "node:assert/strict";
import test from "node:test";
import type { PluginClientContext, PluginComposerPillContribution } from "@getpaseo/plugin/client";
import { installRemoveButtons, removeFinishedRun } from "../client/remove";
import { boardRpc } from "../shared/board";

test("Remove retries a stale plugin scope only for the same finished turn", async () => {
  const scopes: string[] = [];
  const remove = async (scope: string) => {
    scopes.push(scope);
    return { removed: scope === "current" };
  };
  const run = { id: "finished", endedAt: "end" };
  assert.equal(
    await removeFinishedRun(run, "old", remove, async () => ({
      observingSince: "current",
      runs: [{ ...run, status: "completed" as const }],
    })),
    true,
  );
  assert.deepEqual(scopes, ["old", "current"]);
  assert.equal(
    await removeFinishedRun(run, "old", remove, async () => ({
      observingSince: "current",
      runs: [{ id: "finished", endedAt: "new end", status: "completed" as const }],
    })),
    false,
  );
  assert.deepEqual(scopes, ["old", "current", "old"]);
});

test("only finished threads get Remove; navigate only after confirmed removal; clean up", async () => {
  const pills = new Map<string, PluginComposerPillContribution>();
  const calls: unknown[] = [];
  let removed = false;
  let opens = 0;
  const client = {
    async rpc(contract: unknown, input: unknown) {
      if (contract === boardRpc)
        return {
          observingSince: "scope",
          runs: [
            { id: "finished", agentId: "finished", status: "completed", endedAt: "end" },
            { id: "running", agentId: "running", status: "running", endedAt: null },
          ],
        };
      calls.push(input);
      return { removed };
    },
    paseo: {
      agents: { ref: () => ({ refresh: async () => ({ agent: { workspaceId: "workspace" } }) }) },
    },
    addComposerPill(pill: PluginComposerPillContribution) {
      pills.set(pill.agentId, pill);
      return {
        update() {},
        remove() {
          pills.delete(pill.agentId);
        },
      };
    },
    openSurface(id: string) {
      assert.equal(id, "board");
      opens++;
    },
  } as unknown as PluginClientContext;
  const cleanup = installRemoveButtons(client, () => {});
  try {
    await new Promise((resolve) => setImmediate(resolve));
    assert.deepEqual([...pills.keys()], ["finished"]);
    const behavior = pills.get("finished")!.button.behavior;
    assert.equal(behavior.kind, "action");
    if (behavior.kind !== "action") return;
    await assert.rejects(async () => behavior.onPress(), /Run changed/);
    assert.equal(opens, 0);
    assert.equal(pills.size, 1);
    removed = true;
    await behavior.onPress();
    assert.equal(opens, 1);
    assert.equal(pills.size, 0);
    assert.deepEqual(calls, [
      { id: "finished", observingSince: "scope", endedAt: "end" },
      { id: "finished", observingSince: "scope", endedAt: "end" },
    ]);
  } finally {
    cleanup();
  }
  assert.equal(pills.size, 0);
});

test("cleanup discards an in-flight snapshot without registering buttons", async () => {
  let resolve!: (value: unknown) => void;
  const snapshot = new Promise((done) => {
    resolve = done;
  });
  const client = {
    rpc: () => snapshot,
    paseo: {
      agents: { ref: () => ({ refresh: async () => ({ agent: { workspaceId: "workspace" } }) }) },
    },
    addComposerPill() {
      assert.fail("registered after cleanup");
    },
  } as unknown as PluginClientContext;
  const cleanup = installRemoveButtons(client, () => {});
  cleanup();
  resolve({
    observingSince: "scope",
    runs: [{ id: "finished", agentId: "finished", status: "completed" }],
  });
  await new Promise((done) => setImmediate(done));
});

test("running and finished children jump to their direct parent without removing conversations", async () => {
  const pills = new Map<string, PluginComposerPillContribution>();
  const opened: string[] = [];
  const client = {
    async rpc(contract: unknown) {
      assert.equal(contract, boardRpc);
      return {
        observingSince: "scope",
        runs: [
          { id: "root", agentId: "root", status: "completed" },
          { id: "child", agentId: "child", status: "running", parentAgentId: "root" },
          { id: "leaf", agentId: "leaf", status: "completed", parentAgentId: "child" },
          { id: "self", agentId: "self", status: "running", parentAgentId: "self" },
        ],
      };
    },
    paseo: {
      agents: { ref: () => ({ refresh: async () => ({ agent: { workspaceId: "workspace" } }) }) },
    },
    addComposerPill(pill: PluginComposerPillContribution) {
      pills.set(pill.id, pill);
      return {
        update() {},
        remove() {
          pills.delete(pill.id);
        },
      };
    },
    openSurface() {
      assert.fail("must not return to Board");
    },
  } as unknown as PluginClientContext;
  const cleanup = installRemoveButtons(client, (id) => {
    opened.push(id);
  });
  try {
    await new Promise((done) => setImmediate(done));
    assert.deepEqual([...pills.keys()].sort(), [
      "parent-child",
      "parent-leaf",
      "remove-leaf",
      "remove-root",
    ]);
    for (const id of ["parent-child", "parent-leaf"]) {
      const pill = pills.get(id)!;
      assert.equal(pill.button.label, "Jump To Parent");
      assert.equal(pill.workspaceId, "workspace");
      assert.equal(pill.button.behavior.kind, "action");
      if (pill.button.behavior.kind === "action") await pill.button.behavior.onPress();
    }
    assert.deepEqual(opened, ["root", "child"]);
    assert.equal(pills.size, 4);
  } finally {
    cleanup();
  }
  assert.equal(pills.size, 0);
});

test("Remove & New Thread removes first, then opens the project; never on failure or without cwd", async () => {
  const pills = new Map<string, PluginComposerPillContribution>();
  const started: string[] = [];
  let removed = false;
  const client = {
    async rpc(contract: unknown) {
      if (contract === boardRpc)
        return {
          observingSince: "scope",
          runs: [
            { id: "finished", agentId: "finished", status: "completed", cwd: "/repo" },
            { id: "nocwd", agentId: "nocwd", status: "completed" },
          ],
        };
      return { removed };
    },
    paseo: {
      agents: { ref: () => ({ refresh: async () => ({ agent: { workspaceId: "workspace" } }) }) },
    },
    addComposerPill(pill: PluginComposerPillContribution) {
      pills.set(pill.id, pill);
      return {
        update() {},
        remove() {
          pills.delete(pill.id);
        },
      };
    },
    openSurface() {
      assert.fail("must open the project, not Board");
    },
  } as unknown as PluginClientContext;
  const cleanup = installRemoveButtons(
    client,
    () => {},
    (run) => started.push(run.id),
  );
  try {
    await new Promise((resolve) => setImmediate(resolve));
    assert.deepEqual([...pills.keys()].sort(), [
      "new-thread-finished",
      "remove-finished",
      "remove-nocwd",
    ]);
    const pill = pills.get("new-thread-finished")!;
    assert.equal(pill.button.label, "Remove & New Thread");
    const behavior = pill.button.behavior;
    if (behavior.kind !== "action") return assert.fail("expected action");
    await assert.rejects(async () => behavior.onPress(), /Run changed/);
    assert.deepEqual(started, []);
    removed = true;
    await behavior.onPress();
    assert.deepEqual(started, ["finished"]);
    assert.deepEqual([...pills.keys()], ["remove-nocwd"]);
  } finally {
    cleanup();
  }
  assert.equal(pills.size, 0);
});
