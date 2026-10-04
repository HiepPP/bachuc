import assert from "node:assert/strict";
import test from "node:test";
import type { PaseoApi } from "@getpaseo/client";
import { RESUME_SETTLE_MS, SWEEP_THROTTLE_MS, createJanitor, sweep } from "../server/janitor";

const HOUR_MS = 60 * 60 * 1000;
const NOW = Date.parse("2026-09-23T12:00:00.000Z");
const SECRET_TITLE = "private prompt title";
const SETTINGS = { enabled: true, idleHours: 24 };

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
  options: {
    failIds?: string[];
    workspaces?: Workspace[];
    terminalWorkspaceIds?: string[];
    beforeList?: (count: number) => void;
  } = {},
) {
  const closed: string[] = [];
  const archived: string[] = [];
  const archivedWorkspaces: string[] = [];
  let lists = 0;
  const api = {
    workspaces: {
      list: async () => ({ entries: options.workspaces ?? [], pageInfo: { hasMore: false } }),
      ref: (id: string) => ({
        refresh: async () => options.workspaces?.find((entry) => entry.id === id) ?? null,
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
        entries: options.terminalWorkspaceIds?.includes(workspaceId) ? [{ id: "t1" }] : [],
      }),
    },
    agents: {
      list: async () => {
        lists += 1;
        options.beforeList?.(lists);
        return { entries: rows.map((agent) => ({ agent })), pageInfo: { hasMore: false } };
      },
      ref: (id: string) => ({
        closeRuntime: async () => {
          if (options.failIds?.includes(id)) throw new Error("daemon refused");
          closed.push(id);
          const agent = rows.find((entry) => entry.id === id);
          if (agent) agent.status = "closed";
        },
        archive: async () => {
          archived.push(id);
        },
      }),
    },
  } as unknown as PaseoApi;
  return { api, closed, archived, archivedWorkspaces, lists: () => lists };
}

const ready =
  (enabled = true) =>
  async () => ({
    status: "ready" as const,
    revision: "r1",
    values: { enabled, idleHours: 24 },
  });

const sweepOptions = (log: (line: string) => void = () => {}) => ({
  now: () => new Date(NOW),
  log,
  signal: new AbortController().signal,
});

// Fake API calls resolve as microtasks, so a sweep finishes before the next macrotask.
const settle = () => new Promise((resolve) => setImmediate(resolve));

test("closes stale runtimes without archiving, and one failure does not stop the sweep", async () => {
  const { api, closed, archived } = fakeApi(
    [row("a", 30), row("b", 30), row("c", 30), row("new", 1)],
    { failIds: ["b"] },
  );
  const lines: string[] = [];
  const result = await sweep(
    api,
    SETTINGS,
    sweepOptions((line) => lines.push(line)),
  );
  assert.deepEqual(closed, ["a", "c"]);
  assert.deepEqual(archived, []);
  assert.deepEqual(result, {
    checked: 4,
    stale: 3,
    closed: 2,
    failed: 1,
    workspacesStale: 0,
    workspacesArchived: 0,
    workspacesFailed: 0,
  });
  assert.ok(lines.includes("closed a"));
  assert.ok(lines.some((line) => line.startsWith("close failed b: daemon refused")));
  assert.ok(lines.every((line) => !line.includes(SECRET_TITLE)));
});

test("an agent that became active before its close is skipped", async () => {
  const rows = [row("a", 30)];
  const { api, closed } = fakeApi(rows, {
    beforeList: (count) => {
      if (count === 2) rows[0].status = "running";
    },
  });
  const result = await sweep(api, SETTINGS, sweepOptions());
  assert.deepEqual(closed, []);
  assert.equal(result.stale, 1);
  assert.equal(result.closed, 0);
});

test("timer waits for a hook API, then hook sweeps are throttled to one per 10 minutes", async () => {
  const { api, closed, lists } = fakeApi([row("a", 30)]);
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
  assert.deepEqual(closed, ["a"]);
  assert.ok(lines.some((line) => line.startsWith("sweep (turn_ended): closed 1 of 1 stale")));
  clock += SWEEP_THROTTLE_MS - 1;
  assert.equal(janitor.fromHook(api, "created"), undefined);
  clock += 1;
  await janitor.fromTimer();
  // Two lists per sweep plus one fresh list before the first close.
  assert.equal(lists(), 5);
  janitor.stop();
  clock += SWEEP_THROTTLE_MS;
  assert.equal(janitor.fromTimer(), undefined);
  assert.equal(lists(), 5);
});

test("resumes schedule one sweep after the burst settles, even inside the throttle", async (t) => {
  t.mock.timers.enable({ apis: ["setTimeout"] });
  const rows = [row("a", 30)];
  const { api, closed } = fakeApi(rows);
  const lines: string[] = [];
  const janitor = createJanitor({
    readSettings: ready(),
    log: (line) => lines.push(line),
    now: () => NOW,
  });
  await janitor.fromHook(api, "turn_ended");
  assert.deepEqual(closed, ["a"]);
  // A reconnect resumes threads one after another.
  rows.push(row("b", 30), row("c", 30));
  janitor.afterResume(api);
  t.mock.timers.tick(RESUME_SETTLE_MS - 1);
  janitor.afterResume(api);
  t.mock.timers.tick(RESUME_SETTLE_MS - 1);
  await settle();
  assert.deepEqual(closed, ["a"]);
  t.mock.timers.tick(1);
  await settle();
  assert.deepEqual(closed, ["a", "b", "c"]);
  assert.ok(lines.some((line) => line.startsWith("sweep (resume): closed 2 of 2 stale")));
});

test("stop cancels a pending resume sweep", async (t) => {
  t.mock.timers.enable({ apis: ["setTimeout"] });
  const { api, lists } = fakeApi([row("a", 30)]);
  const janitor = createJanitor({ readSettings: ready(), log: () => {}, now: () => NOW });
  janitor.afterResume(api);
  janitor.stop();
  t.mock.timers.tick(RESUME_SETTLE_MS);
  await settle();
  assert.equal(lists(), 0);
});

test("disabled setting stops all closing", async () => {
  const { api, closed, lists } = fakeApi([row("a", 30)]);
  const lines: string[] = [];
  const janitor = createJanitor({
    readSettings: ready(false),
    log: (line) => lines.push(line),
    now: () => NOW,
  });
  await janitor.fromHook(api, "turn_ended");
  assert.deepEqual(closed, []);
  assert.equal(lists(), 0);
  assert.deepEqual(lines, ["sweep skipped (turn_ended): disabled"]);
});

test("idle workspaces without live agents or terminals are archived", async () => {
  const live = { ...row("live", 1), workspaceId: "busy" };
  const { api, archivedWorkspaces } = fakeApi([live], {
    workspaces: [
      workspace("old", 30),
      workspace("busy", 30),
      workspace("recent", 1),
      workspace("pinned", 30, { pinnedAt: new Date(NOW).toISOString() }),
      workspace("worktree", 30, { workspaceKind: "worktree" }),
      workspace("terminal", 30),
    ],
    terminalWorkspaceIds: ["terminal"],
  });
  const result = await sweep(api, SETTINGS, sweepOptions());
  assert.deepEqual(archivedWorkspaces, ["old"]);
  assert.equal(result.workspacesStale, 2);
  assert.equal(result.workspacesArchived, 1);
});

test("closed threads keep their workspaces", async () => {
  const rows = [{ ...row("old", 30), workspaceId: "kept" }];
  const { api, closed, archivedWorkspaces } = fakeApi(rows, {
    workspaces: [workspace("kept", 30)],
  });
  await sweep(api, SETTINGS, sweepOptions());
  assert.deepEqual(closed, ["old"]);
  assert.deepEqual(archivedWorkspaces, []);
});
