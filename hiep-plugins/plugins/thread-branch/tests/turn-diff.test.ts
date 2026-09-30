import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdtemp, rm, unlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import {
  createTurnDiffTracker,
  parseNumstat,
  defaultGit,
  readFileDiff,
  readFileImage,
  snapshotTree,
} from "../server/turn-diff";
import { run } from "../server/git";
import { imageType, parseUnifiedDiff, turnDiffHeader } from "../shared/turn-diff";

const agent = (cwd: string, id = "a1") => ({ id, cwd });

async function withRepo(body: (root: string, git: (...args: string[]) => string) => Promise<void>) {
  const root = await mkdtemp(join(tmpdir(), "thread-branch-diff-"));
  const git = (...args: string[]) =>
    execFileSync("git", ["-c", "user.name=t", "-c", "user.email=t@t", ...args], {
      cwd: root,
    }).toString();
  try {
    git("init", "-q", "-b", "main");
    await writeFile(join(root, "a.txt"), "one\ntwo\n");
    await writeFile(join(root, "b.txt"), "b\n");
    await writeFile(join(root, "gone.txt"), "x\ny\nz\n");
    git("add", ".");
    git("commit", "-q", "-m", "init");
    await body(root, git);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
}

test("parses numstat records with binary markers", () => {
  const parsed = parseNumstat("3\t1\ta.txt\0-\t-\timg.png\0");
  assert.deepEqual(parsed.get("a.txt"), { added: 3, deleted: 1 });
  assert.deepEqual(parsed.get("img.png"), { added: null, deleted: null });
});

test("lists edited, new untracked, deleted, and binary files", async () => {
  await withRepo(async (root) => {
    const tracker = createTurnDiffTracker();
    await tracker.started(agent(root), "t1");
    await writeFile(join(root, "a.txt"), "one\nTWO\nthree\n");
    await writeFile(join(root, "new.txt"), "n1\nn2");
    await writeFile(join(root, "bin.dat"), Buffer.from([0, 1, 2, 0]));
    await unlink(join(root, "gone.txt"));
    const row = await tracker.ended(agent(root), "t1");
    assert.ok(row);
    assert.match(row.rowId, /^turn-diff:\d+-t1$/);
    assert.deepEqual(row.diff.files, [
      { path: "a.txt", added: 2, deleted: 1 },
      { path: "bin.dat", added: null, deleted: null },
      { path: "gone.txt", added: 0, deleted: 3 },
      { path: "new.txt", added: 2, deleted: 0 },
    ]);
    assert.equal(row.diff.fileCount, 4);
    assert.equal(row.diff.added, 4);
    assert.equal(row.diff.deleted, 4);
    assert.deepEqual(row.diff.commits, []);
    assert.equal(row.diff.shared, false);
    assert.equal(turnDiffHeader(row.diff), "4 files changed +4 -4");
  });
});

test("counts only what changed during the turn when the tree was already dirty", async () => {
  await withRepo(async (root) => {
    await writeFile(join(root, "a.txt"), "one\ntwo\nthree\n");
    await writeFile(join(root, "old.txt"), "left over\n");
    const tracker = createTurnDiffTracker();
    await tracker.started(agent(root), "t1");
    await writeFile(join(root, "b.txt"), "b\nc\n");
    const row = await tracker.ended(agent(root), "t1");
    assert.deepEqual(row?.diff.files, [{ path: "b.txt", added: 1, deleted: 0 }]);
  });
});

test("lists edits inside already-dirty files and files untracked before the turn", async () => {
  await withRepo(async (root) => {
    await writeFile(join(root, "a.txt"), "one\ntwo\nB\nC\n");
    await writeFile(join(root, "u.txt"), "u1\n");
    const tracker = createTurnDiffTracker();
    await tracker.started(agent(root), "t1");
    // Same numstat against HEAD as before the turn, but the content changed.
    await writeFile(join(root, "a.txt"), "one\ntwo\nB2\nC\n");
    await writeFile(join(root, "u.txt"), "u1\nu2\n");
    const row = await tracker.ended(agent(root), "t1");
    assert.deepEqual(row?.diff.files, [
      { path: "a.txt", added: 1, deleted: 1 },
      { path: "u.txt", added: 1, deleted: 0 },
    ]);
  });
});

test("lists commits made during the turn with their files", async () => {
  await withRepo(async (root, git) => {
    const tracker = createTurnDiffTracker();
    await tracker.started(agent(root), "t1");
    await writeFile(join(root, "b.txt"), "b\nmore\n");
    git("commit", "-q", "-am", "feat: extend b");
    git("commit", "-q", "--allow-empty", "-m", "chore: empty");
    const row = await tracker.ended(agent(root), "t1");
    assert.ok(row);
    assert.deepEqual(
      row.diff.commits.map((commit) => commit.subject),
      ["chore: empty", "feat: extend b"],
    );
    assert.deepEqual(row.diff.files, [{ path: "b.txt", added: 1, deleted: 0 }]);
    assert.equal(turnDiffHeader(row.diff), "1 file changed +1 -0 · 2 commits");
  });
});

test("adds no row for no change, a missing start snapshot, or a non-git folder", async () => {
  await withRepo(async (root) => {
    const tracker = createTurnDiffTracker();
    await tracker.started(agent(root), "t1");
    assert.equal(await tracker.ended(agent(root), "t1"), null);

    await writeFile(join(root, "a.txt"), "changed\n");
    assert.equal(await tracker.ended(agent(root), "t2"), null);
  });
  const plain = await mkdtemp(join(tmpdir(), "thread-branch-plain-"));
  try {
    const tracker = createTurnDiffTracker();
    await tracker.started(agent(plain), "t1");
    await writeFile(join(plain, "x.txt"), "x\n");
    assert.equal(await tracker.ended(agent(plain), "t1"), null);
  } finally {
    await rm(plain, { recursive: true, force: true });
  }
});

test("flags a turn that overlapped another agent in the same folder", async () => {
  await withRepo(async (root) => {
    const tracker = createTurnDiffTracker();
    await tracker.started(agent(root, "a1"), "t1");
    await tracker.started(agent(root, "a2"), "t2");
    await writeFile(join(root, "a.txt"), "x\n");
    const second = await tracker.ended(agent(root, "a2"), "t2");
    const first = await tracker.ended(agent(root, "a1"), "t1");
    assert.equal(second?.diff.shared, true);
    assert.equal(first?.diff.shared, true);

    await tracker.started(agent(root, "a1"), "t3");
    await writeFile(join(root, "b.txt"), "y\n");
    assert.equal((await tracker.ended(agent(root, "a1"), "t3"))?.diff.shared, false);
  });
});

test("keeps 20 files in the row and counts the rest", async () => {
  await withRepo(async (root) => {
    const tracker = createTurnDiffTracker();
    await tracker.started(agent(root), "t1");
    for (let index = 0; index < 25; index++)
      await writeFile(join(root, `f${String(index).padStart(2, "0")}.txt`), "line\n");
    const row = await tracker.ended(agent(root), "t1");
    assert.ok(row);
    assert.equal(row.diff.fileCount, 25);
    assert.equal(row.diff.files.length, 20);
    assert.equal(row.diff.added, 25);
  });
});

const git = (args: readonly string[], cwd: string) => run("git", args, cwd);

test("opens the diff of one file with only the turn's own changes", async () => {
  await withRepo(async (root, gitSync) => {
    await writeFile(join(root, "a.txt"), "one\ntwo\nbefore\n");
    gitSync("add", "a.txt");
    const tracker = createTurnDiffTracker();
    const withWorkspace = { ...agent(root), workspaceId: "w1" };
    await tracker.started(withWorkspace, "t1");
    await writeFile(join(root, "a.txt"), "one\ntwo\nbefore\nduring\n");
    await writeFile(join(root, "new.txt"), "n1\nn2\n");
    await writeFile(join(root, "b.txt"), "b\nother\n");
    const row = await tracker.ended(withWorkspace, "t1");
    const source = row?.diff.source;
    assert.ok(source);
    assert.equal(source.workspaceId, "w1");

    const edited = await readFileDiff(git, { ...source, path: "a.txt" });
    const editedRows = parseUnifiedDiff(edited.diff).rows;
    assert.deepEqual(
      editedRows.map((row) => row.type),
      ["hunk", "context", "context", "context", "add"],
    );
    assert.deepEqual(editedRows.at(-1), { type: "add", text: "during", oldLine: null, newLine: 4 });
    assert.equal(edited.truncated, false);
    const created = await readFileDiff(git, { ...source, path: "new.txt" });
    const createdDiff = parseUnifiedDiff(created.diff);
    assert.equal(createdDiff.status, "added");
    assert.deepEqual(
      createdDiff.rows.slice(1).map((row) => row.text),
      ["n1", "n2"],
    );

    // The snapshot uses a copy of the index: staging stays as it was and new.txt stays untracked.
    assert.equal(gitSync("diff", "--cached", "--name-only").trim(), "a.txt");
    assert.match(gitSync("status", "--porcelain"), /^\?\? new\.txt$/m);
  });
});

test("never stages files in the real index, even with a runner that drops env", async () => {
  await withRepo(async (root, gitSync) => {
    await writeFile(join(root, "new.txt"), "n\n");
    await writeFile(join(root, "a.txt"), "edited\n");
    const dropsEnv = (args: readonly string[], cwd: string) => run("git", args, cwd);
    assert.equal(await snapshotTree(dropsEnv, root), null);
    assert.equal((await snapshotTree(defaultGit, root)) !== null, true);
    assert.equal(gitSync("diff", "--cached", "--name-only").trim(), "");
  });
});

test("gives a new row id to a later turn that reuses a turn id", async () => {
  await withRepo(async (root) => {
    const tracker = createTurnDiffTracker();
    await tracker.started(agent(root), "foreground-turn-1");
    await writeFile(join(root, "a.txt"), "first\n");
    const first = await tracker.ended(agent(root), "foreground-turn-1");
    // Paseo restarts turn ids at 1 when it reloads a session.
    await new Promise((resolve) => setTimeout(resolve, 5));
    await tracker.started(agent(root), "foreground-turn-1");
    await writeFile(join(root, "b.txt"), "second\n");
    const second = await tracker.ended(agent(root), "foreground-turn-1");
    assert.ok(first && second);
    assert.notEqual(first.rowId, second.rowId);
  });
});

test("shows the start of a diff larger than the runner's buffer", async () => {
  await withRepo(async (root, gitSync) => {
    const lines = (tag: string) =>
      Array.from(
        { length: 20_000 },
        (_, i) => `"${tag}-dependency-${i}": "registry/pkg-${i}-1.0.${i}.tgz",`,
      ).join("\n");
    await writeFile(join(root, "package-lock.json"), `${lines("old")}\n`);
    gitSync("add", "package-lock.json");
    gitSync("commit", "-q", "-m", "lock");
    const tracker = createTurnDiffTracker();
    const withWorkspace = { ...agent(root), workspaceId: "w1" };
    await tracker.started(withWorkspace, "t1");
    await writeFile(join(root, "package-lock.json"), `${lines("new")}\n`);
    const source = (await tracker.ended(withWorkspace, "t1"))?.diff.source;
    assert.ok(source);
    // Over the runner's 1 MiB buffer, so git is killed mid-output.
    const result = await readFileDiff(git, { ...source, path: "package-lock.json" });
    assert.equal(result.truncated, true);
    assert.equal(result.diff.length, 200_000);
    assert.match(result.diff, /^diff --git a\/package-lock\.json/);
  });
});

test("keeps large untracked files out of the snapshot and lists them as large", async () => {
  await withRepo(async (root, gitSync) => {
    const big = Buffer.alloc(2 * 1024 * 1024, "x");
    await writeFile(join(root, "a.txt"), `${"tracked edit ".repeat(200_000)}\n`);
    await unlink(join(root, "gone.txt"));
    const tracker = createTurnDiffTracker();
    const withWorkspace = { ...agent(root), workspaceId: "w1" };
    await tracker.started(withWorkspace, "t1");
    await writeFile(join(root, "big.bin"), big);
    await writeFile(join(root, "small note.txt"), "small\n");
    await writeFile(join(root, "odd\nname*.txt"), "odd\n");
    await writeFile(join(root, "b.txt"), "b\nmore\n");
    await writeFile(join(root, "a.txt"), `${"tracked edit ".repeat(200_001)}\n`);
    const row = await tracker.ended(withWorkspace, "t1");
    const source = row?.diff.source;
    assert.ok(row && source);
    assert.deepEqual(
      row.diff.files.map((file) => [file.path, file.large === true]),
      [
        ["a.txt", false],
        ["b.txt", false],
        ["big.bin", true],
        ["odd\nname*.txt", false],
        ["small note.txt", false],
      ],
    );
    assert.throws(() => gitSync("cat-file", "-e", `${source.to}:big.bin`));
    gitSync("cat-file", "-e", `${source.to}:small note.txt`);
    gitSync("cat-file", "-e", `${source.to}:odd\nname*.txt`);
    // The large tracked edit and the deletion made before the turn stay in both trees.
    gitSync("cat-file", "-e", `${source.to}:a.txt`);
    assert.throws(() => gitSync("cat-file", "-e", `${source.to}:gone.txt`));
    assert.equal(gitSync("diff", "--cached", "--name-only").trim(), "");
  });
});

test("does not list a large untracked file that existed before the turn", async () => {
  await withRepo(async (root) => {
    await writeFile(join(root, "big.bin"), Buffer.alloc(2 * 1024 * 1024, "x"));
    const tracker = createTurnDiffTracker();
    const withWorkspace = { ...agent(root), workspaceId: "w1" };
    await tracker.started(withWorkspace, "t1");
    await writeFile(join(root, "b.txt"), "b\nmore\n");
    const row = await tracker.ended(withWorkspace, "t1");
    assert.deepEqual(
      row?.diff.files.map((file) => file.path),
      ["b.txt"],
    );
  });
});

test("adds no diff source without a workspace", async () => {
  await withRepo(async (root) => {
    const tracker = createTurnDiffTracker();
    await tracker.started(agent(root), "t1");
    await writeFile(join(root, "a.txt"), "x\n");
    const row = await tracker.ended(agent(root), "t1");
    assert.ok(row);
    assert.equal(row.diff.source, undefined);
  });
});

test("numbers old and new lines and reads the file status", () => {
  const parsed = parseUnifiedDiff(
    [
      "diff --git a/x b/x",
      "--- a/x",
      "+++ b/x",
      "@@ -3,3 +3,3 @@ fn",
      " keep",
      "-old",
      "+new",
      "\\ No newline at end of file",
      "",
    ].join("\n"),
  );
  assert.equal(parsed.status, "modified");
  assert.equal(parsed.added, 1);
  assert.equal(parsed.removed, 1);
  assert.deepEqual(parsed.rows, [
    { type: "hunk", text: "@@ -3,3 +3,3 @@ fn" },
    { type: "context", text: "keep", oldLine: 3, newLine: 3 },
    { type: "remove", text: "old", oldLine: 4, newLine: null },
    { type: "add", text: "new", oldLine: null, newLine: 4 },
    { type: "note", text: "No newline at end of file" },
  ]);
  const binary = parseUnifiedDiff(
    "diff --git a/i b/i\nnew file mode 100644\nBinary files /dev/null and b/i differ\n",
  );
  assert.equal(binary.binary, true);
  assert.equal(binary.status, "added");
  assert.deepEqual(binary.rows, []);
  assert.deepEqual(parseUnifiedDiff("").rows, []);
});

test("previews an image from before and after the turn", async () => {
  await withRepo(async (root, gitSync) => {
    const before = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0, 1, 2]);
    const after = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0, 3, 4, 5]);
    await writeFile(join(root, "shot.png"), before);
    gitSync("add", "shot.png");
    gitSync("commit", "-q", "-m", "shot");
    const tracker = createTurnDiffTracker();
    const withWorkspace = { ...agent(root), workspaceId: "w1" };
    await tracker.started(withWorkspace, "t1");
    await writeFile(join(root, "shot.png"), after);
    await writeFile(join(root, "new.PNG"), before);
    const source = (await tracker.ended(withWorkspace, "t1"))?.diff.source;
    assert.ok(source);

    const changed = await readFileImage({ ...source, path: "shot.png" });
    assert.equal(changed.before, `data:image/png;base64,${before.toString("base64")}`);
    assert.equal(changed.after, `data:image/png;base64,${after.toString("base64")}`);
    assert.equal(changed.tooLarge, false);
    const created = await readFileImage({ ...source, path: "new.PNG" });
    assert.equal(created.before, null);
    assert.ok(created.after?.startsWith("data:image/png;base64,"));
    await assert.rejects(readFileImage({ ...source, path: "a.txt" }), /not a previewable image/);
  });
});

test("knows which paths are previewable images", () => {
  assert.equal(imageType("docs/a/after-1664.png"), "image/png");
  assert.equal(imageType("x.JPG"), "image/jpeg");
  assert.equal(imageType("icon.svg"), "image/svg+xml");
  assert.equal(imageType("notes.txt"), null);
  assert.equal(imageType("Makefile"), null);
});
