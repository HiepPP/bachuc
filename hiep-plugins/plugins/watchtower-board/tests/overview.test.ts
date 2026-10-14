import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
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
import { loadWorkspaceDecision, loadWorkspaceOverview } from "../server/handlers";
import {
  parseDecisions,
  parseManualChecks,
  parseOverviewQuestions,
  parseOverviewRun,
  readDecision,
  readOverview,
  runFromGitLog,
} from "../server/overview";
import {
  decisionDetailSchema,
  HISTORY_LIMIT,
  overviewSchema,
  readDecisionRpc,
  readOverviewRpc,
} from "../shared/overview";

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

test("dated log times show a gap of several days and clock times continue from them", () => {
  const day = 24 * 60;
  const run = parseOverviewRun(
    `# Run

- Started: 2026-10-05 13:00
- Finished: -

## Log

| Start | End | TASK | Result | PR or reason |
| --- | --- | --- | --- | --- |
| 13:05 | 13:30 | TASK-001 | DONE | #1 |
| 13:35 | 14:10 | TASK-002 | DONE | #2 |
| 2026-10-08 09:30 | 2026-10-08 10:00 | TASK-003 | DONE | #3 |
| 10:05 | 10:40 | TASK-004 | DONE | #4 |
| 23:50 | 00:20 | TASK-005 | DONE | #5 |
`,
    new Date(2026, 9, 9, 0, 30),
  );
  assert.deepEqual(
    run.log.map((row) => [row.task, row.startMinute, row.endMinute]),
    [
      ["TASK-001", 5, 30],
      ["TASK-002", 35, 70],
      ["TASK-003", 3 * day - 210, 3 * day - 180],
      ["TASK-004", 3 * day - 175, 3 * day - 140],
      // Clock times after a dated row still roll over midnight.
      ["TASK-005", 3 * day + 650, 3 * day + 680],
    ],
  );
  assert.equal(run.log[2].start, "2026-10-08 09:30");
  assert.equal(run.nowMinute, 3 * day + 690);
  assert.equal(run.stopped, false);
  // Without `Started:` a dated time falls back to its clock part.
  const unstarted = parseOverviewRun(
    "# Run\n\n## Log\n\n| Start | End | TASK | Result |\n| --- | --- | --- | --- |\n| 2026-10-08 09:30 | 2026-10-08 10:00 | TASK-001 | DONE |\n",
  );
  assert.deepEqual(
    unstarted.log.map((row) => [row.startMinute, row.endMinute]),
    [[0, 30]],
  );
});

test("a Started date with no time counts from midnight, so a stale run has stopped", () => {
  const run = parseOverviewRun(
    "# Run\n\n- Started: 2026-10-07\n- Finished: -\n\n## Log\n\n| Start | End | TASK | Result |\n| --- | --- | --- | --- |\n| 15:42 | 16:04 | TASK-001 | DONE |\n| 17:14 | 17:18 | TASK-002 | DONE |\n",
    new Date(2026, 9, 10, 16, 0),
  );
  assert.deepEqual(
    run.log.map((row) => [row.startMinute, row.endMinute]),
    [
      [942, 964],
      [1034, 1038],
    ],
  );
  assert.equal(run.stopped, true);
  assert.equal(run.nowMinute, 1038);
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

test("history lists the 20 newest real archive folders", async (t) => {
  assert.equal(HISTORY_LIMIT, 20);
  const files: Files = { "NEXT.md": manifest, ...specs };
  for (let day = 1; day <= 24; day++) {
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
  assert.equal(overview.history.length, 20);
  assert.deepEqual(overview.history.slice(0, 2), [
    { slug: "20260924-plan-24", date: "2026-09-24", title: "Plan 24", hasLearn: true },
    { slug: "20260923-plan-23", date: "2026-09-23", title: "Plan 23", hasLearn: false },
  ]);
  assert.deepEqual(overview.history.at(-1), {
    slug: "20260905-plan-5",
    date: "2026-09-05",
    title: "Plan 5",
    hasLearn: false,
  });
});

const adr =
  "# ADR-0003 Chip-only sources\n\nStatus: proposed\nDate: 2026-10-07\n\n## Decision\n\n- Use chips.\n";

test("the decision RPC has its own name and takes only ADR IDs", () => {
  assert.equal(readDecisionRpc.name, "watchtower.decision.read");
  const parses = (id: string) => readDecisionRpc.input.safeParse({ workspaceId: "ws", id }).success;
  for (const id of ["ADR-0003", "ADR-1", "ADR-123456"]) assert.equal(parses(id), true, id);
  for (const id of ["ADR-", "ADR-1234567", "adr-0003", "../ADR-0003", "ADR-0003/x", "ADR-0003.md"])
    assert.equal(parses(id), false, id);
});

test("a decision reads the ADR file named by its ID", async (t) => {
  const root = await fixture(t, {
    "NEXT.md": manifest,
    "decisions/ADR-0003-chip-only-sources.md": adr,
    "decisions/ADR-0030-other.md": "# ADR-0030 Other\n",
    "decisions/ADR-0004.md": "# ADR-0004: Exact name\n\nStatus: accepted\n",
  });
  assert.deepEqual(decisionDetailSchema.parse(await readDecision(root, "ADR-0003")), {
    id: "ADR-0003",
    title: "Chip-only sources",
    status: "proposed",
    file: "watchtower/decisions/ADR-0003-chip-only-sources.md",
    markdown: adr,
  });
  const exact = await readDecision(root, "ADR-0004");
  assert.deepEqual(
    [exact.title, exact.status, exact.file],
    ["Exact name", "accepted", "watchtower/decisions/ADR-0004.md"],
  );
});

test("a decision without a readable file gives a clear error", async (t) => {
  const root = await fixture(t, {
    "NEXT.md": manifest,
    "decisions/ADR-0003-chip-only-sources.md": adr,
    "decisions/ADR-0005-big.md": `# ADR-0005 Big\n${"x".repeat(128 * 1024)}`,
    "../elsewhere/secret.md": "# ADR-0006 Outside\n",
  });
  await assert.rejects(
    readDecision(root, "ADR-0099"),
    /ADR-0099 has no file in watchtower\/decisions/,
  );
  await assert.rejects(readDecision(root, "ADR-0005"), /ADR-0005-big\.md: .*128 KiB/);
  await symlink(
    path.join(root, "elsewhere/secret.md"),
    path.join(root, "watchtower/decisions/ADR-0006-link.md"),
  );
  await assert.rejects(readDecision(root, "ADR-0006"), /outside the workspace Watchtower/);

  const noFolder = await fixture(t, { "NEXT.md": manifest });
  await assert.rejects(
    readDecision(noFolder, "ADR-0003"),
    /Cannot read watchtower\/decisions\. File not found\./,
  );
  const empty = await mkdtemp(path.join(os.tmpdir(), "watchtower-decision-empty-"));
  t.after(() => rmdir(empty));
  await assert.rejects(readDecision(empty, "ADR-0003"), /no watchtower directory/);
});

test("the decision handler reads the workspace directory", async (t) => {
  const root = await fixture(t, { "decisions/ADR-0003-chip-only-sources.md": adr });
  const workspace = { id: "ws-1", workspaceDirectory: root } as PaseoWorkspace;
  const paseo = {
    workspaces: { ref: () => ({ refresh: async () => workspace }) },
  } as unknown as PaseoApi;
  assert.equal((await loadWorkspaceDecision("ws-1", "ADR-0003", paseo)).status, "proposed");
  const missing = {
    workspaces: { ref: () => ({ refresh: async () => null }) },
  } as unknown as PaseoApi;
  await assert.rejects(
    loadWorkspaceDecision("ws-2", "ADR-0003", missing),
    /Workspace is unavailable/,
  );
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
  // The real index changes as decisions move; the fixture cases above cover each status.
  assert.ok(overview.decisions.length > 0);
  for (const decision of overview.decisions)
    assert.match(decision.status, /^(proposed|accepted|superseded by ADR-\d{4})$/);
  const size = Buffer.byteLength(JSON.stringify(overview));
  t.diagnostic(`overview JSON bytes: ${size}`);
  assert.ok(size <= 32 * 1024, `overview JSON is ${size} bytes`);
});

// Git log lines are `sha<TAB>iso time<TAB>subject`, newest first. Times are built in local time,
// because the run log shows local clock times.
const iso = (hour: number, minute: number, day = 14) =>
  new Date(2026, 9, day, hour, minute).toISOString();
const commit = (sha: string, time: string, subject: string) => `${sha}\t${time}\t${subject}`;
const done = (...ids: string[]) => ids.map((id) => ({ id, status: "DONE" }));
const gitLines = [
  commit("e4e4e4e4e4e4", iso(11, 0), "docs(watchtower): record wave 2"),
  commit("c3c3c3c3c3c3", iso(10, 30), "feat(board): second part (TASK-002)"),
  commit("c2c2c2c2c2c2", iso(10, 0), "fix(board): tweak the first part (TASK-001)"),
  commit("c1c1c1c1c1c1", iso(9, 20), "feat(board): first part (TASK-001)"),
  commit("c0c0c0c0c0c0", iso(9, 0), "docs(watchtower): plan it"),
  commit("a0a0a0a0a0a0", iso(8, 0), "chore: older work"),
];

test("the git log gives each counted TASK a row from the commit before its first commit", () => {
  const run = runFromGitLog(gitLines, done("TASK-001", "TASK-002"), "20261014-plan");
  assert.ok(run);
  assert.equal(run.runner, "implement (git)");
  assert.equal(run.started, "2026-10-14 09:00");
  assert.equal(run.finished, "2026-10-14 10:30");
  assert.deepEqual(run.log, [
    {
      task: "TASK-001",
      result: "DONE",
      detail: "commit c2c2c2c",
      start: "09:00",
      end: "10:00",
      startMinute: 0,
      endMinute: 60,
    },
    {
      task: "TASK-002",
      result: "DONE",
      detail: "commit c3c3c3c",
      start: "10:00",
      end: "10:30",
      startMinute: 60,
      endMinute: 90,
    },
  ]);
  assert.equal(run.nowMinute, 90);
  assert.equal(run.stopped, false);
});

test("the plan's own NEXT.md commit sets Started, and the plan slug picks it", () => {
  // Each record is `sha<TAB>iso time<TAB>subject<TAB>body`, newest first.
  const plan = [
    commit("b2b2b2b2b2b2", iso(9, 10), "docs(watchtower): tick TASK-001"),
    commit("b1b1b1b1b1b1", iso(8, 30), "docs(watchtower): start the plan\tSlug: 20261014-plan"),
    commit("b0b0b0b0b0b0", iso(7, 0), "docs(watchtower): older plan\tSlug: 20261001-other"),
  ];
  const run = runFromGitLog(gitLines, done("TASK-001", "TASK-002"), "20261014-plan", plan);
  assert.equal(run?.started, "2026-10-14 08:30");
  assert.deepEqual(
    run?.log.map((row) => [row.task, row.startMinute, row.endMinute]),
    [
      ["TASK-001", 30, 90],
      ["TASK-002", 90, 120],
    ],
  );
  // A plan commit that does not name the slug does not count.
  const none = runFromGitLog(gitLines, done("TASK-001"), "20261014-plan", [plan[2]]);
  assert.equal(none?.started, "2026-10-14 09:00");
});

test("only a TASK ID of the Tracker in parentheses counts", () => {
  const lines = [
    commit("d3d3d3d3d3d3", iso(12, 0), "docs(watchtower): add TASK-001..002 to the plan"),
    commit("d2d2d2d2d2d2", iso(11, 0), "feat(board): not in this plan (TASK-099)"),
    commit("d1d1d1d1d1d1", iso(10, 0), "feat(board): both parts (TASK-001, TASK-002)"),
    commit("d0d0d0d0d0d0", iso(9, 0), "docs(watchtower): plan it"),
  ];
  const run = runFromGitLog(lines, done("TASK-001", "TASK-002"), null);
  assert.deepEqual(
    run?.log.map((row) => [row.task, row.detail, row.startMinute, row.endMinute]),
    [
      ["TASK-001", "commit d1d1d1d", 0, 60],
      ["TASK-002", "commit d1d1d1d", 0, 60],
    ],
  );
  assert.equal(runFromGitLog(lines, done("TASK-050"), null), null);
  assert.equal(runFromGitLog([], done("TASK-001"), null), null);
  assert.equal(runFromGitLog(["not a log line", ""], done("TASK-001"), null), null);
});

test("a Tracker row that is not DONE keeps the run open until it stops", () => {
  const tasks = [...done("TASK-001"), { id: "TASK-002", status: "IN PROGRESS" }];
  // Last activity is minute 90, at 10:30.
  const running = runFromGitLog(gitLines, tasks, null, [], new Date(2026, 9, 14, 12, 0));
  assert.equal(running?.finished, null);
  assert.equal(running?.nowMinute, 180);
  assert.equal(running?.stopped, false);
  const stopped = runFromGitLog(gitLines, tasks, null, [], new Date(2026, 9, 14, 12, 31));
  assert.equal(stopped?.stopped, true);
  assert.equal(stopped?.nowMinute, 90);
});

test("a TASK commit after midnight shows a dated time", () => {
  const lines = [
    commit("f1f1f1f1f1f1", iso(0, 20, 15), "feat(board): late part (TASK-001)"),
    commit("f0f0f0f0f0f0", iso(23, 30, 14), "docs(watchtower): plan it"),
  ];
  const run = runFromGitLog(lines, done("TASK-001"), null);
  assert.equal(run?.started, "2026-10-14 23:30");
  assert.equal(run?.finished, "2026-10-15 00:20");
  assert.deepEqual(run?.log[0], {
    task: "TASK-001",
    result: "DONE",
    detail: "commit f1f1f1f",
    start: "23:30",
    end: "2026-10-15 00:20",
    startMinute: 0,
    endMinute: 50,
  });
  // The oldest counted commit has no parent in the log, so its own time is the start.
  const first = runFromGitLog(lines.slice(0, 1), done("TASK-001"), null);
  assert.deepEqual(
    first?.log.map((row) => [row.startMinute, row.endMinute]),
    [[0, 0]],
  );
});

test("the overview builds the run from the git log only when RUN.md is absent", async (t) => {
  let calls = 0;
  const reader = async () => {
    calls += 1;
    return { lines: gitLines.map((line) => line.replace("TASK-002", "TASK-007")), planLines: [] };
  };
  const root = await fixture(t, { "NEXT.md": manifest, ...specs });
  const overview = overviewSchema.parse(
    await readOverview(root, new Date(2026, 9, 14, 12, 0), reader),
  );
  assert.deepEqual(overview.warnings, []);
  assert.equal(overview.run?.runner, "implement (git)");
  assert.deepEqual(
    overview.run?.log.map((row) => row.task),
    ["TASK-007"],
  );
  assert.equal(overview.run?.finished, null);
  assert.equal(calls, 1);

  const withRun = await fixture(t, { "NEXT.md": manifest, ...specs, "RUN.md": runLog });
  const logged = await readOverview(withRun, new Date(2026, 9, 7, 1, 12), reader);
  assert.equal(logged.run?.runner, "loop");
  assert.equal(calls, 1);
  // An unreadable RUN.md is not an absent one.
  const broken = await fixture(t, { "NEXT.md": manifest, ...specs, "RUN.md/keep": "x" });
  const unreadable = await readOverview(broken, new Date(), reader);
  assert.equal(unreadable.run, null);
  assert.deepEqual(unreadable.warnings, ["RUN.md: Expected a regular Markdown file."]);
  assert.equal(calls, 1);
});

test("no counted commit keeps run null, and a git failure adds one warning", async (t) => {
  const root = await fixture(t, { "NEXT.md": manifest, ...specs });
  const empty = await readOverview(root, new Date(), async () => ({ lines: [], planLines: [] }));
  assert.equal(empty.run, null);
  assert.deepEqual(empty.warnings, []);
  const failed = overviewSchema.parse(
    await readOverview(root, new Date(), async () => {
      throw new Error("git log timed out after 5 s.");
    }),
  );
  assert.equal(failed.run, null);
  assert.deepEqual(failed.warnings, ["git log: git log timed out after 5 s."]);
  assert.equal(failed.tasks.length, 3);
});

test("the git reader reads a real repository, and a folder outside one gives no run", async (t) => {
  const root = await fixture(t, { "NEXT.md": manifest, ...specs });
  const git = (time: string, ...args: string[]) =>
    execFileSync(
      "git",
      [
        "-c",
        "user.name=t",
        "-c",
        "user.email=t@example.com",
        "-c",
        "commit.gpgsign=false",
        ...args,
      ],
      { cwd: root, env: { ...process.env, GIT_COMMITTER_DATE: time, GIT_AUTHOR_DATE: time } },
    );
  const outside = await readOverview(root);
  assert.equal(outside.run, null);
  assert.deepEqual(outside.warnings, []);
  git(iso(9, 0), "init", "-q");
  git(iso(9, 0), "commit", "-q", "--allow-empty", "-m", "docs: plan it");
  git(iso(9, 40), "commit", "-q", "--allow-empty", "-m", "feat(board): host (TASK-006)");
  const overview = overviewSchema.parse(await readOverview(root, new Date(2026, 9, 14, 10, 0)));
  assert.deepEqual(overview.warnings, []);
  assert.equal(overview.run?.started, "2026-10-14 09:00");
  assert.deepEqual(
    overview.run?.log.map((row) => [row.task, row.start, row.end]),
    [["TASK-006", "09:00", "09:40"]],
  );
  assert.match(overview.run?.log[0].detail ?? "", /^commit [0-9a-f]{7}$/);
});
