import assert from "node:assert/strict";
import test from "node:test";
import type { PluginClientContext, PluginComposerPillContribution } from "@getpaseo/plugin/client";
import { closeRemovedRuntimes, installRemoveButtons, removeFinishedRun } from "../client/remove";
import { boardRpc, closeRuntimeRpc } from "../shared/board";

const agents = { ref: () => ({ refresh: async () => ({ agent: { workspaceId: "workspace" } }) }) };

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
  const closed: string[] = [];
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
      if (contract === closeRuntimeRpc) {
        closed.push((input as { agentId: string }).agentId);
        return { closed: 1, kept: 0, failed: 0 };
      }
      calls.push(input);
      return { removed };
    },
    paseo: { agents },
    addComposerPill(pill: PluginComposerPillContribution) {
      pills.set(pill.agentId ?? "", pill);
      return {
        update() {},
        remove() {
          pills.delete(pill.agentId ?? "");
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
    assert.deepEqual(closed, [], "a failed removal keeps the runtime");
    removed = true;
    await behavior.onPress();
    assert.equal(opens, 1);
    assert.equal(pills.size, 0);
    assert.deepEqual(closed, ["finished"]);
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
      assert.equal(pill.button.color, "#f97316");
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
  const closed: string[] = [];
  let removed = false;
  const client = {
    async rpc(contract: unknown, input: unknown) {
      if (contract === boardRpc)
        return {
          observingSince: "scope",
          runs: [
            { id: "finished", agentId: "finished", status: "completed", cwd: "/repo" },
            { id: "nocwd", agentId: "nocwd", status: "completed" },
          ],
        };
      if (contract === closeRuntimeRpc) {
        closed.push((input as { agentId: string }).agentId);
        return { closed: 2, kept: 1, failed: 0 };
      }
      return { removed };
    },
    paseo: { agents },
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
    assert.deepEqual(closed, []);
    removed = true;
    await behavior.onPress();
    assert.deepEqual(started, ["finished"]);
    assert.deepEqual(closed, ["finished"]);
    assert.deepEqual([...pills.keys()], ["remove-nocwd"]);
  } finally {
    cleanup();
  }
  assert.equal(pills.size, 0);
});

test("thread actions sit in the corner, stacked Remove, Remove & New Thread, Jump To Parent", async (t) => {
  t.mock.timers.enable({ apis: ["setTimeout"] });
  const stack: PluginComposerPillContribution[] = [];
  let status = "running";
  const client = {
    async rpc() {
      return {
        observingSince: "scope",
        runs: [{ id: "child", agentId: "child", status, cwd: "/repo", parentAgentId: "root" }],
      };
    },
    paseo: {
      agents: { ref: () => ({ refresh: async () => ({ agent: { workspaceId: "workspace" } }) }) },
    },
    addComposerPill(pill: PluginComposerPillContribution) {
      stack.push(pill);
      return {
        update() {},
        remove() {
          const index = stack.indexOf(pill);
          if (index >= 0) stack.splice(index, 1);
        },
      };
    },
  } as unknown as PluginClientContext;
  const settle = () => new Promise((resolve) => setImmediate(resolve));
  const ids = () => stack.map((pill) => pill.id);
  const cleanup = installRemoveButtons(
    client,
    () => {},
    () => {},
  );
  try {
    await settle();
    assert.deepEqual(ids(), ["parent-child"]);
    // The child finishes: Remove actions must land above the parent pill registered earlier.
    status = "completed";
    for (let pass = 0; pass < 2; pass++) {
      t.mock.timers.tick(2_000);
      await settle();
      assert.deepEqual(ids(), ["remove-child", "new-thread-child", "parent-child"]);
    }
    assert.ok(stack.every((pill) => pill.placement === "corner"));
    assert.deepEqual(
      stack.map((pill) => pill.button.color),
      [undefined, undefined, "#f97316"],
    );
  } finally {
    cleanup();
  }
  assert.equal(stack.length, 0);
});

test("a failed runtime close reports after Remove has already returned to Board", async () => {
  const pills = new Map<string, PluginComposerPillContribution>();
  let opens = 0;
  const client = {
    async rpc(contract: unknown) {
      if (contract === boardRpc)
        return {
          observingSince: "scope",
          runs: [{ id: "finished", agentId: "finished", status: "completed" }],
        };
      if (contract === closeRuntimeRpc) return { closed: 1, kept: 0, failed: 2 };
      return { removed: true };
    },
    paseo: { agents },
    addComposerPill(pill: PluginComposerPillContribution) {
      pills.set(pill.id, pill);
      return { update() {}, remove() {} };
    },
    openSurface() {
      opens++;
    },
  } as unknown as PluginClientContext;
  const cleanup = installRemoveButtons(client, () => {});
  try {
    await new Promise((resolve) => setImmediate(resolve));
    const behavior = pills.get("remove-finished")!.button.behavior;
    if (behavior.kind !== "action") return assert.fail("expected action");
    await assert.rejects(async () => behavior.onPress(), /Could not close 2 thread processes/);
    assert.equal(opens, 1, "the removal already happened, so the user still returns to Board");
  } finally {
    cleanup();
  }
});

test("closing removed runtimes asks the thread's host and fails only on a failed close", async () => {
  const asked: unknown[] = [];
  const host = (failed: number) =>
    ({
      rpc: async (contract: unknown, input: unknown) => {
        asked.push([contract === closeRuntimeRpc, input]);
        return { closed: 3, kept: 2, failed };
      },
    }) as unknown as Pick<PluginClientContext, "rpc">;
  await closeRemovedRuntimes(host(0), "parent");
  assert.deepEqual(asked, [[true, { agentId: "parent" }]]);
  await assert.rejects(
    closeRemovedRuntimes(host(1), "parent"),
    /Could not close 1 thread process\./,
  );
});
