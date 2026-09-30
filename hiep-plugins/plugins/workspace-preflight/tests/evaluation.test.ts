import assert from "node:assert/strict";
import { test } from "node:test";
import { evaluationInput, evaluateHandoff } from "../shared/evaluation";
import { runPreflight } from "../server/preflight";
import { fixture } from "./fixtures";

test("handoff carries requirements/gaps, preserves measurements and validates typed choices", async (t) => {
  const report = await runPreflight(await fixture(t));
  const before = JSON.stringify(report);
  const task = {
    summary: "Verify login UI",
    requiredChecks: [{ id: "health:0", reason: "Login needs backend" }],
    coverageGaps: ["No authorized test login"],
  };
  const input = evaluationInput(task, report);
  assert.deepEqual(input.state.task, task);
  for (const choice of ["proceed", "prepare_environment", "need_more_evidence"]) {
    const result = await evaluateHandoff(task, report, async () => ({
      answers: {
        nextStep: {
          type: "choice",
          choice,
          probabilities: { proceed: 0.1, prepare_environment: 0.1, need_more_evidence: 0.8 },
        },
      },
    }));
    assert.equal(result.evaluation.available, true);
    assert.equal(result.evidence, report);
  }
  for (const answer of [
    { answers: { nextStep: { type: "choice", choice: "repair_now" } } },
    { ok: false, error: { code: "API_ERROR" } },
    {},
  ]) {
    const result = await evaluateHandoff(task, report, async () => answer);
    assert.equal(result.evaluation.available, false);
    assert.ok(!("choice" in result.evaluation));
    assert.ok("result" in result.evaluation && result.evaluation.result === answer);
  }
  assert.equal((await evaluateHandoff(task, report)).evaluation.available, false);
  assert.equal(
    (
      await evaluateHandoff(task, report, async () => {
        throw new Error("secret error");
      })
    ).evaluation.available,
    false,
  );
  const timeout = await evaluateHandoff(task, report, () => new Promise(() => {}), 10);
  assert.deepEqual(timeout.evaluation, { available: false, error: "Evaluator timeout" });
  assert.equal(JSON.stringify(report), before);
});
