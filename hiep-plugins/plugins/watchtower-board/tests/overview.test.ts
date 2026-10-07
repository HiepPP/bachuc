import assert from "node:assert/strict";
import { test, type TestContext } from "node:test";
import { mkdtemp, mkdir, writeFile, readdir, unlink, rmdir } from "node:fs/promises";
import path from "node:path";
import os from "node:os";
import type { PaseoApi, PaseoWorkspace } from "@getpaseo/client";
import { loadWorkspaceOverview } from "../server/handlers";
import { parseManualChecks, readOverview } from "../server/overview";
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
    { text: "install assistant-cite on live and cite a reply (TASK-007).", tasks: ["TASK-007"] },
    {
      text: "a queued message survives a reload (TASK-003, TASK-004).",
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
