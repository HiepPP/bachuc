import assert from "node:assert/strict";
import { mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";
import type { PaseoAgentHandle } from "@getpaseo/client";
import { needsBackfill } from "../server/export";
import { readWithoutKeepingRuntime } from "../server/runtime";

function fakeHandle(after: { status: string; pendingPermissions?: unknown[] } | null) {
  let closes = 0;
  const handle = {
    refresh: async () => (after ? { agent: { pendingPermissions: [], ...after } } : null),
    closeRuntime: async () => {
      closes++;
    },
  } as unknown as PaseoAgentHandle;
  return { handle, closes: () => closes };
}

test("a thread that was closed before the read is closed again afterwards", async () => {
  const { handle, closes } = fakeHandle({ status: "idle" });
  assert.equal(await readWithoutKeepingRuntime(handle, "closed", async () => "text"), "text");
  assert.equal(closes(), 1);
});

test("a thread that was live before the read keeps its runtime", async () => {
  for (const status of ["idle", "running", "error"]) {
    const { handle, closes } = fakeHandle({ status: "idle" });
    await readWithoutKeepingRuntime(handle, status, async () => "text");
    assert.equal(closes(), 0);
  }
});

test("a thread that started working or awaits a permission is left alone", async () => {
  for (const after of [
    { status: "running" },
    { status: "initializing" },
    { status: "idle", pendingPermissions: [{ id: "p1" }] },
    null,
  ]) {
    const { handle, closes } = fakeHandle(after);
    await readWithoutKeepingRuntime(handle, "closed", async () => "text");
    assert.equal(closes(), 0);
  }
});

test("a failed read still releases the runtime, and a failed close keeps the read result", async () => {
  const failing = fakeHandle({ status: "idle" });
  await assert.rejects(
    readWithoutKeepingRuntime(failing.handle, "closed", async () => {
      throw new Error("timeline unavailable");
    }),
    /timeline unavailable/,
  );
  assert.equal(failing.closes(), 1);

  const handle = {
    refresh: async () => ({ agent: { status: "idle", pendingPermissions: [] } }),
    closeRuntime: async () => {
      throw new Error("Update the host to close an agent runtime.");
    },
  } as unknown as PaseoAgentHandle;
  assert.equal(await readWithoutKeepingRuntime(handle, "closed", async () => "text"), "text");
});

test("a host without closeRuntime reads normally", async () => {
  const handle = { refresh: async () => null } as unknown as PaseoAgentHandle;
  assert.equal(await readWithoutKeepingRuntime(handle, "closed", async () => "text"), "text");
});

test("backfill skips a closed thread that already has an export file", async () => {
  const dir = await mkdtemp(path.join(tmpdir(), "tca-backfill-"));
  await writeFile(path.join(dir, "exported.md"), "x");
  assert.equal(await needsBackfill({ id: "exported", status: "closed" }, dir), false);
  assert.equal(await needsBackfill({ id: "missing", status: "closed" }, dir), true);
  assert.equal(await needsBackfill({ id: "exported", status: "idle" }, dir), true);
});
