import assert from "node:assert/strict";
import { test, type TestContext } from "node:test";
import {
  mkdtemp,
  mkdir,
  writeFile,
  readFile,
  symlink,
  readdir,
  unlink,
  rmdir,
} from "node:fs/promises";
import path from "node:path";
import os from "node:os";
import type { PaseoApi, PaseoWorkspace } from "@getpaseo/client";
import { readBoard, parseManifest, section } from "../server/board";
import { loadWorkspaceBoard, searchTaskAttachments } from "../server/handlers";
import { attachmentKey, boardSchema, searchTasksRpc } from "../shared/board";

const manifest = `# NEXT
## Current Active Plan
- Title: Example plan
## Tracker
| Order | TASK | Status | Spec | Deps | Context | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | TASK-001 First task | TODO | [Spec](tasks/TASK-001-first.md) | - | - | A note \\| with pipe |
| 2 | TASK-002 Second task | BLOCKED | [Spec](watchtower/tasks/TASK-002-second.md) | TASK-001 | - | Waiting |
`;
const first =
  "# TASK-001 First task\n\n## Brief\nBuild the first feature.\n\n```md\n## Keep this inside the brief\n```\n\n## Verify\nDo not attach this section.\n";
const second = "# TASK-002 Second task\n\n## Brief\nBuild the second feature.\n";

async function fixture(t: TestContext) {
  const root = await mkdtemp(path.join(os.tmpdir(), "watchtower-board-test-"));
  const dir = path.join(root, "watchtower");
  await mkdir(path.join(dir, "tasks"), { recursive: true });
  await writeFile(path.join(dir, "NEXT.md"), manifest);
  await writeFile(path.join(dir, "tasks/TASK-001-first.md"), first);
  await writeFile(path.join(dir, "tasks/TASK-002-second.md"), second);
  await writeFile(
    path.join(dir, "tasks/TASK-002-outcome.md"),
    "# TASK-002 Outcome\n\n## Outcome\nStatus: BLOCKED\n\nBlocked:\n- Need a fixture.\n\nVerified:\n- Nothing yet.\n",
  );
  t.after(async () => {
    // Remove only files created inside this test-owned temporary directory.
    for (const entry of await readdir(root, { recursive: true, withFileTypes: true })) {
      if (!entry.isDirectory()) await unlink(path.join(entry.parentPath, entry.name));
    }
    await rmdir(path.join(dir, "tasks"));
    await rmdir(dir);
    await rmdir(root);
  });
  return root;
}
function api(root: string): PaseoApi {
  const workspace = {
    id: "ws-1",
    workspaceDirectory: root,
    projectDisplayName: "Project",
    name: "main",
  } as PaseoWorkspace;
  return {
    workspaces: {
      ref: (id: string) => ({ refresh: async () => (id === workspace.id ? workspace : null) }),
      list: async () => ({ entries: [workspace] }),
    },
  } as unknown as PaseoApi;
}
async function snapshot(root: string) {
  const files = await readdir(root, { recursive: true, withFileTypes: true });
  return Promise.all(
    files
      .filter((file) => file.isFile())
      .map(async (file) => [
        path.join(file.parentPath, file.name),
        await readFile(path.join(file.parentPath, file.name), "utf8"),
      ]),
  );
}

test("reads task status, dependencies, brief and recorded blocker without changing files", async (t) => {
  const root = await fixture(t);
  const before = await snapshot(root);
  const board = boardSchema.parse(await loadWorkspaceBoard("ws-1", api(root)));
  assert.equal(board.title, "Example plan");
  assert.deepEqual(
    board.tasks.map((task) => [task.id, task.status, task.deps]),
    [
      ["TASK-001", "TODO", "-"],
      ["TASK-002", "BLOCKED", "TASK-001"],
    ],
  );
  assert.match(board.tasks[0].notes, /note \| with pipe/);
  assert.match(board.tasks[0].brief!, /## Keep this inside the brief/);
  assert.doesNotMatch(board.tasks[0].brief!, /Do not attach/);
  assert.equal(board.tasks[1].blocker, "- Need a fixture.");
  await loadWorkspaceBoard("ws-1", api(root));
  const result = await searchTaskAttachments(attachmentKey("ws-1", "TASK-002"), api(root));
  assert.equal(result.items.length, 1);
  assert.equal(result.items[0].id, "ws-1:TASK-002");
  assert.equal(
    result.items[0].subtitle,
    `BLOCKED · ${root} · ${attachmentKey("ws-1", "TASK-002")}`,
  );
  assert.match(result.items[0].text, /Build the second feature/);
  assert.match(result.items[0].text, /Blocker: - Need a fixture/);
  assert.deepEqual(await snapshot(root), before);
});

test("attachment search returns validated snapshots without any send API", async (t) => {
  const root = await fixture(t);
  const result = searchTasksRpc.output.parse(
    await searchTaskAttachments("Project TASK-001", api(root)),
  );
  assert.equal(result.items.length, 1);
  assert.match(result.items[0].text, /Build the first feature/);
  assert.doesNotMatch(result.items[0].text, /Do not attach/);
  assert.equal(result.items[0].subtitle, `TODO · ${root}`);
  assert.deepEqual((await searchTaskAttachments("no-match", api(root))).items, []);
  await assert.rejects(loadWorkspaceBoard("unknown", api(root)), /unavailable/);
});

test("missing spec stays visible but cannot be attached", async (t) => {
  const root = await fixture(t);
  await unlink(path.join(root, "watchtower/tasks/TASK-001-first.md"));
  const board = await readBoard(root);
  assert.equal(board.tasks.length, 2);
  assert.equal(board.tasks[0].brief, null);
  assert.equal(board.tasks[0].error, "File not found.");
  assert.deepEqual((await searchTaskAttachments("TASK-001", api(root))).items, []);
});

test("legacy, missing, and malformed plans have readable messages", async (t) => {
  const root = await fixture(t);
  await writeFile(path.join(root, "watchtower/NEXT.md"), "# NEXT\n## TODO 1\nLegacy");
  assert.match((await readBoard(root)).message!, /Unsupported/);
  await assert.rejects(searchTaskAttachments("workspace:ws-1", api(root)), /Unsupported/);
  await unlink(path.join(root, "watchtower/NEXT.md"));
  assert.match((await readBoard(root)).message!, /File not found/);
  assert.match(
    parseManifest(manifest.replace("TASK-002 Second", "TASK-001 Second")).message!,
    /unique TASK/,
  );
});

test("rejects traversal, external links, and symlinks outside Watchtower", async (t) => {
  const root = await fixture(t);
  await writeFile(path.join(root, "outside.md"), "# TASK-001\n## Brief\nprivate");
  for (const link of ["../outside.md", "https://example.com/file.md", "%2e%2e/outside.md"]) {
    await writeFile(
      path.join(root, "watchtower/NEXT.md"),
      manifest.replace("tasks/TASK-001-first.md", link),
    );
    const task = (await readBoard(root)).tasks[0];
    assert.equal(task.brief, null);
    assert.ok(task.error);
  }
  await writeFile(path.join(root, "watchtower/NEXT.md"), manifest);
  await unlink(path.join(root, "watchtower/tasks/TASK-001-first.md"));
  await symlink(
    path.join(root, "outside.md"),
    path.join(root, "watchtower/tasks/TASK-001-first.md"),
  );
  assert.match((await readBoard(root)).tasks[0].error!, /outside/);
});

test("invalid identity, missing Brief, oversized specs and unknown status fail clearly", async (t) => {
  const root = await fixture(t);
  const file = path.join(root, "watchtower/tasks/TASK-001-first.md");
  await writeFile(file, "# TASK-999\n## Brief\nwrong");
  assert.match((await readBoard(root)).tasks[0].error!, /does not match/);
  await writeFile(file, "# TASK-001\n## Verify\nno brief");
  assert.match((await readBoard(root)).tasks[0].error!, /Brief/);
  await writeFile(file, "x".repeat(128 * 1024 + 1));
  assert.match((await readBoard(root)).tasks[0].error!, /128 KiB/);
  assert.match(parseManifest(manifest.replace("| TODO |", "| MAYBE |")).tasks[0].error!, /Unknown/);
  assert.equal(section("## Brief\nA\n## Verify\nB", "Brief"), "A");
});

test("scoped attachment keys match the exact task ID and keep snapshots stable", async (t) => {
  const root = await fixture(t);
  const file = path.join(root, "watchtower/NEXT.md");
  await writeFile(
    file,
    manifest +
      "| 3 | TASK-0010 Longer ID | TODO | [Spec](tasks/TASK-0010-extra.md) | - | - | - |\n",
  );
  await writeFile(
    path.join(root, "watchtower/tasks/TASK-0010-extra.md"),
    "# TASK-0010 Longer ID\n## Brief\nAnother brief.",
  );
  const selected = await searchTaskAttachments(attachmentKey("ws-1", "TASK-001"), api(root));
  assert.equal(selected.items.length, 1);
  assert.equal(selected.items[0].identifier, "TASK-001");
  const original = selected.items[0].text;
  await writeFile(
    path.join(root, "watchtower/tasks/TASK-001-first.md"),
    "# TASK-001 First task\n## Brief\nUpdated brief.",
  );
  const refreshed = await searchTaskAttachments(attachmentKey("ws-1", "TASK-001"), api(root));
  assert.match(refreshed.items[0].text, /Updated brief/);
  assert.equal(selected.items[0].text, original);
});
