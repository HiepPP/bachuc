import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtemp } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { AgentModes } from "../server/modes";

const A = "00000000-0000-4000-8000-000000000001";

test("AgentModes defaults to full and round trips a saved mode", async () => {
  const modes = new AgentModes(await mkdtemp(path.join(tmpdir(), "pony-modes-")));
  assert.deepEqual(await modes.get(A), { mode: "full" });
  assert.equal(await modes.find(A), null);
  assert.deepEqual(await modes.set(A, "ultra"), { mode: "ultra" });
  assert.deepEqual(await modes.get(A), { mode: "ultra" });
  await assert.rejects(modes.set("../outside", "lite"));
});
