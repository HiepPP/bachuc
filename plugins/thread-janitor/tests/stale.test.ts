import assert from "node:assert/strict";
import test from "node:test";
import { janitorSettings } from "../shared/settings";
import {
  type JanitorAgent,
  type JanitorWorkspace,
  liveWorkspaces,
  selectStale,
  selectStaleWorkspaces,
} from "../server/stale";

const HOUR_MS = 60 * 60 * 1000;
const now = new Date("2026-09-23T12:00:00.000Z");
const defaults = janitorSettings.schema.parse({ keepRecent: 0 });

function agent(id: string, idleMs: number, overrides: Partial<JanitorAgent> = {}): JanitorAgent {
  return {
    id,
    title: `title ${id}`,
    status: "idle",
    updatedAt: new Date(now.getTime() - idleMs).toISOString(),
    lastUserMessageAt: null,
    pendingPermissions: [],
    labels: {},
    archivedAt: null,
    ...overrides,
  };
}

const ids = (agents: JanitorAgent[]) => agents.map((row) => row.id);

test("defaults are enabled with a 24 hour idle threshold and a 1 hour minimum", () => {
  assert.deepEqual(janitorSettings.schema.parse({}), {
    enabled: true,
    idleHours: 24,
    keepRecent: 7,
  });
  for (const keepRecent of [-1, 1.5]) {
    assert.throws(() => janitorSettings.schema.parse({ keepRecent }));
  }
  assert.throws(() => janitorSettings.schema.parse({ idleHours: 0.5 }));
});

test("idle boundary: only agents idle longer than idleHours are stale", () => {
  const agents = [
    agent("exact", 24 * HOUR_MS),
    agent("older", 24 * HOUR_MS + 1),
    agent("recent", HOUR_MS),
    agent("recent-message", 48 * HOUR_MS, {
      lastUserMessageAt: new Date(now.getTime() - HOUR_MS).toISOString(),
    }),
    agent("bad-date", 48 * HOUR_MS, { updatedAt: "not a date" }),
  ];
  assert.deepEqual(ids(selectStale(agents, now, defaults)), ["older"]);
  assert.deepEqual(ids(selectStale(agents, now, { ...defaults, idleHours: 2 })), [
    "exact",
    "older",
  ]);
});

test("running agent is never stale", () => {
  const agents = [agent("running", 72 * HOUR_MS, { status: "running" })];
  assert.deepEqual(selectStale(agents, now, defaults), []);
});

test("agent with a pending permission is never stale", () => {
  const pending = { id: "perm-1" } as unknown as JanitorAgent["pendingPermissions"][number];
  const agents = [agent("waiting", 72 * HOUR_MS, { pendingPermissions: [pending] })];
  assert.deepEqual(selectStale(agents, now, defaults), []);
});

test("already archived agent is not selected again", () => {
  const agents = [agent("archived", 72 * HOUR_MS, { archivedAt: "2026-09-20T00:00:00.000Z" })];
  assert.deepEqual(selectStale(agents, now, defaults), []);
});

test("disabled setting selects nothing", () => {
  const agents = [agent("old", 72 * HOUR_MS), agent("closed", 72 * HOUR_MS, { status: "closed" })];
  assert.deepEqual(ids(selectStale(agents, now, defaults)), ["old", "closed"]);
  assert.deepEqual(selectStale(agents, now, { ...defaults, enabled: false }), []);
});

function workspace(
  id: string,
  idleMs: number,
  overrides: Partial<JanitorWorkspace> = {},
): JanitorWorkspace {
  return {
    id,
    workspaceKind: "local_checkout",
    workspaceDirectory: `/repo/${id}`,
    projectRootPath: `/repo/${id}`,
    pinnedAt: null,
    archivingAt: null,
    status: "done",
    activityAt: new Date(now.getTime() - idleMs).toISOString(),
    statusEnteredAt: null,
    ...overrides,
  };
}

test("workspaces: only idle, unpinned, non-worktree rows without live agents are stale", () => {
  const old = 25 * HOUR_MS;
  const live = liveWorkspaces([
    { workspaceId: "has-agent", cwd: "/repo/has-agent" },
    { workspaceId: undefined, cwd: "/repo/by-cwd" },
  ]);
  const rows = [
    workspace("old", old),
    workspace("recent", 23 * HOUR_MS),
    workspace("has-agent", old),
    workspace("by-cwd", old),
    workspace("pinned", old, { pinnedAt: now.toISOString() }),
    workspace("worktree", old, { workspaceKind: "worktree" }),
    workspace("running", old, { status: "running" }),
    workspace("needs-input", old, { status: "needs_input" }),
    workspace("archiving", old, { archivingAt: now.toISOString() }),
    workspace("no-activity", old, { activityAt: null }),
  ];
  assert.deepEqual(
    selectStaleWorkspaces(rows, live, now, defaults).map((row) => row.id),
    ["old"],
  );
  assert.deepEqual(selectStaleWorkspaces(rows, live, now, { ...defaults, enabled: false }), []);
});

test("keeps the latest seven unarchived threads even when all are stale", () => {
  const rows = Array.from({ length: 10 }, (_, i) => agent(String(i), (30 + i) * HOUR_MS));
  const settings = janitorSettings.schema.parse({});
  assert.deepEqual(ids(selectStale(rows.reverse(), now, settings)).sort(), ["7", "8", "9"]);
  assert.deepEqual(selectStale(rows.slice(0, 7), now, settings), []);
});

test("retention uses last message activity and excludes already archived threads", () => {
  const rows = [
    agent("archived", HOUR_MS, { archivedAt: now.toISOString() }),
    agent("message", 80 * HOUR_MS, {
      lastUserMessageAt: new Date(now.getTime() - 25 * HOUR_MS).toISOString(),
    }),
    agent("older", 30 * HOUR_MS),
  ];
  assert.deepEqual(ids(selectStale(rows, now, { ...defaults, keepRecent: 1 })), ["older"]);
});

test("equal activity uses stable id ordering across snapshots", () => {
  const rows = [agent("b", 30 * HOUR_MS), agent("a", 30 * HOUR_MS)];
  const settings = { ...defaults, keepRecent: 1 };
  assert.deepEqual(ids(selectStale(rows, now, settings)), ["b"]);
  assert.deepEqual(ids(selectStale(rows.reverse(), now, settings)), ["b"]);
});

test("retained ancestor traversal terminates on cycles and missing parents", () => {
  const rows = [
    agent("child", 30 * HOUR_MS, { labels: { "paseo.parent-agent-id": "parent" } }),
    agent("parent", 40 * HOUR_MS, { labels: { "paseo.parent-agent-id": "child" } }),
    agent("unrelated", 50 * HOUR_MS),
  ];
  assert.deepEqual(ids(selectStale(rows, now, { ...defaults, keepRecent: 1 })), ["unrelated"]);
  const orphan = agent("orphan", 25 * HOUR_MS, { labels: { "paseo.parent-agent-id": "missing" } });
  assert.deepEqual(ids(selectStale([orphan, rows[2]], now, { ...defaults, keepRecent: 1 })), [
    "unrelated",
  ]);
  assert.deepEqual(ids(selectStale(rows, now, defaults)), ["child", "parent", "unrelated"]);
});
