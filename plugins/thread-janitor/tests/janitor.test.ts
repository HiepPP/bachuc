import assert from "node:assert/strict";
import test from "node:test";
import type { PaseoApi } from "@getpaseo/client";
import { SWEEP_THROTTLE_MS, createJanitor, sweep } from "../server/janitor";

const HOUR_MS = 60 * 60 * 1000;
const NOW = Date.parse("2026-09-23T12:00:00.000Z");
const SECRET_TITLE = "private prompt title";

function row(id: string, idleHours: number) {
  return {
    id,
    title: SECRET_TITLE,
    status: "idle",
    updatedAt: new Date(NOW - idleHours * HOUR_MS).toISOString(),
    lastUserMessageAt: null,
    pendingPermissions: [],
    archivedAt: null,
  };
}

type Row = ReturnType<typeof row> & {
  workspaceId?: string;
  cwd?: string;
  labels?: Record<string, string>;
};

function workspace(id: string, idleHours: number, overrides: Record<string, unknown> = {}) {
  return {
    id,
    workspaceKind: "local_checkout",
    workspaceDirectory: `/repo/${id}`,
    projectRootPath: `/repo/${id}`,
    pinnedAt: null,
    archivingAt: null,
    status: "done",
    activityAt: new Date(NOW - idleHours * HOUR_MS).toISOString(),
    statusEnteredAt: null,
    ...overrides,
  };
}

type Workspace = ReturnType<typeof workspace>;

function fakeApi(
  rows: Row[],
  failIds: string[] = [],
  fresh = (agent?: Row) => agent,
  workspaces: Workspace[] = [],
  terminalWorkspaceIds: string[] = [],
) {
  const archived: string[] = [];
  const archivedWorkspaces: string[] = [];
  let lists = 0;
  // Model daemon cascading for closed child tabs in the same workspace.
  const archiveTree = (id: string) => {
    if (archived.includes(id)) return;
    archived.push(id);
    for (const child of rows) {
      if (child.labels?.["paseo.parent-agent-id"] === id) archiveTree(child.id);
    }
  };
  const api = {
    workspaces: {
      list: async () => ({ entries: workspaces, pageInfo: { hasMore: false } }),
      ref: (id: string) => ({
        refresh: async () => workspaces.find((entry) => entry.id === id) ?? null,
        archive: async () => {
          archivedWorkspaces.push(id);
          return {
            requestId: "r",
            workspaceId: id,
            archivedAt: new Date(NOW).toISOString(),
            error: null,
          };
        },
      }),
    },
    terminals: {
      list: async ({ workspaceId }: { workspaceId: string }) => ({
        requestId: "r",
        entries: terminalWorkspaceIds.includes(workspaceId) ? [{ id: "t1" }] : [],
      }),
    },
    agents: {
      list: async () => {
        lists += 1;
        return {
          entries: rows.filter((agent) => !archived.includes(agent.id)).map((agent) => ({ agent })),
          pageInfo: { hasMore: false },
        };
      },
      ref: (id: string) => ({
        refresh: async () => ({
          agent: fresh(rows.find((agent) => agent.id === id)),
          project: null,
        }),
        archive: async () => {
          if (failIds.includes(id)) throw new Error("daemon refused");
          archiveTree(id);
          return { archivedAt: new Date(NOW).toISOString() };
        },
      }),
    },
  } as unknown as PaseoApi;
  return { api, archived, archivedWorkspaces, lists: () => lists };
}

const ready =
  (enabled = true) =>
  async () => ({
    status: "ready" as const,
    revision: "r1",
    values: { enabled, idleHours: 24, keepRecent: 0 },
  });

test("one failed archive does not stop the sweep and logs never contain titles", async () => {
  const { api, archived } = fakeApi(
    [row("a", 30), row("b", 30), row("c", 30), row("new", 1)],
    ["b"],
  );
  const lines: string[] = [];
  const result = await sweep(
    api,
    { enabled: true, idleHours: 24, keepRecent: 0 },
    {
      now: () => new Date(NOW),
      log: (line) => lines.push(line),
      signal: new AbortController().signal,
    },
  );
  assert.deepEqual(archived, ["a", "c"]);
  assert.deepEqual(result, {
    checked: 4,
    stale: 3,
    archived: 2,
    failed: 1,
    workspacesStale: 0,
    workspacesArchived: 0,
    workspacesFailed: 0,
  });
  assert.ok(lines.some((line) => line.includes(`titleLength=${SECRET_TITLE.length}`)));
  assert.ok(lines.every((line) => !line.includes(SECRET_TITLE)));
});

test("fresh snapshot that became active is skipped", async () => {
  const { api, archived } = fakeApi([row("a", 30)], [], (agent) =>
    agent ? { ...agent, status: "running" } : agent,
  );
  const result = await sweep(
    api,
    { enabled: true, idleHours: 24, keepRecent: 0 },
    { now: () => new Date(NOW), log: () => {}, signal: new AbortController().signal },
  );
  assert.deepEqual(archived, []);
  assert.deepEqual(result, {
    checked: 1,
    stale: 1,
    archived: 0,
    failed: 0,
    workspacesStale: 0,
    workspacesArchived: 0,
    workspacesFailed: 0,
  });
});

test("timer waits for a hook API, then sweeps are throttled to one per 10 minutes", async () => {
  const { api, archived, lists } = fakeApi([row("a", 30)]);
  let clock = NOW;
  const lines: string[] = [];
  const janitor = createJanitor({
    readSettings: ready(),
    log: (line) => lines.push(line),
    now: () => clock,
  });
  assert.equal(janitor.fromTimer(), undefined);
  assert.equal(lists(), 0);
  await janitor.fromHook(api, "turn_ended");
  assert.deepEqual(archived, ["a"]);
  assert.ok(lines.some((line) => line.startsWith("sweep (turn_ended): archived 1 of 1 stale")));
  clock += SWEEP_THROTTLE_MS - 1;
  assert.equal(janitor.fromHook(api, "created"), undefined);
  clock += 1;
  await janitor.fromTimer();
  // Two lists per sweep plus one fresh list before the first archive.
  assert.equal(lists(), 5);
  janitor.stop();
  clock += SWEEP_THROTTLE_MS;
  assert.equal(janitor.fromTimer(), undefined);
  assert.equal(lists(), 5);
});

test("disabled setting stops all archiving", async () => {
  const { api, archived, lists } = fakeApi([row("a", 30)]);
  const lines: string[] = [];
  const janitor = createJanitor({
    readSettings: ready(false),
    log: (line) => lines.push(line),
    now: () => NOW,
  });
  await janitor.fromHook(api, "turn_ended");
  assert.deepEqual(archived, []);
  assert.equal(lists(), 0);
  assert.deepEqual(lines, ["sweep skipped (turn_ended): disabled"]);
});

test("idle workspaces without live agents or terminals are archived", async () => {
  const live = { ...row("live", 1), workspaceId: "busy" };
  const { api, archivedWorkspaces } = fakeApi(
    [live],
    [],
    undefined,
    [
      workspace("old", 30),
      workspace("busy", 30),
      workspace("recent", 1),
      workspace("pinned", 30, { pinnedAt: new Date(NOW).toISOString() }),
      workspace("worktree", 30, { workspaceKind: "worktree" }),
      workspace("terminal", 30),
    ],
    ["terminal"],
  );
  const result = await sweep(
    api,
    { enabled: true, idleHours: 24, keepRecent: 0 },
    { now: () => new Date(NOW), log: () => {}, signal: new AbortController().signal },
  );
  assert.deepEqual(archivedWorkspaces, ["old"]);
  assert.equal(result.workspacesStale, 2);
  assert.equal(result.workspacesArchived, 1);
});

test("retention is per workspace and retained threads protect their workspaces", async () => {
  const busy = Array.from({ length: 10 }, (_, i) => ({
    ...row(`busy-${i}`, 30 + i),
    workspaceId: "busy",
  }));
  const quiet = Array.from({ length: 2 }, (_, i) => ({
    ...row(`quiet-${i}`, 60 + i),
    workspaceId: "quiet",
  }));
  const { api, archived, archivedWorkspaces } = fakeApi([...busy, ...quiet], [], undefined, [
    workspace("busy", 40),
    workspace("quiet", 70),
  ]);
  const settings = { enabled: true, idleHours: 24, keepRecent: 7 };
  const options = { now: () => new Date(NOW), log: () => {}, signal: new AbortController().signal };
  await sweep(api, settings, options);
  assert.deepEqual(archived, ["busy-7", "busy-8", "busy-9"]);
  assert.deepEqual(archivedWorkspaces, []);
  await sweep(api, settings, options);
  assert.deepEqual(archived, ["busy-7", "busy-8", "busy-9"]);
});

test("fresh activity can move a stale candidate into the retained threads", async () => {
  const { api, archived } = fakeApi([row("recent", 30), row("older", 40)], [], (agent) =>
    agent ? { ...agent, updatedAt: new Date(NOW - 25 * HOUR_MS).toISOString() } : agent,
  );
  await sweep(
    api,
    { enabled: true, idleHours: 24, keepRecent: 1 },
    { now: () => new Date(NOW), log: () => {}, signal: new AbortController().signal },
  );
  assert.deepEqual(archived, []);
});

test("retention protects ancestors from cascading into the newest seven threads", async () => {
  const recent = Array.from({ length: 7 }, (_, i) => ({
    ...row(`recent-${i}`, 30 + i),
    workspaceId: "retained",
    ...(i === 0 ? { labels: { "paseo.parent-agent-id": "parent" } } : {}),
  }));
  const rows: Row[] = [
    ...recent,
    { ...row("grandparent", 80), workspaceId: "retained" },
    {
      ...row("parent", 70),
      workspaceId: "retained",
      labels: { "paseo.parent-agent-id": "grandparent" },
    },
    { ...row("old-parent", 90), workspaceId: "retained" },
    {
      ...row("old-child", 85),
      workspaceId: "retained",
      labels: { "paseo.parent-agent-id": "old-parent" },
    },
  ];
  const { api, archived, archivedWorkspaces } = fakeApi(rows, [], undefined, [
    workspace("retained", 100),
    workspace("old", 100),
  ]);
  const settings = { enabled: true, idleHours: 24, keepRecent: 7 };
  const options = { now: () => new Date(NOW), log: () => {}, signal: new AbortController().signal };
  await sweep(api, settings, options);
  await sweep(api, settings, options);
  assert.deepEqual(archived, ["old-parent", "old-child"]);
  assert.ok(recent.every((agent) => !archived.includes(agent.id)));
  assert.ok(!archivedWorkspaces.includes("retained"));
});
