import assert from "node:assert/strict";
import { mkdtemp, readFile, stat, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";
import type { PaseoApi } from "@getpaseo/client";
import {
  createExporter,
  CUT_LINE,
  exportDir,
  renderThreadMarkdown,
  writeIfChanged,
} from "../server/export";

const thread = {
  id: "agent-1",
  title: "Fix login",
  provider: "codex",
  cwd: "/repo/app",
  lastActivityAt: "2026-09-23T10:00:00.000Z",
};
const user = (text: string) => ({ item: { type: "user_message", text } });
const assistant = (text: string) => ({ item: { type: "assistant_message", text } });
const tool = { item: { type: "tool_call" } };
const paseo = {} as PaseoApi;
const tick = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

test("markdown groups each user message with the replies after it and skips other items", () => {
  const text = renderThreadMarkdown(
    thread,
    [
      user("Why does login fail?"),
      tool,
      assistant("Checking."),
      assistant("Expiry uses <."),
      user("Fix it"),
      assistant("   "),
    ],
    false,
  );
  assert.equal(
    text,
    [
      "# Fix login",
      "",
      "Thread: agent-1 · codex · /repo/app",
      "Updated: 2026-09-23T10:00:00.000Z",
      "",
      "## Turn 1",
      "",
      "### Asked",
      "",
      "Why does login fail?",
      "",
      "### Answer",
      "",
      "Checking.\n\nExpiry uses <.",
      "",
      "## Turn 2",
      "",
      "### Asked",
      "",
      "Fix it",
      "",
      "### Answer",
      "",
      "(no reply yet)",
      "",
    ].join("\n"),
  );
});

test("replies before the first user message form Turn 0, and a cut adds the cut line", () => {
  const text =
    renderThreadMarkdown(thread, [assistant("tail of old turn"), user("next")], true) ?? "";
  assert.ok(text.includes(`\n${CUT_LINE}\n`));
  assert.ok(text.includes("## Turn 0\n\n### Answer\n\ntail of old turn"));
  assert.ok(text.includes("## Turn 1\n\n### Asked\n\nnext"));
  assert.equal(text.includes("## Turn 0\n\n### Asked"), false);
  assert.equal(renderThreadMarkdown(thread, [tool], false), null);
});

test("files are written with mode 0600 and an unchanged file is not rewritten", async () => {
  const dir = path.join(await mkdtemp(path.join(tmpdir(), "tca-export-")), "threads");
  assert.equal(await writeIfChanged(dir, "agent-1", "one\n"), true);
  const file = path.join(dir, "agent-1.md");
  assert.equal((await stat(file)).mode & 0o777, 0o600);
  assert.equal((await stat(dir)).mode & 0o777, 0o700);
  assert.equal(await writeIfChanged(dir, "agent-1", "one\n"), false);
  assert.equal(await writeIfChanged(dir, "agent-1", "two\n"), true);
  assert.equal(await readFile(file, "utf8"), "two\n");
  await writeFile(file, "edited");
  assert.equal(await writeIfChanged(dir, "agent-1", "two\n"), true);
});

test("the export folder follows PASEO_HOME", () => {
  assert.equal(
    exportDir({ PASEO_HOME: "/data/paseo" }),
    "/data/paseo/plugin-data/thread-context-attach/threads",
  );
});

test("scheduling debounces per thread and reports changed exports", async () => {
  const calls: string[] = [];
  const exporter = createExporter({
    exportOne: async (_, id) => {
      calls.push(id);
      return id !== "same";
    },
    log: () => {},
    debounceMs: 20,
  });
  let notified = 0;
  exporter.onExported(() => notified++);
  exporter.schedule(paseo, "a");
  exporter.schedule(paseo, "a");
  exporter.schedule(paseo, "b");
  exporter.schedule(paseo, "same");
  await tick(60);
  assert.deepEqual(calls.sort(), ["a", "b", "same"]);
  assert.equal(notified, 2);
  exporter.stop();
});

test("no more than 2 exports run at once, and a failure is logged without stopping others", async () => {
  let running = 0;
  let peak = 0;
  const lines: string[] = [];
  const exporter = createExporter({
    exportOne: async (_, id) => {
      running++;
      peak = Math.max(peak, running);
      await tick(15);
      running--;
      if (id === "bad") throw new Error("timeline unavailable");
      return true;
    },
    log: (line) => lines.push(line),
    debounceMs: 1,
  });
  for (const id of ["a", "b", "c", "bad", "d"]) exporter.schedule(paseo, id);
  await tick(120);
  assert.equal(peak, 2);
  assert.deepEqual(lines, ["export bad failed: timeline unavailable"]);
  exporter.stop();
});

test("backfill exports each thread once, notifies once, and later calls do nothing", async () => {
  const calls: string[] = [];
  const lines: string[] = [];
  const exporter = createExporter({
    exportOne: async (_, id) => {
      calls.push(id);
      return id !== "b";
    },
    log: (line) => lines.push(line),
  });
  let notified = 0;
  exporter.onExported(() => notified++);
  await exporter.backfill(paseo, ["a", "b", "c"]);
  await exporter.backfill(paseo, ["a"]);
  assert.deepEqual(calls, ["a", "b", "c"]);
  assert.equal(notified, 1);
  assert.deepEqual(lines, ["backfill exported 2 of 3 threads"]);
  exporter.stop();
});

test("stop cancels pending exports", async () => {
  const calls: string[] = [];
  const exporter = createExporter({
    exportOne: async (_, id) => {
      calls.push(id);
      return true;
    },
    log: () => {},
    debounceMs: 10,
  });
  exporter.schedule(paseo, "a");
  exporter.stop();
  await tick(30);
  assert.deepEqual(calls, []);
});
