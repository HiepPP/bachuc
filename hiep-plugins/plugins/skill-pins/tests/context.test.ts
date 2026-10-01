import { test } from "node:test";
import assert from "node:assert/strict";
import { appendContext, requested, skillContext } from "../server/context";

// Output of the removed native UserPromptSubmit hook for the same skill sets, captured on
// 2026-10-01 with Codex paths under /skills. The daemon hook must send the same text.
const expected = {
  "mixed-claude":
    "The user pinned these skills for this conversation. Before you respond, invoke every one of them with the Skill tool. Skip a skill only if its full text is still visible in your current context; an earlier load that has since been summarised away does not count:\n- watchtower\nInvoke these skills with the Skill tool on this turn, even if you already invoked them earlier in this conversation:\n- chase-goal-claude\nOnly the skills listed above are pinned now. Stop following any skill that was pinned earlier in this conversation but is missing from this list.\nUser instructions in the current prompt take priority over pinned skills.",
  "mixed-codex":
    "The user pinned these skills for this conversation. Before you respond, read each file and follow it. Skip a file only if its full text is still visible in your current context; an earlier read that has since been summarised away does not count:\n- /skills/watchtower/SKILL.md\nRead and follow these files again on this turn, even if you already read them earlier in this conversation:\n- /skills/chase-goal-claude/SKILL.md\nOnly the skills listed above are pinned now. Stop following any skill that was pinned earlier in this conversation but is missing from this list.\nUser instructions in the current prompt take priority over pinned skills.",
  "seq-claude":
    "The user pinned these skills for this conversation. Before you respond, invoke every one of them with the Skill tool. Skip a skill only if its full text is still visible in your current context; an earlier load that has since been summarised away does not count:\n- sequential-thinking\nOnly the skills listed above are pinned now. Stop following any skill that was pinned earlier in this conversation but is missing from this list.\nUser instructions in the current prompt take priority over pinned skills.",
  "seq-codex":
    "The user pinned these skills for this conversation. Before you respond, read each file and follow it. Skip a file only if its full text is still visible in your current context; an earlier read that has since been summarised away does not count:\n- /skills/sequential-thinking/SKILL.md\nOnly the skills listed above are pinned now. Stop following any skill that was pinned earlier in this conversation but is missing from this list.\nUser instructions in the current prompt take priority over pinned skills.",
} as const;

test("the context text equals the old native hook output", () => {
  assert.equal(
    skillContext(["watchtower", "chase-goal-claude"], "claude", "/skills"),
    expected["mixed-claude"],
  );
  assert.equal(
    skillContext(["watchtower", "chase-goal-claude"], "codex", "/skills"),
    expected["mixed-codex"],
  );
  assert.equal(skillContext(["sequential-thinking"], "claude", "/skills"), expected["seq-claude"]);
  assert.equal(skillContext(["sequential-thinking"], "codex", "/skills"), expected["seq-codex"]);
});

test("a skill named in the prompt counts as requested", () => {
  assert.equal(requested("Use /watchtower now", "watchtower"), true);
  assert.equal(requested("Use $sequential-thinking", "sequential-thinking"), true);
  assert.equal(requested("watchtower board", "watchtower"), false);
});

test("context is appended to text and block prompts", () => {
  assert.equal(appendContext("Hi", "Pins"), "Hi\n\nPins");
  assert.deepEqual(appendContext([{ type: "text", text: "Hi" }], "Pins"), [
    { type: "text", text: "Hi" },
    { type: "text", text: "Pins" },
  ]);
});
