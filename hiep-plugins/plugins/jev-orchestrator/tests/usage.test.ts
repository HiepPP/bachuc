import test from "node:test";
import assert from "node:assert/strict";
import { evaluationUsage, missingUsage, summarizeUsage } from "../server/usage";

test("workflow includes each actor and evaluator once, without adding cache or reasoning twice", () => {
  const usage = {
    ...evaluationUsage({ inputTokens: 100, outputTokens: 20 }),
    cachedInputTokens: 80,
    cacheWriteTokens: 5,
    reasoningTokens: 12,
  };
  const summary = summarizeUsage([
    { id: "parent", role: "parent", usage },
    { id: "child", role: "implementation", usage },
    { id: "jev-0", role: "route", usage: evaluationUsage({ inputTokens: 10, outputTokens: 2 }) },
  ]);
  assert.equal(summary.complete, true);
  assert.equal(summary.totals.totalTokens, 252);
  assert.equal(summary.totals.cachedInputTokens, null);
  assert.equal(summary.totals.reasoningTokens, null);
  assert.throws(
    () =>
      summarizeUsage([
        { id: "same", role: "parent", usage },
        { id: "same", role: "child", usage },
      ]),
    /Duplicate/,
  );
});
test("missing or failed usage remains incomplete and is not a zero-token call", () => {
  const summary = summarizeUsage([
    { id: "known", role: "child", usage: evaluationUsage({ inputTokens: 100, outputTokens: 20 }) },
    { id: "unknown", role: "route", usage: missingUsage("gateway", "HTTP failure") },
  ]);
  assert.equal(summary.complete, false);
  assert.equal(summary.totals.totalTokens, null);
  assert.equal(summary.knownTotalTokens, 120);
  assert.deepEqual(summary.missing, ["unknown"]);
  assert.equal(evaluationUsage({ inputTokens: 10 }).totalTokens, null);
  assert.equal(summarizeUsage([]).complete, false);
});
