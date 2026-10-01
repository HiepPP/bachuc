import assert from "node:assert/strict";
import test from "node:test";
import {
  createBranchReader,
  parseAheadBehind,
  parsePullRequest,
  parseRemoteUrl,
  run,
  sumNumstat,
} from "../server/git";

test("parses rev-list left-right counts as behind/ahead", () => {
  assert.deepEqual(parseAheadBehind("1\t3\n"), { behind: 1, ahead: 3 });
  assert.equal(parseAheadBehind("fatal: no upstream"), null);
});

test("sums numstat lines and skips binary counts", () => {
  assert.deepEqual(sumNumstat("3\t1\ta.ts\n-\t-\timg.png\n10\t0\tb.ts\n"), {
    added: 13,
    deleted: 1,
  });
  assert.deepEqual(sumNumstat(""), { added: 0, deleted: 0 });
});

test("parses gh pr view JSON and rejects malformed output", () => {
  assert.deepEqual(
    parsePullRequest('{"number":42,"url":"https://github.com/o/r/pull/42","state":"OPEN"}'),
    {
      number: 42,
      url: "https://github.com/o/r/pull/42",
      state: "OPEN",
    },
  );
  assert.equal(parsePullRequest("not json"), null);
  assert.equal(parsePullRequest('{"url":"x"}'), null);
});

test("turns origin remotes into browser URLs", () => {
  assert.equal(parseRemoteUrl("git@github.com:o/r.git\n"), "https://github.com/o/r");
  assert.equal(parseRemoteUrl("https://github.com/o/r.git"), "https://github.com/o/r");
  assert.equal(parseRemoteUrl("https://user@gitlab.com/g/s/r"), "https://gitlab.com/g/s/r");
  assert.equal(parseRemoteUrl("ssh://git@github.com:22/o/r.git"), "https://github.com/o/r");
  assert.equal(parseRemoteUrl("/local/path.git"), null);
  assert.equal(parseRemoteUrl(""), null);
});

test("reads this repository and reports a non-git directory without throwing", async () => {
  const reader = createBranchReader();
  const here = await reader.get(process.cwd());
  assert.equal(here.repo, true);
  assert.ok(here.branch || here.detached);
  assert.ok(here.sha);
  assert.equal(here.remoteUrl, "https://github.com/HiepPP/paseo");
  const outside = await reader.get("/");
  assert.equal(outside.repo, false);
  assert.equal(outside.pr, null);
});

test("fetch updates the behind count without committing, merging, or pushing", async () => {
  const { execFileSync } = await import("node:child_process");
  const { mkdtemp, rm } = await import("node:fs/promises");
  const { tmpdir } = await import("node:os");
  const { join } = await import("node:path");
  const root = await mkdtemp(join(tmpdir(), "thread-branch-"));
  const git = (cwd: string, ...args: string[]) =>
    execFileSync("git", ["-c", "user.name=t", "-c", "user.email=t@t", ...args], { cwd });
  try {
    git(root, "init", "-q", "--bare", "-b", "main", "remote.git");
    git(root, "clone", "-q", "remote.git", "a");
    git(join(root, "a"), "commit", "-q", "--allow-empty", "-m", "one");
    git(join(root, "a"), "push", "-q", "-u", "origin", "main");
    git(root, "clone", "-q", "remote.git", "b");
    git(join(root, "b"), "commit", "-q", "--allow-empty", "-m", "two");
    git(join(root, "b"), "push", "-q");

    const reader = createBranchReader();
    const a = join(root, "a");
    assert.equal((await reader.get(a)).behind, 0);
    const head = () => git(a, "rev-parse", "HEAD").toString().trim();
    const remoteMain = () => git(root, "--git-dir=remote.git", "rev-parse", "main").toString();
    const headBefore = head();
    const remoteBefore = remoteMain();
    const fetched = await reader.get(a, true, true);
    assert.equal(fetched.behind, 1);
    assert.equal(fetched.ahead, 0);
    // Fetch only moves remote-tracking refs: no merge into HEAD, nothing pushed, no new commit.
    assert.equal(head(), headBefore);
    assert.equal(remoteMain(), remoteBefore);
    assert.equal(git(a, "status", "--porcelain").toString(), "");

    git(a, "remote", "set-url", "origin", join(root, "missing.git"));
    await assert.rejects(reader.get(a, true, true), /git fetch failed/);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("a command killed by its timeout is not reported as success", async () => {
  const result = await run("sh", ["-c", "echo partial; sleep 5"], "/tmp", 300);
  assert.notEqual(result.code, 0);
  assert.equal(result.stdout, "partial\n");
});

test("counts uncommitted tracked files and lines only when the work tree is dirty", async () => {
  const { execFileSync } = await import("node:child_process");
  const { mkdtemp, rm, writeFile } = await import("node:fs/promises");
  const { tmpdir } = await import("node:os");
  const { join } = await import("node:path");
  const root = await mkdtemp(join(tmpdir(), "thread-branch-"));
  const git = (...args: string[]) =>
    execFileSync("git", ["-c", "user.name=t", "-c", "user.email=t@t", ...args], { cwd: root });
  try {
    git("init", "-q", "-b", "main");
    await writeFile(join(root, "a.txt"), "1\n2\n3\n");
    await writeFile(join(root, "b.txt"), "x\n");
    git("add", ".");
    git("commit", "-q", "-m", "init");
    const reader = createBranchReader();
    await writeFile(join(root, "untracked.txt"), "ignored\n");
    const clean = await reader.get(root, true);
    assert.equal(clean.dirty, false);
    assert.equal(clean.changes, null);

    await writeFile(join(root, "a.txt"), "1\nchanged\n3\n4\n");
    await writeFile(join(root, "b.txt"), "x\ny\n");
    git("add", "b.txt");
    const dirty = await reader.get(root, true);
    assert.equal(dirty.dirty, true);
    // Staged and unstaged edits both count against HEAD; the untracked file does not.
    assert.deepEqual(dirty.changes, { files: 2, added: 3, deleted: 1 });
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
