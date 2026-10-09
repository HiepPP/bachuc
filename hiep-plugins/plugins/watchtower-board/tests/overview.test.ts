import assert from "node:assert/strict";
import { test, type TestContext } from "node:test";
import {
  mkdtemp,
  mkdir,
  writeFile,
  readdir,
  readFile,
  unlink,
  rmdir,
  symlink,
} from "node:fs/promises";
import path from "node:path";
import os from "node:os";
import type { PaseoApi, PaseoWorkspace } from "@getpaseo/client";
import { loadWorkspaceOverview } from "../server/handlers";
import {
  parseDecisions,
  parseManualChecks,
  parseOverviewQuestions,
  parseOverviewRun,
  readOverview,
} from "../server/overview";
import { overviewSchema, readOverviewRpc } from "../shared/overview";

const manifest = `# NEXT

## Current Active Plan

- Title: Overview plan
- Slug: 20261007-overview-plan
- Status: ACTIVE
- Updated: 2026-10-07

## Tracker

| Order | TASK | Group | Status | Spec | Deps | Context | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | TASK-006 Host API | B | IN PROGRESS | [Spec](tasks/TASK-006-host.md) | - | - | - |
| 2 | TASK-010 Reveal API | B | TODO | [Spec](tasks/TASK-010-reveal.md) | TASK-006 | - | - |
| 3 | TASK-007 Cite plugin | D | TODO | [Spec](tasks/TASK-007-cite.md) | TASK-006, TASK-010 | - | - |

## Plan Verify

- \`npm run typecheck\` -> exit 0.
- [Link text](watchtower/CONTEXT.md) check -> passes.

## Handoff

- Next action: Keep going.
- Manual check pending: install \`assistant-cite\` on live and cite a reply (TASK-007).
- Manual check pending: a queued message survives a reload (TASK-003, TASK-004).
- Owner review: answer Q-010.
`;

type Files = Record<string, string>;

const specs: Files = {
  "tasks/TASK-006-host.md":
    "# TASK-006 Host API\n\nClass: risky\n\n## Brief\nBuild the host API.\n",
  "tasks/TASK-010-reveal.md": "# TASK-010 Reveal API\n\n## Brief\nReveal a passage.\n",
  "tasks/TASK-007-cite.md": "# TASK-007 Cite plugin\n\n## Brief\nCite a reply.\n",
};

async function fixture(t: TestContext, files: Files) {
  const root = await mkdtemp(path.join(os.tmpdir(), "watchtower-overview-test-"));
  const dir = path.join(root, "watchtower");
  for (const [name, text] of Object.entries(files)) {
    await mkdir(path.dirname(path.join(dir, name)), { recursive: true });
    await writeFile(path.join(dir, name), text);
  }
  t.after(async () => {
    // Remove only files and folders created inside this test-owned temporary directory.
    const entries = await readdir(root, { recursive: true, withFileTypes: true });
    for (const entry of entries)
      if (!entry.isDirectory()) await unlink(path.join(entry.parentPath, entry.name));
    const folders = entries
      .filter((entry) => entry.isDirectory())
      .map((entry) => path.join(entry.parentPath, entry.name))
      .sort((a, b) => b.length - a.length);
    for (const folder of folders) await rmdir(folder);
    await rmdir(root);
  });
  return root;
}

test("the overview RPC has its own name", () => {
  assert.equal(readOverviewRpc.name, "watchtower.overview.read");
});

test("reads plan meta, deps, groups, Plan Verify, and manual checks", async (t) => {
  const root = await fixture(t, { "NEXT.md": manifest, ...specs });
  const overview = overviewSchema.parse(await readOverview(root));
  assert.deepEqual(overview.plan, {
    title: "Overview plan",
    slug: "20261007-overview-plan",
    status: "ACTIVE",
    updated: "2026-10-07",
  });
  assert.equal(overview.message, null);
  assert.deepEqual(
    overview.tasks.map((task) => [task.id, task.depIds, task.group, task.taskClass]),
    [
      ["TASK-006", [], "B", "risky"],
      ["TASK-010", ["TASK-006"], "B", null],
      ["TASK-007", ["TASK-006", "TASK-010"], "D", null],
    ],
  );
  assert.deepEqual(overview.planVerify, [
    "npm run typecheck -> exit 0.",
    "Link text check -> passes.",
  ]);
  assert.deepEqual(overview.manualChecks, [
    { text: "Install assistant-cite on live and cite a reply (TASK-007).", tasks: ["TASK-007"] },
    {
      text: "A queued message survives a reload (TASK-003, TASK-004).",
      tasks: ["TASK-003", "TASK-004"],
    },
  ]);
});

test("a plan without Plan Verify or a Group column gives empty values", async (t) => {
  const plain = manifest
    .replace(/## Plan Verify[\s\S]*?(?=## Handoff)/, "")
    .replace("| Order | TASK | Group |", "| Order | TASK |")
    .replace("| --- | --- | --- |", "| --- | --- |")
    .replace(/\| (\d) \| (TASK-\d+ [^|]+)\| [BD] \|/g, "| $1 | $2|");
  const root = await fixture(t, { "NEXT.md": plain, ...specs });
  const overview = overviewSchema.parse(await readOverview(root));
  assert.equal(overview.message, null);
  assert.deepEqual(overview.planVerify, []);
  assert.deepEqual(
    overview.tasks.map((task) => task.group),
    [null, null, null],
  );
});

test("a missing NEXT.md gives an empty plan with a message", async (t) => {
  const root = await fixture(t, { "DECISIONS.md": "# Decisions\n" });
  const overview = overviewSchema.parse(await readOverview(root));
  assert.deepEqual(overview.tasks, []);
  assert.match(overview.message ?? "", /Cannot load watchtower\/NEXT\.md\. File not found\./);
  assert.deepEqual(overview.plan, { title: "Watchtower", slug: null, status: null, updated: null });
  assert.deepEqual(overview.planVerify, []);
  assert.deepEqual(overview.manualChecks, []);
});

test("manual checks keep only the pending-check bullets", () => {
  assert.deepEqual(parseManualChecks(null), []);
  assert.deepEqual(
    parseManualChecks("- Next action: ship.\n  - Manual check pending: nested.\n"),
    [],
  );
});

test("the workspace handler reads the workspace directory", async (t) => {
  const root = await fixture(t, { "NEXT.md": manifest, ...specs });
  const workspace = { id: "ws-1", workspaceDirectory: root } as PaseoWorkspace;
  const paseo = {
    workspaces: { ref: () => ({ refresh: async () => workspace }) },
  } as unknown as PaseoApi;
  const overview = await loadWorkspaceOverview("ws-1", paseo);
  assert.equal(overview.plan.slug, "20261007-overview-plan");
  const missing = {
    workspaces: { ref: () => ({ refresh: async () => null }) },
  } as unknown as PaseoApi;
  await assert.rejects(loadWorkspaceOverview("ws-2", missing), /Workspace is unavailable/);
});

const runLog = `# Run

- Runner: loop
- Started: 2026-10-06 22:52
- Finished: -

## Log

| Start | End | TASK | Result | PR or reason |
| --- | --- | --- | --- | --- |
| 22:52 | 23:06 | TASK-001 | DONE | #2 |
| 23:09 | 23:28 | TASK-002 | DONE | #3 |
| 00:02 | 00:21 | TASK-004 | DONE | #5 |
| 00:35 | now | TASK-006 | IN PROGRESS | - |
`;

test("run log times become minutes from the start across midnight", () => {
  const run = parseOverviewRun(runLog, new Date(2026, 9, 7, 1, 12));
  assert.equal(run.runner, "loop");
  assert.equal(run.started, "2026-10-06 22:52");
  assert.equal(run.finished, null);
  assert.deepEqual(
    run.log.map((row) => [row.task, row.startMinute, row.endMinute]),
    [
      ["TASK-001", 0, 14],
      ["TASK-002", 17, 36],
      ["TASK-004", 70, 89],
      ["TASK-006", 103, null],
    ],
  );
  assert.equal(run.log[0].detail, "#2");
  assert.equal(run.nowMinute, 140);
  assert.equal(run.stopped, false);
  const finished = parseOverviewRun(
    runLog.replace("- Finished: -", "- Finished: 2026-10-07 02:18. All done."),
    new Date(2026, 9, 9, 9, 0),
  );
  assert.equal(finished.nowMinute, 206);
  assert.equal(finished.stopped, false);
  assert.equal(parseOverviewRun("# Run\n").nowMinute, null);
  assert.equal(parseOverviewRun("# Run\n").stopped, false);
});

test("rows that overlap by a few minutes stay on the same day", () => {
  const run = parseOverviewRun(
    `# Run

- Started: 2026-10-09 14:20
- Finished: 2026-10-09 16:10

## Log

| Start | End | TASK | Result | PR or reason |
| --- | --- | --- | --- | --- |
| 14:19 | 14:40 | TASK-001 | DONE | #1 |
| 14:39 | 15:10 | TASK-002 | DONE | #2 |
`,
  );
  assert.deepEqual(
    run.log.map((row) => [row.startMinute, row.endMinute]),
    [
      [-1, 20],
      [19, 50],
    ],
  );
  assert.equal(run.nowMinute, 110);
});

test("a run with no finish and no log activity for over two hours has stopped", () => {
  // The last logged activity ends at 00:21, minute 89.
  const recent = parseOverviewRun(runLog, new Date(2026, 9, 7, 2, 21));
  assert.equal(recent.stopped, false);
  assert.equal(recent.nowMinute, 209);
  const stale = parseOverviewRun(runLog, new Date(2026, 9, 7, 2, 22));
  assert.equal(stale.stopped, true);
  assert.equal(stale.nowMinute, 89);
  const empty = parseOverviewRun(
    "# Run\n\n- Started: 2026-10-07 08:00\n- Finished: -\n",
    new Date(2026, 9, 7, 13, 0),
  );
  assert.equal(empty.stopped, true);
  assert.equal(empty.nowMinute, 0);
});

test("questions keep OPEN and DEFAULTED rows only", () => {
  const questions =
    parseOverviewQuestions(`| ID | TASK | Blocks | Question | Default | Status | Answer |
| --- | --- | --- | --- | --- | --- | --- |
| Q-001 | TASK-002 | - | Done? | Yes | ANSWERED | Yes |
| Q-008 | TASK-008, TASK-009 | TASK-009 | Network access? | No network | OPEN | - |
| Q-010 | TASK-010 | - | Native scroll? | Open the agent only | DEFAULTED | - |
| Q-011 | TASK-007 | - | Toast API? | No | DROPPED | - |
`);
  assert.deepEqual(questions, [
    {
      id: "Q-008",
      tasks: ["TASK-008", "TASK-009"],
      blocks: ["TASK-009"],
      question: "Network access?",
      default: "No network",
      status: "OPEN",
    },
    {
      id: "Q-010",
      tasks: ["TASK-010"],
      blocks: [],
      question: "Native scroll?",
      default: "Open the agent only",
      status: "DEFAULTED",
    },
  ]);
  assert.deepEqual(
    parseDecisions(`## Index

| ID | Date | Title | Status | Scope | File |
| --- | --- | --- | --- | --- | --- |
| ADR-0003 | 2026-10-07 | Chip-only sources | proposed | x | [ADR](a.md) |
`),
    [{ id: "ADR-0003", date: "2026-10-07", title: "Chip-only sources", status: "proposed" }],
  );
});

test("history lists the 5 newest real archive folders", async (t) => {
  const files: Files = { "NEXT.md": manifest, ...specs };
  for (let day = 1; day <= 8; day++) {
    const slug = `202609${String(day).padStart(2, "0")}-plan-${day}`;
    files[`archive/${slug}/NEXT.md`] = `# NEXT\n\n- Title: Plan ${day}\n`;
    if (day % 2 === 0) files[`archive/${slug}/LEARN.md`] = "# Learn\n";
  }
  files["archive/notes.md"] = "A file, not a plan.\n";
  files["../elsewhere/NEXT.md"] = "# NEXT\n\n- Title: Outside\n";
  const root = await fixture(t, files);
  await symlink(
    path.join(root, "elsewhere"),
    path.join(root, "watchtower/archive/20261231-linked"),
    "dir",
  );
  const overview = overviewSchema.parse(await readOverview(root));
  assert.deepEqual(overview.warnings, []);
  assert.deepEqual(overview.history, [
    { slug: "20260908-plan-8", date: "2026-09-08", title: "Plan 8", hasLearn: true },
    { slug: "20260907-plan-7", date: "2026-09-07", title: "Plan 7", hasLearn: false },
    { slug: "20260906-plan-6", date: "2026-09-06", title: "Plan 6", hasLearn: true },
    { slug: "20260905-plan-5", date: "2026-09-05", title: "Plan 5", hasLearn: false },
    { slug: "20260904-plan-4", date: "2026-09-04", title: "Plan 4", hasLearn: true },
  ]);
});

test("an unreadable run file warns and the plan still loads", async (t) => {
  const root = await fixture(t, { "NEXT.md": manifest, ...specs, "QUESTIONS.md/keep": "x" });
  const overview = overviewSchema.parse(await readOverview(root));
  assert.equal(overview.message, null);
  assert.deepEqual(overview.warnings, ["QUESTIONS.md: Expected a regular Markdown file."]);
  assert.deepEqual(overview.questions, []);
});

test("the real t3code plan parses without warnings and stays small", async (t) => {
  const repo = path.resolve(import.meta.dirname, "../../../..");
  const plan = path.join(repo, "watchtower/archive/20261006-t3code-orchestration-port");
  const files: Files = {};
  for (const name of ["NEXT.md", "QUESTIONS.md", "RUN.md"])
    files[name] = await readFile(path.join(plan, name), "utf8");
  for (const name of await readdir(path.join(plan, "tasks")))
    files[`tasks/${name}`] = await readFile(path.join(plan, "tasks", name), "utf8");
  files["DECISIONS.md"] = await readFile(path.join(repo, "watchtower/DECISIONS.md"), "utf8");
  const root = await fixture(t, files);
  const overview = overviewSchema.parse(await readOverview(root, new Date(2026, 9, 7, 3, 0)));
  assert.deepEqual(overview.warnings, []);
  assert.equal(overview.message, null);
  assert.equal(overview.tasks.length, 10);
  assert.equal(overview.run?.log.length, 10);
  assert.equal(overview.run?.log.at(-1)?.endMinute, 203);
  assert.equal(overview.run?.nowMinute, 206);
  assert.ok(overview.decisions.some((decision) => decision.status === "proposed"));
  const size = Buffer.byteLength(JSON.stringify(overview));
  t.diagnostic(`overview JSON bytes: ${size}`);
  assert.ok(size <= 32 * 1024, `overview JSON is ${size} bytes`);
});
