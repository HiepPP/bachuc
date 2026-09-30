import assert from "node:assert/strict";
import test from "node:test";
import { nativePreset } from "../server/native-preset";

test("native presets retain exactly the user's provider-specific pairs for every role", () => {
  const expected = {
    codex: [
      ["gpt-6-astra", "low"],
      ["gpt-5.6-luna", "max"],
    ],
    claude: [
      ["claude-opus-5-5", "medium"],
      ["claude-opus-5-5", "high"],
      ["claude-opus-5-5", "xhigh"],
    ],
  };
  for (const runtime of ["codex", "claude"] as const) {
    const policy = nativePreset(runtime, ["worker", "reviewer"]);
    assert.equal(policy.routes.length, 2);
    for (const route of policy.routes) {
      assert.deepEqual(
        route.candidates.map(({ model, effort }) => [model, effort]),
        expected[runtime],
      );
    }
    assert.equal(
      new Set(policy.routes.flatMap((route) => route.candidates.map((item) => item.agentType)))
        .size,
      2 * expected[runtime].length,
    );
  }
});
