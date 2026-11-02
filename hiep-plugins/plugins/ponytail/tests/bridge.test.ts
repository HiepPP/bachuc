import { test } from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const bridge = path.join(
  path.dirname(fileURLToPath(import.meta.url)),
  "../server/ponytail-hook.cjs",
);
const run = (env: Record<string, string>, input: object) =>
  spawnSync(process.execPath, [bridge, "--claude"], {
    input: JSON.stringify(input),
    env: { PATH: process.env.PATH ?? "", ...env },
    encoding: "utf8",
  });

test("the bridge stays silent inside a Paseo agent", () => {
  const result = run(
    { PASEO_AGENT_ID: "00000000-0000-4000-8000-000000000000" },
    { hook_event_name: "SessionStart" },
  );
  assert.equal(result.status, 0);
  assert.equal(result.stdout, "");
});

test("the bridge exits 1, never 2, when the runtime file is missing", () => {
  const home = mkdtempSync(path.join(tmpdir(), "pony-bridge-"));
  const result = run({ PASEO_HOME: home }, { hook_event_name: "UserPromptSubmit", prompt: "x" });
  assert.equal(result.status, 1);
  assert.equal(result.stderr.trim().split("\n").length, 1);
});
