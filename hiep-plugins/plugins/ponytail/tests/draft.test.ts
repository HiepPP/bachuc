import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtemp } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import type { PluginSessionOpenRequest } from "@getpaseo/plugin/server";
import { seedDraftPrefs } from "../server/draft";
import { AgentModes } from "../server/modes";
import { draftEnvKey } from "../shared/modes";

const request = (agentId: string, env: Record<string, string> = {}): PluginSessionOpenRequest => ({
  agentId,
  workspaceId: null,
  provider: "claude",
  cwd: "/tmp",
  reason: "create",
  purpose: "interactive",
  env,
});
const A = "00000000-0000-4000-8000-000000000001";
const B = "00000000-0000-4000-8000-000000000002";

test("a draft mode seeds a new agent and the draft key is removed", async () => {
  const modes = new AgentModes(await mkdtemp(path.join(tmpdir(), "pony-draft-")));
  const opened = await seedDraftPrefs(
    request(A, { KEEP: "1", [draftEnvKey]: JSON.stringify({ mode: "ultra" }) }),
    modes,
  );
  assert.deepEqual(await modes.get(A), { mode: "ultra" });
  assert.deepEqual(opened.env, { KEEP: "1" });
});

test("no draft seeds full, and a later session keeps a changed mode", async () => {
  const modes = new AgentModes(await mkdtemp(path.join(tmpdir(), "pony-draft-")));
  await seedDraftPrefs(request(B), modes);
  assert.deepEqual(await modes.get(B), { mode: "full" });
  await modes.set(B, "lite");
  await seedDraftPrefs(request(B, { [draftEnvKey]: JSON.stringify({ mode: "ultra" }) }), modes);
  assert.deepEqual(await modes.get(B), { mode: "lite" });
});

test("resume keeps the saved mode and only removes the draft key", async () => {
  const modes = new AgentModes(await mkdtemp(path.join(tmpdir(), "pony-draft-")));
  const resumed = await seedDraftPrefs(
    { ...request(A, { [draftEnvKey]: JSON.stringify({ mode: "ultra" }) }), reason: "resume" },
    modes,
  );
  assert.equal(await modes.find(A), null);
  assert.deepEqual(resumed.env, {});
});
