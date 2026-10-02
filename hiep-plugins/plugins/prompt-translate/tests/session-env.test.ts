import { seedDraftPrefs } from "../server/draft";
import { AgentModes } from "../server/modes";
import { translateSettings } from "../shared/settings";
import { draftEnvKey } from "../shared/draft";
import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import type { PluginSessionOpenRequest } from "@getpaseo/plugin/server";
import { withBridgeCavemanEnv } from "../server/session-env";

const request = (provider: string, env: Record<string, string> = {}): PluginSessionOpenRequest => ({
  agentId: "00000000-0000-4000-8000-000000000001",
  workspaceId: null,
  provider,
  cwd: "/tmp",
  reason: "create",
  purpose: "interactive",
  env,
});

test("silences native Claude Caveman only when the bridge hook is installed", async () => {
  const dir = await mkdtemp(path.join(tmpdir(), "pt-session-"));
  assert.equal(withBridgeCavemanEnv(request("claude"), dir, {}), undefined);
  await writeFile(path.join(dir, "hook-runtime.json"), "{}");
  assert.equal(withBridgeCavemanEnv(request("codex"), dir, {}), undefined);
  const opened = withBridgeCavemanEnv(request("claude", { KEEP: "1" }), dir, {
    CAVEMAN_DEFAULT_MODE: "ultra",
  });
  assert.deepEqual(opened?.env, {
    KEEP: "1",
    PROMPT_TRANSLATE_CAVEMAN_DEFAULT_MODE: "ultra",
    CAVEMAN_DEFAULT_MODE: "off",
  });
  // Resume passes the already-silenced env back; the saved user default must survive.
  assert.deepEqual(withBridgeCavemanEnv(request("claude", opened!.env), dir, {})?.env, opened?.env);
  assert.equal(
    withBridgeCavemanEnv(request("claude"), dir, {})?.env.PROMPT_TRANSLATE_CAVEMAN_DEFAULT_MODE,
    "",
  );
});

test("New workspace preferences seed before the first turn and do not reset later selections", async () => {
  const modes = new AgentModes(await mkdtemp(path.join(tmpdir(), "pt-draft-")));
  const defaults = async () => translateSettings.schema.parse({ cavemanMode: "full" });
  const initial = request("codex", {
    KEEP: "1",
    [draftEnvKey]: JSON.stringify({ mode: "ultra", rewrite: false }),
  });
  const opened = await seedDraftPrefs(initial, modes, defaults);
  assert.deepEqual(await modes.get(initial.agentId), { mode: "ultra" });
  assert.deepEqual(opened.env, { KEEP: "1" });
  await modes.set(initial.agentId, "lite");
  await seedDraftPrefs(initial, modes, defaults);
  await seedDraftPrefs({ ...initial, reason: "resume" }, modes, defaults);
  assert.deepEqual(await modes.get(initial.agentId), { mode: "lite" });
  const second = { ...request("codex"), agentId: "00000000-0000-4000-8000-000000000002" };
  await seedDraftPrefs(second, modes, defaults);
  assert.deepEqual(await modes.get(second.agentId), { mode: "full" });
});
