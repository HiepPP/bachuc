import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdtemp, readFile, rm, stat, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { run } from "../server/git";
import {
  createTurnDiffTracker,
  dropTrees,
  keepTrees,
  readFileDiff,
  turnRefs,
} from "../server/turn-diff";
import { createTurnJournal, JOURNAL_MAX_ENTRIES } from "../server/turn-journal";
import type { TurnHistoryEntry } from "../shared/turn-diff";

const DAY = 24 * 60 * 60 * 1000;
const NOW = Date.parse("2026-09-24T12:00:00Z");

function entry(key: string, agentId: string, endedAt: number): TurnHistoryEntry {
  return {
    key,
    agentId,
    endedAt: new Date(endedAt).toISOString(),
    diff: { fileCount: 1, added: 1, deleted: 0, files: [], commits: [], shared: false },
  };
}

async function withDir(body: (dir: string) => Promise<void>) {
  const dir = await mkdtemp(join(tmpdir(), "thread-branch-journal-"));
  try {
    await body(dir);
  } finally {
    await rm(dir, { recursive: true, force: true });
  }
}

test("lists one agent's turns newest first and survives a new journal instance", async () => {
  await withDir(async (dir) => {
    const file = join(dir, "data", "turn-diffs.jsonl");
    const journal = createTurnJournal(file, () => NOW);
    await journal.add(entry("1", "a1", NOW - 3000));
    await journal.add(entry("2", "a2", NOW - 2000));
    await journal.add(entry("3", "a1", NOW - 1000));
    // A new instance stands in for a daemon restart: nothing is kept in memory.
    const reopened = createTurnJournal(file, () => NOW);
    assert.deepEqual(
      (await reopened.list("a1")).map((item) => item.key),
      ["3", "1"],
    );
    assert.equal((await stat(file)).mode & 0o777, 0o600);
  });
});

test("drops turns older than 30 days and returns them", async () => {
  await withDir(async (dir) => {
    const file = join(dir, "turn-diffs.jsonl");
    await writeFile(file, `${JSON.stringify(entry("old", "a1", NOW - 31 * DAY))}\n{"torn":\n`);
    const journal = createTurnJournal(file, () => NOW);
    const expired = await journal.add(entry("new", "a1", NOW));
    assert.deepEqual(
      expired.map((item) => item.key),
      ["old"],
    );
    assert.deepEqual(
      (await journal.list("a1")).map((item) => item.key),
      ["new"],
    );
    assert.equal((await readFile(file, "utf8")).trim().split("\n").length, 1);
  });
});

test("keeps at most 2,000 turns", async () => {
  await withDir(async (dir) => {
    const file = join(dir, "turn-diffs.jsonl");
    const lines = Array.from(
      { length: JOURNAL_MAX_ENTRIES },
      (_, index) => `${JSON.stringify(entry(`k${index}`, "a1", NOW - 1000))}\n`,
    );
    await writeFile(file, lines.join(""));
    const expired = await createTurnJournal(file, () => NOW).add(entry("last", "a1", NOW));
    assert.deepEqual(
      expired.map((item) => item.key),
      ["k0"],
    );
  });
});

test("git refs keep a turn's snapshots through git gc until they are dropped", async () => {
  await withDir(async (root) => {
    const gitSync = (...args: string[]) =>
      execFileSync("git", ["-c", "user.name=t", "-c", "user.email=t@t", ...args], {
        cwd: root,
      }).toString();
    const git = (args: readonly string[], cwd: string) => run("git", args, cwd);
    gitSync("init", "-q", "-b", "main");
    await writeFile(join(root, "a.txt"), "one\n");
    gitSync("add", ".");
    gitSync("commit", "-q", "-m", "init");
    const tracker = createTurnDiffTracker();
    const agent = { id: "a1", cwd: root, workspaceId: "w1" };
    await tracker.started(agent, "t1");
    await writeFile(join(root, "a.txt"), "one\nturn only\n");
    const source = (await tracker.ended(agent, "t1"))?.diff.source;
    assert.ok(source);

    const key = "1727000000000-foreground-turn:1";
    await keepTrees(git, root, key, source.from, source.to);
    assert.match(
      gitSync("for-each-ref", "refs/thread-branch"),
      /turns\/1727000000000-foreground-turn_1\/to/,
    );
    assert.equal(gitSync("branch", "--list").trim(), "* main");
    // Reset the file so the snapshot trees are reachable only through the refs.
    gitSync("checkout", "--", "a.txt");
    gitSync("gc", "-q", "--prune=now");
    const kept = await readFileDiff(git, { ...source, path: "a.txt" });
    assert.match(kept.diff, /\+turn only/);

    await dropTrees(git, root, key);
    assert.equal(gitSync("for-each-ref", turnRefs(key).to).trim(), "");
    gitSync("gc", "-q", "--prune=now");
    await assert.rejects(readFileDiff(git, { ...source, path: "a.txt" }));
  });
});
