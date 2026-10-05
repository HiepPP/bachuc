import { test } from "node:test";
import assert from "node:assert/strict";
import type { PluginBeforeRequests } from "@getpaseo/plugin/server";
import { EDIT_TOOL_RULE, withEditToolRule } from "../server/edit-tool-rule";

function request(systemPrompt?: string): PluginBeforeRequests["agent.create"] {
  return {
    config: { provider: "claude", cwd: "/repo", systemPrompt },
    env: { A: "1" },
  } as PluginBeforeRequests["agent.create"];
}

test("adds the edit tool rule when the agent has no system prompt", () => {
  const next = withEditToolRule(request());
  assert.equal(next.config.systemPrompt, EDIT_TOOL_RULE);
  assert.deepEqual(next.env, { A: "1" });
  assert.equal(next.config.cwd, "/repo");
});

test("appends the rule after an existing system prompt", () => {
  assert.equal(
    withEditToolRule(request("Be terse.")).config.systemPrompt,
    `Be terse.\n\n${EDIT_TOOL_RULE}`,
  );
});

test("does not add the rule twice", () => {
  const once = withEditToolRule(request("Be terse."));
  assert.equal(withEditToolRule(once), once);
});
