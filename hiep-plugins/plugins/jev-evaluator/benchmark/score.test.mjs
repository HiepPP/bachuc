import assert from "node:assert/strict";
import test from "node:test";

import { renderReport, summarize } from "./score.mjs";

/** @type {Parameters<typeof summarize>[0]} */
const dataset = {
  version: 1,
  batches: [
    {
      id: "logic-a",
      category: "deduction",
      state: {},
      questions: {
        route: {
          type: "choice",
          instructions: "Route?",
          criteria: { yes: null, no: null, unknown: null },
        },
        valid: { type: "boolean", instructions: "Valid?" },
        risk: {
          type: "score",
          instructions: "Risk?",
          criteria: ["none", "low", "medium", "high"],
        },
      },
      expected: {
        route: { answer: "yes", reason: "The premise entails yes." },
        valid: { answer: true, reason: "All required facts are present." },
        risk: { answer: 2, reason: "Two known risk factors." },
      },
    },
    {
      id: "logic-b",
      category: "arithmetic",
      state: {},
      questions: {
        answer: {
          type: "choice",
          instructions: "Known?",
          criteria: { yes: null, no: null, unknown: null },
        },
        safe: { type: "boolean", instructions: "Safe?" },
        severity: {
          type: "score",
          instructions: "Severity?",
          criteria: ["none", "low", "medium", "high"],
        },
      },
      expected: {
        answer: { answer: "unknown", reason: "The state omits one operand." },
        safe: { answer: false, reason: "The guard failed." },
        severity: { answer: 1, reason: "One low-severity issue." },
      },
    },
  ],
};

test("computes type-specific metrics, calibration, and high-certainty errors", () => {
  const summary = summarize(dataset, [
    {
      batchId: "logic-a",
      attempt: 1,
      latencyMs: 100,
      isError: false,
      result: {
        answers: {
          route: {
            type: "choice",
            choice: "yes",
            probabilities: { yes: 0.8, no: 0.1, unknown: 0.1 },
          },
          valid: { type: "boolean", probability: 0.75 },
          risk: { type: "score", score: 1.6 },
        },
        usage: { inputTokens: 10, outputTokens: 2, totalTokens: 12 },
        providerMetadata: { typesafe: { confidence: { route: 0.85, valid: 0.7, risk: 0.8 } } },
      },
    },
    {
      batchId: "logic-b",
      attempt: 1,
      latencyMs: 300,
      isError: false,
      result: {
        answers: {
          answer: {
            type: "choice",
            choice: "no",
            probabilities: { yes: 0.02, no: 0.96, unknown: 0.02 },
          },
          safe: { type: "boolean", probability: 0.95 },
          severity: { type: "score", score: 2.2 },
        },
        usage: { inputTokens: 20, outputTokens: 3, totalTokens: 23 },
        providerMetadata: {
          typesafe: { confidence: { answer: 0.97, safe: 0.95, severity: 0.99 } },
        },
      },
    },
  ]);

  assert.deepEqual(summary.coverage, {
    completedBatches: 2,
    totalBatches: 2,
    evaluableQuestions: 6,
    totalQuestions: 6,
    missingAnswers: 0,
    invalidAnswers: 0,
  });
  assert.equal(summary.choice.accuracy, 0.5);
  assert.equal(summary.choice.brier, 0.9712);
  assert.equal(summary.choice.unknownRecall, 0);
  assert.equal(summary.choice.unknownEvaluableExpected, 1);
  assert.equal(summary.choice.unknownTotalExpected, 1);
  assert.deepEqual(
    summary.choice.optionProbabilityCalibration.map((bin) => [bin.count, bin.accuracy]),
    [
      [0, null],
      [0, null],
      [1, 1],
      [1, 0],
    ],
  );
  assert.deepEqual(
    summary.choice.typesafeConfidenceStrata.map((bin) => [bin.count, bin.accuracy]),
    [
      [0, null],
      [0, null],
      [1, 1],
      [1, 0],
    ],
  );
  assert.equal(summary.boolean.accuracy, 0.5);
  assert(Math.abs((summary.boolean.brier ?? 0) - 0.4825) < Number.EPSILON * 2);
  assert(Math.abs((summary.score.mae ?? 0) - 0.8) < Number.EPSILON * 2);
  assert.equal(summary.score.exactAgreement, 0);
  assert.equal(summary.score.nearestLevelAgreement, 0.5);
  assert.equal(summary.errors.wrongCases.length, 4);
  assert.deepEqual(
    summary.errors.wrongCases
      .filter((item) => item.type === "score")
      .map((item) => [item.questionId, item.expected, item.predicted]),
    [
      ["risk", 2, 1.6],
      ["severity", 1, 2.2],
    ],
  );
  assert.deepEqual(
    summary.errors.highOptionProbability.map((item) => item.questionId),
    ["answer", "safe"],
  );
  assert.deepEqual(
    summary.errors.highTypesafeConfidence.map((item) => item.questionId),
    ["answer", "safe", "severity"],
  );
  assert.deepEqual(summary.usage, { inputTokens: 30, outputTokens: 5, totalTokens: 35 });
  assert.deepEqual(summary.latencyMs, {
    count: 2,
    min: 100,
    p50: 100,
    p95: 300,
    max: 300,
    mean: 200,
  });
});

test("counts a rate-limit retry while scoring only the final success", () => {
  const oneBatch = { ...dataset, batches: [dataset.batches[0]] };
  const summary = summarize(oneBatch, [
    {
      batchId: "logic-a",
      attempt: 1,
      latencyMs: 20,
      isError: true,
      result: {
        ok: false,
        error: { code: "RATE_LIMITED", status: 429, message: "Retry later." },
      },
    },
    {
      batchId: "logic-a",
      attempt: 2,
      latencyMs: 120,
      isError: false,
      result: {
        answers: {
          route: {
            type: "choice",
            choice: "yes",
            probabilities: { yes: 1, no: 0, unknown: 0 },
          },
          valid: { type: "boolean", probability: 1 },
          risk: { type: "score", score: 2 },
        },
        usage: { inputTokens: 8, outputTokens: 2, totalTokens: 10 },
      },
    },
  ]);

  assert.equal(summary.api.attempts, 2);
  assert.equal(summary.api.failedAttempts, 1);
  assert.deepEqual(summary.api.errorsByCode, { RATE_LIMITED: 1 });
  assert.equal(summary.coverage.completedBatches, 1);
  assert.equal(summary.coverage.evaluableQuestions, 3);
  assert.equal(summary.choice.correct, 1);
  assert.equal(summary.boolean.correct, 1);
  assert.equal(summary.score.nearestLevelAgreement, 1);
  assert.deepEqual(summary.usage, { inputTokens: 8, outputTokens: 2, totalTokens: 10 });
  assert.equal(summary.latencyMs.count, 1);
  assert.equal(summary.latencyMs.mean, 120);
});

test("exposes failed batches, missing answers, and report limitations", () => {
  const summary = summarize(dataset, [
    {
      batchId: "logic-a",
      attempt: 1,
      latencyMs: 50,
      isError: false,
      result: {
        answers: {
          route: {
            type: "choice",
            choice: "yes",
            probabilities: { yes: 0.7, no: 0.2, unknown: 0.1 },
          },
          valid: { type: "boolean", probability: "invalid" },
        },
        usage: { inputTokens: 5, outputTokens: 1, totalTokens: 6 },
      },
    },
    {
      batchId: "logic-b",
      attempt: 1,
      latencyMs: 30,
      isError: true,
      result: { ok: false, error: { code: "TIMEOUT", status: 504, message: "Timed out." } },
    },
  ]);

  assert.deepEqual(summary.api.failedBatchIds, ["logic-b"]);
  assert.equal(summary.coverage.evaluableQuestions, 1);
  assert.equal(summary.coverage.missingAnswers, 4);
  assert.equal(summary.coverage.invalidAnswers, 1);
  assert.equal(summary.choice.unknownRecall, null);
  assert.equal(summary.choice.unknownEvaluableExpected, 0);
  assert.equal(summary.choice.unknownTotalExpected, 1);

  const report = renderReport(summary);
  assert.match(report, /mẫu tổng hợp 72 câu hỏi có tương quan/);
  assert.match(report, /không phải calibration production/);
  assert.match(report, /Không tính khoảng tin cậy giả định các câu hỏi độc lập/);
  assert.match(report, /Câu đánh giá được: 1\/6/);
  assert.match(report, /TIMEOUT=1/);
  assert.match(report, /không gộp chúng thành một điểm reliability tùy ý/);
});
