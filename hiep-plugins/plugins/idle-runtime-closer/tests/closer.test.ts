import assert from "node:assert/strict";
import test from "node:test";
import type { PaseoApi } from "@getpaseo/client";
import { createCloser, sweep } from "../server/closer";
import { selectIdle } from "../server/idle";

const MINUTE_MS = 60 * 1000;
const NOW = Date.parse("2026-10-01T12:00:00.000Z");
const SETTINGS = { enabled: true, idleMinutes: 30 };
const PARENT = "paseo.parent-agent-id";

function row(id: string, idleMinutes: number, overrides: Record<string, unknown> = {}) {
  return {
    id,
    status: "idle",
    updatedAt: new Date(NOW - idleMinutes * MINUTE_MS).toISOString(),
    lastUserMessageAt: null as string | null,
    pendingPermissions: [] as unknown[],
    archivedAt: null as string | null,
    labels: {} as Record<string, string>,
    ...overrides,
  };
}

type Row = ReturnType<typeof row>;

function ids(rows: Row[]): string[] {
  return selectIdle(rows as never, new Date(NOW), SETTINGS).map((agent) => agent.id);
}

function fakeApi(rows: Row[], failIds: string[] = []) {
  const closed: string[] = [];
  const api = {
    agents: {
      list: async () => ({
        entries: rows.map((agent) => ({ agent })),
        pageInfo: { hasMore: false },
      }),
      ref: (id: string) => ({
        closeRuntime: async () => {
          if (failIds.includes(id)) throw new Error("boom");
          closed.push(id);
          const agent = rows.find((entry) => entry.id === id);
          if (agent) agent.status = "closed";
        },
      }),
    },
  } as unknown as PaseoApi;
  return { api, closed };
}

const options = () => ({
  now: () => new Date(NOW),
  log: () => {},
  signal: new AbortController().signal,
});

test("selects agents idle past the threshold, by the later of updatedAt and last message", () => {
  assert.deepEqual(
    ids([
      row("old", 31),
      row("fresh", 29),
      row("old-error", 31, { status: "error" }),
      row("recent-message", 31, {
        lastUserMessageAt: new Date(NOW - 5 * MINUTE_MS).toISOString(),
      }),
      row("bad-time", 0, { updatedAt: "not a date" }),
    ]),
    ["old", "old-error"],
  );
});

test("skips running, initializing, closed, archived, and permission-pending agents", () => {
  assert.deepEqual(
    ids([
      row("running", 60, { status: "running" }),
      row("initializing", 60, { status: "initializing" }),
      row("closed", 60, { status: "closed" }),
      row("archived", 60, { archivedAt: new Date(NOW).toISOString() }),
      row("permission", 60, { pendingPermissions: [{ id: "p1" }] }),
    ]),
    [],
  );
});

test("skips every ancestor of a running agent and still closes its idle siblings", () => {
  assert.deepEqual(
    ids([
      row("grandparent", 60),
      row("parent", 60, { labels: { [PARENT]: "grandparent" } }),
      row("child", 60, { status: "running", labels: { [PARENT]: "parent" } }),
      row("sibling", 60, { labels: { [PARENT]: "parent" } }),
      row("unrelated", 60),
    ]),
    ["sibling", "unrelated"],
  );
});

test("a thread opened recently stays open even when its stored activity is old", () => {
  const rows = [row("opened", 600), row("old", 600)];
  const openedAt = new Map([["opened", NOW - 5 * MINUTE_MS]]);
  assert.deepEqual(
    selectIdle(rows as never, new Date(NOW), SETTINGS, openedAt).map((agent) => agent.id),
    ["old"],
  );
  openedAt.set("opened", NOW - 31 * MINUTE_MS);
  assert.deepEqual(
    selectIdle(rows as never, new Date(NOW), SETTINGS, openedAt).map((agent) => agent.id),
    ["opened", "old"],
  );
});

test("disabled settings select nothing", () => {
  assert.deepEqual(
    selectIdle([row("old", 60)] as never, new Date(NOW), { enabled: false, idleMinutes: 30 }),
    [],
  );
});

test("sweep closes idle agents and keeps going after a failure", async () => {
  const { api, closed } = fakeApi([row("a", 60), row("b", 60), row("c", 5)], ["a"]);
  const result = await sweep(api, SETTINGS, options());
  assert.deepEqual(closed, ["b"]);
  assert.deepEqual(result, { checked: 3, idle: 2, closed: 1, failed: 1 });
});

test("sweep re-checks an agent that started a turn after selection", async () => {
  const rows = [row("a", 60), row("b", 60)];
  const { api, closed } = fakeApi(rows);
  const agents = api.agents as unknown as {
    ref: (id: string) => { closeRuntime(): Promise<void> };
  };
  const ref = agents.ref;
  agents.ref = (id) => ({
    closeRuntime: async () => {
      await ref(id).closeRuntime();
      rows[1].status = "running";
    },
  });
  await sweep(api, SETTINGS, options());
  assert.deepEqual(closed, ["a"]);
});

test("timer sweeps wait for a hook to supply the API, and stop ends them", async () => {
  const { api, closed } = fakeApi([row("a", 60)]);
  const closer = createCloser({
    readSettings: async () => ({ status: "ready", values: SETTINGS }) as never,
    log: () => {},
    now: () => NOW,
  });
  assert.equal(closer.fromTimer(), undefined);
  closer.remember(api, "other");
  assert.deepEqual(closed, []);
  await closer.fromTimer();
  assert.deepEqual(closed, ["a"]);
  closer.stop();
  assert.equal(closer.fromTimer(), undefined);
});
