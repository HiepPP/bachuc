import assert from "node:assert/strict";
import { mkdtemp } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";
import { createIndexer, findQmd, updateChanged, type RunResult } from "../server/qmd";

const tick = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));
const tempDir = async () => path.join(await mkdtemp(path.join(tmpdir(), "tca-qmd-")), "threads");

/** A fake qmd that records calls; `list` stdout decides whether the collection exists. */
function fakeRun(
  list: string,
  updateMs = 0,
  updateCode = 0,
  updateStdout = () => "",
  embedCode = 0,
) {
  const calls: string[] = [];
  let running = 0;
  let peak = 0;
  const run = async (args: string[]): Promise<RunResult> => {
    calls.push(args.slice(0, 2).join(" "));
    if (args[0] === "update") {
      running++;
      peak = Math.max(peak, running);
      await tick(updateMs);
      running--;
    }
    if (args[0] === "update") return { code: updateCode, stdout: updateStdout() };
    return { code: args[0] === "embed" ? embedCode : 0, stdout: args[1] === "list" ? list : "" };
  };
  return { run, calls, peak: () => peak };
}

test("qmd lookup prefers QMD_BIN, then PATH, then known install paths", () => {
  const has =
    (...files: string[]) =>
    (file: string) =>
      files.includes(file);
  assert.equal(findQmd({ QMD_BIN: "/x/qmd", PATH: "/a" }, has("/x/qmd", "/a/qmd")), "/x/qmd");
  assert.equal(findQmd({ QMD_BIN: "/missing", PATH: "/a:/b" }, has("/b/qmd")), "/b/qmd");
  assert.equal(findQmd({ PATH: "/a" }, has("/opt/homebrew/bin/qmd")), "/opt/homebrew/bin/qmd");
  assert.equal(findQmd({ PATH: "/a" }, has()), null);
});

test("the collection is added only when it is missing", async () => {
  const missing = fakeRun("threads (qmd://threads/)");
  assert.equal(
    await createIndexer({ dir: await tempDir(), run: missing.run, log: () => {} }).start(),
    true,
  );
  assert.deepEqual(missing.calls, ["collection list", "collection add"]);
  const present = fakeRun("paseo-threads (qmd://paseo-threads/)");
  assert.equal(
    await createIndexer({ dir: await tempDir(), run: present.run, log: () => {} }).start(),
    true,
  );
  assert.deepEqual(present.calls, ["collection list"]);
});

test("updates run at most once per interval and never overlap", async () => {
  const fake = fakeRun("qmd://paseo-threads/", 30);
  const lines: string[] = [];
  const indexer = createIndexer({
    dir: await tempDir(),
    run: fake.run,
    log: (line) => lines.push(line),
    intervalMs: 150,
  });
  await indexer.start();
  indexer.notify();
  indexer.notify();
  await tick(10);
  // A notify during a run schedules exactly one more run after the interval.
  indexer.notify();
  indexer.notify();
  await tick(60);
  assert.equal(fake.calls.filter((call) => call === "update").length, 1);
  await tick(150);
  assert.equal(fake.calls.filter((call) => call === "update").length, 2);
  assert.equal(fake.peak(), 1);
  assert.ok(lines.every((line) => /^qmd (update|embed) exit 0 in \d+ ms$/.test(line)));
  indexer.stop();
});

test("a failing qmd is logged, and stop cancels a pending update", async () => {
  const lines: string[] = [];
  const failing = createIndexer({
    dir: await tempDir(),
    run: async () => {
      throw new Error("spawn qmd ENOENT");
    },
    log: (line) => lines.push(line),
  });
  assert.equal(await failing.start(), false);
  failing.notify();
  await tick(10);
  assert.deepEqual(lines, ["qmd setup failed: spawn qmd ENOENT"]);

  const fake = fakeRun("qmd://paseo-threads/");
  const stopped = createIndexer({
    dir: await tempDir(),
    run: fake.run,
    log: () => {},
    intervalMs: 20,
  });
  stopped.notify();
  stopped.stop();
  await tick(40);
  assert.deepEqual(fake.calls, []);
});

test("embed runs after a successful update and is skipped after a failed one", async () => {
  const ok = fakeRun("qmd://paseo-threads/");
  const indexer = createIndexer({
    dir: await tempDir(),
    run: ok.run,
    log: () => {},
    intervalMs: 0,
  });
  await indexer.start();
  indexer.notify();
  await tick(20);
  assert.deepEqual(ok.calls.slice(1), ["update", "embed"]);
  indexer.stop();

  const failed = fakeRun("qmd://paseo-threads/", 0, 1);
  const lines: string[] = [];
  const failing = createIndexer({
    dir: await tempDir(),
    run: failed.run,
    log: (line) => lines.push(line),
    intervalMs: 0,
  });
  await failing.start();
  failing.notify();
  await tick(20);
  assert.deepEqual(failed.calls.slice(1), ["update"]);
  assert.match(lines[0], /^qmd update exit 1 in \d+ ms$/);
  failing.stop();
});

const summary = (added: number, changed: number) =>
  `Indexed: ${added} new, ${changed} updated, 9 unchanged, 1 removed`;

test("an update summary counts only new and updated documents as changes", () => {
  assert.equal(updateChanged(summary(0, 0)), false);
  assert.equal(updateChanged(summary(1, 0)), true);
  assert.equal(updateChanged(summary(0, 2)), true);
  assert.equal(updateChanged("unexpected output"), true);
});

test("embed runs once at start, then only after updates that change documents", async () => {
  let next = summary(0, 0);
  const fake = fakeRun("qmd://paseo-threads/", 0, 0, () => next);
  const indexer = createIndexer({
    dir: await tempDir(),
    run: fake.run,
    log: () => {},
    intervalMs: 0,
    embedIntervalMs: 0,
  });
  await indexer.start();
  indexer.notify();
  await tick(20);
  indexer.notify();
  await tick(20);
  assert.deepEqual(fake.calls.slice(1), ["update", "embed", "update"]);
  next = summary(1, 0);
  indexer.notify();
  await tick(20);
  assert.deepEqual(fake.calls.slice(4), ["update", "embed"]);
  indexer.stop();
});

test("changes inside the embed interval share one later embed", async () => {
  const fake = fakeRun("qmd://paseo-threads/", 0, 0, () => summary(1, 0));
  const indexer = createIndexer({
    dir: await tempDir(),
    run: fake.run,
    log: () => {},
    intervalMs: 0,
    embedIntervalMs: 120,
  });
  await indexer.start();
  for (let i = 0; i < 3; i++) {
    indexer.notify();
    await tick(15);
  }
  const embeds = () => fake.calls.filter((call) => call === "embed").length;
  assert.equal(fake.calls.filter((call) => call === "update").length, 3);
  assert.equal(embeds(), 1);
  await tick(150);
  assert.equal(embeds(), 2);
  indexer.stop();
});

test("a failed embed is retried after the embed interval", async () => {
  const fake = fakeRun("qmd://paseo-threads/", 0, 0, () => summary(0, 0), 1);
  const lines: string[] = [];
  const indexer = createIndexer({
    dir: await tempDir(),
    run: fake.run,
    log: (line) => lines.push(line),
    intervalMs: 0,
    embedIntervalMs: 60,
  });
  await indexer.start();
  indexer.notify();
  await tick(20);
  assert.equal(fake.calls.filter((call) => call === "embed").length, 1);
  await tick(80);
  assert.equal(fake.calls.filter((call) => call === "embed").length, 2);
  assert.ok(lines.some((line) => line.startsWith("qmd embed exit 1 ")));
  indexer.stop();
});
