import assert from "node:assert/strict";
import test from "node:test";
import type { PaseoApi } from "@getpaseo/client";
import { closeRuntimes, selectRuntimes } from "../server/runtime";

type Fake = { id: string; status: string; parent?: string; permissions?: number };
const agent = ({ id, status, parent, permissions = 0 }: Fake) => ({
  id,
  status,
  pendingPermissions: Array.from({ length: permissions }, (_, index) => ({ id: String(index) })),
  labels: parent ? { "paseo.parent-agent-id": parent } : {},
});
const select = (fakes: Fake[], root: string) =>
  selectRuntimes(fakes.map(agent) as unknown as Parameters<typeof selectRuntimes>[0], root);

test("a removed parent releases every subagent at any depth, then itself", () => {
  const { close, kept } = select(
    [
      // A grandchild listed before its parent must still join the family.
      { id: "grandchild", status: "error", parent: "child" },
      { id: "parent", status: "idle" },
      { id: "child", status: "idle", parent: "parent" },
      { id: "sibling", status: "idle", parent: "parent" },
      { id: "done", status: "closed", parent: "parent" },
      { id: "stranger", status: "idle" },
      { id: "strangers-child", status: "idle", parent: "stranger" },
    ],
    "parent",
  );
  assert.deepEqual(close, ["grandchild", "child", "sibling", "parent"]);
  assert.equal(kept, 0, "an already closed runtime is neither closed nor kept");
});

test("a removed leaf releases only itself", () => {
  const fakes = [
    { id: "parent", status: "idle" },
    { id: "child", status: "idle", parent: "parent" },
  ];
  assert.deepEqual(select(fakes, "child"), { close: ["child"], kept: 0 });
  assert.deepEqual(select(fakes, "archived"), { close: [], kept: 0 });
});

test("working threads, permission requests, and their ancestors keep their runtime", () => {
  const { close, kept } = select(
    [
      { id: "parent", status: "idle" },
      { id: "busy-branch", status: "idle", parent: "parent" },
      { id: "working", status: "running", parent: "busy-branch" },
      { id: "starting", status: "initializing", parent: "parent" },
      { id: "asking", status: "idle", parent: "parent", permissions: 1 },
      { id: "resting", status: "idle", parent: "parent" },
      { id: "resting-child", status: "error", parent: "asking" },
    ],
    "parent",
  );
  assert.deepEqual(close, ["resting-child", "resting"]);
  assert.equal(kept, 5);
});

function api(fakes: Fake[], options: { pageSize?: number; fail?: string[] } = {}) {
  const closed: string[] = [];
  const state = new Map(fakes.map((fake) => [fake.id, fake]));
  const size = options.pageSize ?? 200;
  const paseo = {
    agents: {
      list: async ({ filter, page }: { filter: object; page: { cursor?: string } }) => {
        assert.deepEqual(filter, { includeArchived: false });
        const start = Number(page.cursor ?? 0);
        return {
          entries: fakes.slice(start, start + size).map((fake) => ({ agent: agent(fake) })),
          pageInfo: { hasMore: start + size < fakes.length, nextCursor: String(start + size) },
        };
      },
      ref: (id: string) => ({
        refresh: async () => ({ agent: agent(state.get(id)!) }),
        closeRuntime: async () => {
          if (options.fail?.includes(id))
            throw new Error("Update the host to close an agent runtime.");
          closed.push(id);
        },
      }),
    },
  } as unknown as PaseoApi;
  return { paseo, closed, state };
}

test("closes the family across list pages and keeps going after a failed close", async () => {
  const fakes = [
    { id: "parent", status: "idle" },
    { id: "a", status: "idle", parent: "parent" },
    { id: "b", status: "idle", parent: "a" },
    { id: "c", status: "running", parent: "parent" },
    { id: "other", status: "idle" },
  ];
  const logs: string[] = [];
  // `c` is working, so `parent` waits for it; `a` fails; `b` still closes.
  const { paseo, closed } = api(fakes, { pageSize: 2, fail: ["a"] });
  assert.deepEqual(await closeRuntimes(paseo, "parent", (line) => logs.push(line)), {
    closed: 1,
    kept: 2,
    failed: 1,
  });
  assert.deepEqual(closed, ["b"]);
  assert.match(logs[0], /could not close runtime a: .*Update the host/);
});

test("subagents close before their parent, and a parent loop ends", async () => {
  const { paseo, closed } = api([
    { id: "parent", status: "idle" },
    { id: "first", status: "idle", parent: "parent" },
    { id: "deep", status: "idle", parent: "first" },
    { id: "deeper", status: "error", parent: "deep" },
    { id: "second", status: "idle", parent: "parent" },
  ]);
  assert.deepEqual(await closeRuntimes(paseo, "parent"), { closed: 5, kept: 0, failed: 0 });
  assert.deepEqual(closed, ["deeper", "deep", "first", "second", "parent"]);
  const loop = api([
    { id: "a", status: "idle", parent: "b" },
    { id: "b", status: "idle", parent: "a" },
    { id: "self", status: "idle", parent: "self" },
  ]);
  await closeRuntimes(loop.paseo, "a");
  await closeRuntimes(loop.paseo, "self");
  assert.deepEqual(loop.closed, ["b", "a", "self"]);
});

test("a thread that started working after the list keeps its runtime", async () => {
  const { paseo, closed, state } = api([
    { id: "parent", status: "idle" },
    { id: "child", status: "idle", parent: "parent" },
  ]);
  state.set("child", { id: "child", status: "running", parent: "parent" });
  assert.deepEqual(await closeRuntimes(paseo, "parent"), { closed: 1, kept: 1, failed: 0 });
  assert.deepEqual(closed, ["parent"], "the working subagent is skipped, the parent still closes");
});
