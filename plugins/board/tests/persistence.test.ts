import assert from "node:assert/strict";
import { mkdtemp, readFile, rmdir, stat, unlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";
import type { PaseoApi } from "@getpaseo/client";
import type { PluginServerContext } from "@getpaseo/plugin/server";
import contribute from "../index.server";
import { createRunPersistence } from "../server/persistence";
import { createRunStore } from "../server/store";

const agent = {
  id: "finished",
  workspaceId: "workspace",
  parentAgentId: null,
  provider: "codex",
  cwd: "/work/project",
  title: "Finished task",
};

test("restores finished, removed, starred, and running cards across plugin instances", async (t) => {
  const dir = await mkdtemp(path.join(tmpdir(), "board-runs-"));
  const file = path.join(dir, "runs.json");
  t.after(async () => {
    await unlink(file);
    await rmdir(dir);
  });
  const persistence = createRunPersistence(file);
  const first = createRunStore(() => "2026-09-24T10:00:00.000Z");
  const oldScope = first.snapshot().observingSince;
  first.start(agent, "done");
  first.end(agent, "done", { kind: "completed" });
  first.setStarred(agent.id, oldScope, true);
  const removed = { ...agent, id: "removed", title: "Removed task" };
  first.start(removed, "gone");
  first.end(removed, "gone", { kind: "completed" });
  first.removeFinished(removed.id, oldScope, first.snapshot().runs[0].endedAt);
  const running = { ...agent, id: "running", title: "Running task" };
  first.start(running, "live");
  first.setStarred(running.id, oldScope, true);
  await persistence.save(first.exportState());

  assert.equal((await stat(file)).mode & 0o777, 0o600);
  assert.equal(JSON.parse(await readFile(file, "utf8")).version, 1);
  const second = createRunStore(() => "2026-09-24T11:00:00.000Z");
  second.restore((await createRunPersistence(file).load())!);
  assert.notEqual(second.snapshot().observingSince, oldScope);
  assert.deepEqual(
    second.snapshot().runs.map((run) => [run.id, run.status, run.starred]),
    [
      ["running", "running", true],
      ["finished", "completed", true],
    ],
  );
  assert.equal(second.setStarred("finished", oldScope, false), false);
  second.end(removed, "gone", { kind: "completed" });
  assert.equal(
    second.snapshot().runs.some((run) => run.id === removed.id),
    false,
  );
  second.reconcile(
    [{ ...running, activeTurn: { turnId: "live", startedAt: "2026-09-24T09:00:00.000Z" } }],
    second.revision,
  );
  assert.equal(second.snapshot().runs[0].status, "running");
  assert.equal(second.snapshot().runs[0].startedAt, "2026-09-24T09:00:00.000Z");
  second.reconcile([], second.revision);
  assert.equal(second.snapshot().runs[0].status, "unknown");
  second.end(running, "live", { kind: "completed" });
  assert.equal(second.snapshot().runs[0].status, "completed");
  await persistence.save(second.exportState());
  const third = createRunStore(() => "2026-09-24T12:00:00.000Z");
  third.restore((await createRunPersistence(file).load())!);
  assert.equal(third.snapshot().runs[0].status, "completed");
  assert.equal(
    third.snapshot().runs.some((run) => run.id === removed.id),
    false,
  );
  third.start(removed, "new");
  assert.equal(
    third.snapshot().runs.some((run) => run.id === removed.id),
    true,
  );
});

test("rejects invalid saved state instead of replacing it", async (t) => {
  const dir = await mkdtemp(path.join(tmpdir(), "board-runs-invalid-"));
  const file = path.join(dir, "runs.json");
  t.after(async () => {
    await unlink(file);
    await rmdir(dir);
  });
  await writeFile(file, '{"version":2,"active":[],"finished":[]}');
  await assert.rejects(createRunPersistence(file).load());
  assert.equal(JSON.parse(await readFile(file, "utf8")).version, 2);
});

test("Board RPC loads saved cards and reconciles missing active runs", async (t) => {
  const home = await mkdtemp(path.join(tmpdir(), "board-server-"));
  const file = path.join(home, "plugin-data/board/runs.json");
  t.after(async () => {
    await unlink(file);
    await rmdir(path.dirname(file));
    await rmdir(path.dirname(path.dirname(file)));
    await rmdir(home);
  });
  const originalHome = process.env.PASEO_HOME;
  process.env.PASEO_HOME = home;
  t.after(() => {
    if (originalHome === undefined) delete process.env.PASEO_HOME;
    else process.env.PASEO_HOME = originalHome;
  });
  const saved = createRunStore();
  saved.end(agent, "done", { kind: "completed" });
  saved.start({ ...agent, id: "interrupted" }, "live");
  await createRunPersistence(file).save(saved.exportState());

  type Handler = (input: unknown, context: { paseo: PaseoApi }) => Promise<unknown>;
  const handlers = new Map<string, Handler>();
  const server = {
    registerSettings() {},
    on() {
      return () => {};
    },
    handle(contract: { name: string }, handler: Handler) {
      handlers.set(contract.name, handler);
    },
  } as unknown as PluginServerContext;
  const paseo = {
    agents: {
      list: async () => ({ entries: [], pageInfo: { hasMore: false } }),
      ref: (id: string) => ({
        refresh: async () =>
          id === "finished"
            ? { agent: { model: "gpt-5.5", effectiveThinkingOptionId: "xhigh" }, project: null }
            : null,
      }),
    },
    providers: {
      listModels: async () => ({
        models: [
          {
            id: "gpt-5.5",
            label: "GPT-5.5",
            thinkingOptions: [{ id: "xhigh", label: "Extra High" }],
          },
        ],
      }),
    },
    projects: { list: async () => ({ projects: [] }) },
  } as unknown as PaseoApi;
  const cleanup = contribute(server);
  try {
    const result = (await handlers.get("board.snapshot")!({}, { paseo })) as {
      runs: { id: string; status: string; model?: string | null; effort?: string | null }[];
    };
    assert.deepEqual(
      result.runs.map((run) => [run.id, run.status, run.model, run.effort]),
      [
        ["interrupted", "unknown", null, null],
        ["finished", "completed", "GPT-5.5", "Extra High"],
      ],
    );
    const persisted = (await createRunPersistence(file).load())!;
    assert.equal(persisted.finished[0].status, "unknown");
  } finally {
    await cleanup();
  }
});
