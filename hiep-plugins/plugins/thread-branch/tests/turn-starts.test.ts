import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdtemp, readdir, rm, utimes, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { run } from "../server/git";
import { createTurnDiffTracker, type Git } from "../server/turn-diff";
import { createStartStore } from "../server/turn-starts";

const git: Git = (args, cwd, env) => run("git", args, cwd, undefined, undefined, env);

async function withRepo(body: (root: string, starts: string) => Promise<void>) {
  const root = await mkdtemp(join(tmpdir(), "thread-branch-starts-"));
  const starts = await mkdtemp(join(tmpdir(), "thread-branch-starts-data-"));
  try {
    const gitSync = (...args: string[]) =>
      execFileSync("git", ["-c", "user.name=t", "-c", "user.email=t@t", ...args], { cwd: root });
    gitSync("init", "-q", "-b", "main");
    await writeFile(join(root, "a.txt"), "one\n");
    await writeFile(join(root, "b.txt"), "b\n");
    gitSync("add", ".");
    gitSync("commit", "-q", "-m", "init");
    await body(root, starts);
  } finally {
    await rm(root, { recursive: true, force: true });
    await rm(starts, { recursive: true, force: true });
  }
}

test("a plugin reload mid-turn still gives the card for only that turn", async () => {
  await withRepo(async (root, dir) => {
    await writeFile(join(root, "b.txt"), "b\ndirty before\n");
    const agent = { id: "a1", cwd: root, workspaceId: "w1" };
    await createTurnDiffTracker(git, createStartStore(dir)).started(agent, "t1");
    assert.deepEqual(await readdir(dir), ["a1.json"]);

    // A new tracker stands in for the reloaded plugin: its memory is empty.
    await writeFile(join(root, "a.txt"), "one\ntwo\n");
    const row = await createTurnDiffTracker(git, createStartStore(dir)).ended(agent, "t1");
    assert.deepEqual(row?.diff.files, [{ path: "a.txt", added: 1, deleted: 0 }]);
    assert.ok(row?.diff.source);
    assert.deepEqual(await readdir(dir), []);
  });
});

test("removes the saved start when the turn ends normally", async () => {
  await withRepo(async (root, dir) => {
    const tracker = createTurnDiffTracker(git, createStartStore(dir));
    const agent = { id: "a1", cwd: root };
    await tracker.started(agent, "t1");
    await writeFile(join(root, "a.txt"), "changed\n");
    assert.ok(await tracker.ended(agent, "t1"));
    assert.deepEqual(await readdir(dir), []);
  });
});

test("ignores a saved start for another turn, another folder, or one over a day old", async () => {
  await withRepo(async (root, dir) => {
    const agent = { id: "a1", cwd: root };
    const fresh = () => createTurnDiffTracker(git, createStartStore(dir));

    await fresh().started(agent, "t1");
    await writeFile(join(root, "a.txt"), "x\n");
    assert.equal(await fresh().ended(agent, "t2"), null);

    await fresh().started(agent, "t3");
    assert.equal(await fresh().ended({ id: "a1", cwd: join(root, "..") }, "t3"), null);

    const now = Date.now();
    await createTurnDiffTracker(
      git,
      createStartStore(dir, () => now),
    ).started(agent, "t4");
    const later = createStartStore(dir, () => now + 25 * 60 * 60 * 1000);
    assert.equal(await createTurnDiffTracker(git, later).ended(agent, "t4"), null);
  });
});

test("adds no card when a restored turn cannot take its end snapshot", async () => {
  await withRepo(async (root, dir) => {
    await writeFile(join(root, "b.txt"), "b\ndirty before\n");
    const agent = { id: "a1", cwd: root };
    await createTurnDiffTracker(git, createStartStore(dir)).started(agent, "t1");
    await writeFile(join(root, "a.txt"), "one\ntwo\n");
    // Without an end tree, falling back to numstat would list b.txt, a change from before the turn.
    const noTree: Git = (args, cwd, env) =>
      args[0] === "write-tree"
        ? Promise.resolve({ code: 1, stdout: "", stderr: "", enoent: false })
        : git(args, cwd, env);
    assert.equal(
      await createTurnDiffTracker(noTree, createStartStore(dir)).ended(agent, "t1"),
      null,
    );
  });
});

test("prunes saved starts older than a day on the first save", async () => {
  await withRepo(async (root, dir) => {
    const old = join(dir, "gone.json");
    await writeFile(old, "{}");
    const past = new Date(Date.now() - 2 * 24 * 60 * 60 * 1000);
    await utimes(old, past, past);
    await createTurnDiffTracker(git, createStartStore(dir)).started({ id: "a1", cwd: root }, "t1");
    assert.deepEqual(await readdir(dir), ["a1.json"]);
  });
});
