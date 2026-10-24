import assert from "node:assert/strict";
import { test } from "node:test";
import { isSecret, sample } from "./lib.ts";

test("isSecret flags keys and tokens", () => {
  assert.equal(isSecret("curl -H 'Authorization: Bearer abc' x"), true);
  assert.equal(isSecret("export OPENAI_API_KEY=sk-abc123"), true);
  assert.equal(isSecret("git push https://ghp_abc@github.com/x"), true);
  assert.equal(isSecret("echo password=hunter2"), true);
});

test("isSecret passes ordinary commands", () => {
  assert.equal(isSecret("rg -n tokenize src"), false);
  assert.equal(isSecret("cargo test -p decider-probe"), false);
});

test("sample is deterministic and bounded", () => {
  const items = Array.from({ length: 50 }, (_, i) => i);
  const a = sample(items, 10, 42);
  assert.deepEqual(a, sample(items, 10, 42));
  assert.equal(a.length, 10);
  assert.equal(new Set(a).size, 10);
  assert.equal(sample(items, 80, 42).length, 50);
});
