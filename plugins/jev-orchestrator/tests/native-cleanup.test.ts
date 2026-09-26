import assert from "node:assert/strict";
import { mkdtemp, mkdir, readFile, readdir, rmdir, unlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";
import { NativeCleanup } from "../server/native-cleanup";

async function cleanupDirectory(directory: string) {
  for (const item of await readdir(directory, { withFileTypes: true })) {
    const target = path.join(directory, item.name);
    if (item.isDirectory()) await cleanupDirectory(target);
    else await unlink(target);
  }
  await rmdir(directory);
}

const id = (n: number) => n.toString(16).padStart(24, "0");

async function setup() {
  const temp = await mkdtemp(path.join(tmpdir(), "jev-cleanup-"));
  const base = path.join(temp, "native");
  const claude = path.join(temp, ".claude/agents");
  const codex = path.join(temp, ".codex/agents");
  for (const directory of [base, claude, codex]) await mkdir(directory, { recursive: true });
  const definition = async (directory: string, n: number, extension: string) => {
    const file = path.join(directory, `jev-native-${id(n)}.${extension}`);
    await writeFile(file, "definition");
    return file;
  };
  const manifest = async (runtime: string, n: number, files: string[]) => {
    const file = path.join(base, `${runtime}-${id(n)}.json`);
    await writeFile(file, JSON.stringify({ definitions: files.map((item) => ({ path: item })) }));
    return file;
  };
  return { temp, base, claude, codex, definition, manifest };
}

test("prune removes manifests of ended sessions and definitions nothing references", async () => {
  const { temp, base, claude, codex, definition, manifest } = await setup();
  try {
    const shared = await definition(claude, 1, "md");
    const oldOnly = await definition(claude, 2, "md");
    const codexOld = await definition(codex, 3, "toml");
    const current = await manifest("claude", 10, [shared]);
    const old = await manifest("claude", 11, [shared, oldOnly]);
    const codexManifest = await manifest("codex", 12, [codexOld]);
    await writeFile(path.join(codex, "jev-native-ticket-abc-0.toml"), "ticket");
    await writeFile(path.join(claude, "debugger.md"), "user role");
    const cleanup = new NativeCleanup(base, [claude, codex]);
    for (const [agent, file] of [
      ["live", current],
      ["ended", old],
      ["ended-codex", codexManifest],
    ])
      await cleanup.launch(agent, async () => ({ PASEO_JEV_NATIVE_POLICY: file }));
    await cleanup.launch("glm", async () => ({}));

    const result = await cleanup.prune([
      { id: "live", provider: "claude" },
      { id: "glm", provider: "glm-acp-agent" },
      { id: "never-opened", provider: "glm-acp-agent" },
    ]);

    assert.deepEqual(result, { manifests: 2, definitions: 2, waitingForUnknownSessions: false });
    assert.deepEqual((await readdir(base)).sort(), [path.basename(current), "leases.json"].sort());
    assert.deepEqual((await readdir(claude)).sort(), ["debugger.md", path.basename(shared)].sort());
    assert.deepEqual(await readdir(codex), ["jev-native-ticket-abc-0.toml"]);
    assert.deepEqual(JSON.parse(await readFile(path.join(base, "leases.json"), "utf8")), {
      live: path.basename(current),
      glm: null,
    });
  } finally {
    await cleanupDirectory(temp);
  }
});

test("a live native session without a lease keeps every manifest", async () => {
  const { temp, base, claude, definition, manifest } = await setup();
  try {
    const referenced = await definition(claude, 1, "md");
    const orphan = await definition(claude, 2, "md");
    const legacy = await manifest("claude", 10, [referenced]);
    const cleanup = new NativeCleanup(base, [claude]);

    const result = await cleanup.prune([{ id: "opened-before-leases", provider: "claude" }]);

    assert.deepEqual(result, { manifests: 0, definitions: 1, waitingForUnknownSessions: true });
    assert.deepEqual(await readdir(claude), [path.basename(referenced)]);
    assert.ok((await readdir(base)).includes(path.basename(legacy)));
    assert.ok(!(await readdir(claude)).includes(path.basename(orphan)));
  } finally {
    await cleanupDirectory(temp);
  }
});

test("prune waits for a launch that is still writing its definitions", async () => {
  const { temp, base, claude, definition, manifest } = await setup();
  try {
    const cleanup = new NativeCleanup(base, [claude]);
    let release!: () => void;
    const gate = new Promise<void>((resolve) => (release = resolve));
    const launch = cleanup.launch("new", async () => {
      const file = await definition(claude, 1, "md");
      await gate;
      return { PASEO_JEV_NATIVE_POLICY: await manifest("claude", 10, [file]) };
    });
    const prune = cleanup.prune([{ id: "new", provider: "claude" }]);
    release();
    await launch;
    assert.deepEqual(await prune, {
      manifests: 0,
      definitions: 0,
      waitingForUnknownSessions: false,
    });
    assert.equal((await readdir(claude)).length, 1);
  } finally {
    await cleanupDirectory(temp);
  }
});
