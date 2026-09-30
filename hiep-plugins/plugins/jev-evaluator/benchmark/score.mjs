/**
 * @typedef {{type: "choice", instructions: unknown, criteria: Record<string, unknown>} | {type: "boolean", instructions: unknown, criteria?: Record<string, unknown>} | {type: "score", instructions: unknown, criteria: unknown[]}} BenchmarkQuestion
 * @typedef {{answer: string | boolean | number, reason: string}} ExpectedAnswer
 * @typedef {{id: string, category: string, state: unknown, questions: Record<string, BenchmarkQuestion>, expected: Record<string, ExpectedAnswer>}} BenchmarkBatch
 * @typedef {{version: 1, batches: BenchmarkBatch[]}} BenchmarkDataset
 * @typedef {{batchId: string, attempt: number, latencyMs: number, result: unknown, isError: boolean}} BenchmarkRecord
 * @typedef {{label: string, count: number, meanConfidence: number | null, accuracy: number | null}} CalibrationBin
 * @typedef {{batchId: string, questionId: string, category: string, type: "choice" | "boolean" | "score", expected: string | boolean | number, predicted: string | boolean | number, probability: number | null, confidence: number | null, reason: string}} WrongCase
 */

const CALIBRATION_BINS = [
  { label: "[0.0, 0.6)", min: 0, max: 0.6, includeMax: false },
  { label: "[0.6, 0.8)", min: 0.6, max: 0.8, includeMax: false },
  { label: "[0.8, 0.9)", min: 0.8, max: 0.9, includeMax: false },
  { label: "[0.9, 1.0]", min: 0.9, max: 1, includeMax: true },
];

/**
 * Score a benchmark run. Only the last successful attempt for each batch contributes answers,
 * tokens, and latency. Every failed attempt remains visible in api errors.
 *
 * @param {BenchmarkDataset} dataset
 * @param {BenchmarkRecord[]} records
 */
export function summarize(dataset, records) {
  validateDataset(dataset);

  const batchesById = new Map(dataset.batches.map((batch) => [batch.id, batch]));
  /** @type {Map<string, BenchmarkRecord>} */
  const finalSuccessByBatch = new Map();
  /** @type {Map<string, number>} */
  const errorsByCode = new Map();
  let failedAttempts = 0;

  for (const record of records) {
    if (!batchesById.has(record.batchId)) {
      throw new Error(`Record references unknown batch: ${record.batchId}`);
    }
    if (isErrorRecord(record)) {
      failedAttempts += 1;
      const code = errorCode(record.result);
      errorsByCode.set(code, (errorsByCode.get(code) ?? 0) + 1);
      continue;
    }
    if (!isSuccessfulResult(record.result)) {
      failedAttempts += 1;
      errorsByCode.set("INVALID_RESULT", (errorsByCode.get("INVALID_RESULT") ?? 0) + 1);
      continue;
    }
    const previous = finalSuccessByBatch.get(record.batchId);
    if (!previous || record.attempt >= previous.attempt) {
      finalSuccessByBatch.set(record.batchId, record);
    }
  }

  const metrics = createMetrics();

  for (const batch of dataset.batches) {
    const record = finalSuccessByBatch.get(batch.id);
    const result = record && isSuccessfulResult(record.result) ? record.result : null;
    const confidenceMap = result ? getConfidenceMap(result) : null;

    for (const [questionId, question] of Object.entries(batch.questions)) {
      metrics.totalQuestions += 1;
      const expected = batch.expected[questionId];
      if (!expected) throw new Error(`Missing expected answer: ${batch.id}.${questionId}`);
      const answer = result?.answers[questionId];
      const confidence = probabilityOrNull(confidenceMap?.[questionId]);
      scoreQuestion(metrics, batch, questionId, question, expected, answer, confidence);
    }
  }

  const successfulRecords = [...finalSuccessByBatch.values()];
  const usage = successfulRecords.reduce(
    (totals, record) => {
      const result = isSuccessfulResult(record.result) ? record.result : null;
      totals.inputTokens += tokenCount(result?.usage?.inputTokens);
      totals.outputTokens += tokenCount(result?.usage?.outputTokens);
      totals.totalTokens += tokenCount(result?.usage?.totalTokens);
      return totals;
    },
    { inputTokens: 0, outputTokens: 0, totalTokens: 0 },
  );
  const failedBatchIds = dataset.batches
    .filter((batch) => !finalSuccessByBatch.has(batch.id))
    .map((batch) => batch.id);

  return {
    dataset: {
      version: dataset.version,
      batches: dataset.batches.length,
      totalQuestions: metrics.totalQuestions,
    },
    coverage: {
      completedBatches: finalSuccessByBatch.size,
      totalBatches: dataset.batches.length,
      evaluableQuestions: metrics.evaluableQuestions,
      totalQuestions: metrics.totalQuestions,
      missingAnswers: metrics.missingAnswers,
      invalidAnswers: metrics.invalidAnswers,
    },
    api: {
      attempts: records.length,
      failedAttempts,
      failedBatchIds,
      errorsByCode: Object.fromEntries([...errorsByCode.entries()].sort()),
    },
    choice: {
      total: metrics.choice.total,
      evaluable: metrics.choice.evaluable,
      correct: metrics.choice.correct,
      accuracy: ratio(metrics.choice.correct, metrics.choice.evaluable),
      brier: average(metrics.choice.brierSum, metrics.choice.evaluable),
      unknownRecall: ratio(metrics.choice.unknownCorrect, metrics.choice.unknownEvaluableExpected),
      unknownCorrect: metrics.choice.unknownCorrect,
      unknownEvaluableExpected: metrics.choice.unknownEvaluableExpected,
      unknownTotalExpected: metrics.choice.unknownTotalExpected,
      byFamily: [...metrics.choice.families.entries()]
        .sort(([left], [right]) => left.localeCompare(right))
        .map(([family, values]) => ({
          family,
          total: values.total,
          evaluable: values.evaluable,
          correct: values.correct,
          accuracy: ratio(values.correct, values.evaluable),
          brier: average(values.brierSum, values.evaluable),
        })),
      optionProbabilityCalibration: calibration(metrics.choice.optionCalibrationPoints),
      typesafeConfidenceStrata: calibration(metrics.choice.metadataCalibrationPoints),
    },
    boolean: {
      total: metrics.boolean.total,
      evaluable: metrics.boolean.evaluable,
      correct: metrics.boolean.correct,
      accuracy: ratio(metrics.boolean.correct, metrics.boolean.evaluable),
      brier: average(metrics.boolean.brierSum, metrics.boolean.evaluable),
    },
    score: {
      total: metrics.score.total,
      evaluable: metrics.score.evaluable,
      mae: average(metrics.score.absoluteErrorSum, metrics.score.evaluable),
      exactAgreement: ratio(metrics.score.exact, metrics.score.evaluable),
      nearestLevelAgreement: ratio(metrics.score.nearest, metrics.score.evaluable),
    },
    errors: {
      wrongCases: metrics.errors.wrongCases,
      highOptionProbability: metrics.errors.highOptionProbability,
      highTypesafeConfidence: metrics.errors.highTypesafeConfidence,
    },
    usage,
    latencyMs: latencyDistribution(successfulRecords.map((record) => record.latencyMs)),
  };
}

function createMetrics() {
  return {
    totalQuestions: 0,
    evaluableQuestions: 0,
    missingAnswers: 0,
    invalidAnswers: 0,
    choice: {
      total: 0,
      evaluable: 0,
      correct: 0,
      brierSum: 0,
      unknownTotalExpected: 0,
      unknownEvaluableExpected: 0,
      unknownCorrect: 0,
      /** @type {Map<string, {total: number, evaluable: number, correct: number, brierSum: number}>} */
      families: new Map(),
      /** @type {{confidence: number, correct: boolean}[]} */
      optionCalibrationPoints: [],
      /** @type {{confidence: number, correct: boolean}[]} */
      metadataCalibrationPoints: [],
    },
    boolean: { total: 0, evaluable: 0, correct: 0, brierSum: 0 },
    score: { total: 0, evaluable: 0, absoluteErrorSum: 0, exact: 0, nearest: 0 },
    errors: {
      /** @type {WrongCase[]} */
      wrongCases: [],
      /** @type {WrongCase[]} */
      highOptionProbability: [],
      /** @type {WrongCase[]} */
      highTypesafeConfidence: [],
    },
  };
}

/** @param {ReturnType<typeof createMetrics>} metrics @param {BenchmarkBatch} batch @param {string} questionId @param {BenchmarkQuestion} question @param {ExpectedAnswer} expected @param {unknown} answer @param {number | null} confidence */
function scoreQuestion(metrics, batch, questionId, question, expected, answer, confidence) {
  if (question.type === "choice") {
    scoreChoice(metrics, batch, questionId, question, expected, answer, confidence);
    return;
  }
  if (question.type === "boolean") {
    scoreBoolean(metrics, batch, questionId, expected, answer, confidence);
    return;
  }
  scoreLevel(metrics, batch, questionId, question, expected, answer, confidence);
}

/** @param {ReturnType<typeof createMetrics>} metrics @param {BenchmarkBatch} batch @param {string} questionId @param {Extract<BenchmarkQuestion, {type: "choice"}>} question @param {ExpectedAnswer} expected @param {unknown} answer @param {number | null} confidence */
function scoreChoice(metrics, batch, questionId, question, expected, answer, confidence) {
  metrics.choice.total += 1;
  const family = metrics.choice.families.get(batch.category) ?? {
    total: 0,
    evaluable: 0,
    correct: 0,
    brierSum: 0,
  };
  family.total += 1;
  metrics.choice.families.set(batch.category, family);
  if (expected.answer === "unknown") metrics.choice.unknownTotalExpected += 1;
  if (answer === undefined) {
    metrics.missingAnswers += 1;
    return;
  }
  const parsed = parseChoiceAnswer(answer, Object.keys(question.criteria));
  if (!parsed || typeof expected.answer !== "string") {
    metrics.invalidAnswers += 1;
    return;
  }

  metrics.evaluableQuestions += 1;
  metrics.choice.evaluable += 1;
  family.evaluable += 1;
  if (expected.answer === "unknown") metrics.choice.unknownEvaluableExpected += 1;
  const correct = parsed.choice === expected.answer;
  const brier = multiclassBrier(parsed.probabilities, expected.answer);
  metrics.choice.brierSum += brier;
  family.brierSum += brier;
  if (correct) recordCorrectChoice(metrics, family, expected.answer);
  const selectedProbability = parsed.probabilities[parsed.choice];
  metrics.choice.optionCalibrationPoints.push({ confidence: selectedProbability, correct });
  if (confidence !== null) {
    metrics.choice.metadataCalibrationPoints.push({ confidence, correct });
  }
  if (correct) return;
  const wrong = wrongCase(
    batch,
    questionId,
    "choice",
    expected,
    parsed.choice,
    selectedProbability,
    confidence,
  );
  recordWrong(metrics, wrong, selectedProbability);
}

/** @param {ReturnType<typeof createMetrics>} metrics @param {{correct: number}} family @param {string} expected */
function recordCorrectChoice(metrics, family, expected) {
  metrics.choice.correct += 1;
  family.correct += 1;
  if (expected === "unknown") metrics.choice.unknownCorrect += 1;
}

/** @param {ReturnType<typeof createMetrics>} metrics @param {BenchmarkBatch} batch @param {string} questionId @param {ExpectedAnswer} expected @param {unknown} answer @param {number | null} confidence */
function scoreBoolean(metrics, batch, questionId, expected, answer, confidence) {
  metrics.boolean.total += 1;
  if (answer === undefined) {
    metrics.missingAnswers += 1;
    return;
  }
  const probability = parseBooleanAnswer(answer);
  if (probability === null || typeof expected.answer !== "boolean") {
    metrics.invalidAnswers += 1;
    return;
  }

  metrics.evaluableQuestions += 1;
  metrics.boolean.evaluable += 1;
  const predicted = probability >= 0.5;
  const correct = predicted === expected.answer;
  const target = expected.answer ? 1 : 0;
  metrics.boolean.brierSum += (probability - target) ** 2;
  if (correct) {
    metrics.boolean.correct += 1;
    return;
  }
  const predictedProbability = predicted ? probability : 1 - probability;
  const wrong = wrongCase(
    batch,
    questionId,
    "boolean",
    expected,
    predicted,
    predictedProbability,
    confidence,
  );
  recordWrong(metrics, wrong, predictedProbability);
}

/** @param {ReturnType<typeof createMetrics>} metrics @param {BenchmarkBatch} batch @param {string} questionId @param {Extract<BenchmarkQuestion, {type: "score"}>} question @param {ExpectedAnswer} expected @param {unknown} answer @param {number | null} confidence */
function scoreLevel(metrics, batch, questionId, question, expected, answer, confidence) {
  metrics.score.total += 1;
  if (answer === undefined) {
    metrics.missingAnswers += 1;
    return;
  }
  const score = parseScoreAnswer(answer, question.criteria.length);
  if (score === null || typeof expected.answer !== "number") {
    metrics.invalidAnswers += 1;
    return;
  }

  metrics.evaluableQuestions += 1;
  metrics.score.evaluable += 1;
  metrics.score.absoluteErrorSum += Math.abs(score - expected.answer);
  const exact = score === expected.answer;
  if (exact) metrics.score.exact += 1;
  if (Math.round(score) === expected.answer) metrics.score.nearest += 1;
  if (exact) return;
  const wrong = wrongCase(batch, questionId, "score", expected, score, null, confidence);
  recordWrong(metrics, wrong, null);
}

/** @param {ReturnType<typeof createMetrics>} metrics @param {WrongCase} wrong @param {number | null} predictionProbability */
function recordWrong(metrics, wrong, predictionProbability) {
  metrics.errors.wrongCases.push(wrong);
  if (predictionProbability !== null && predictionProbability >= 0.9) {
    metrics.errors.highOptionProbability.push(wrong);
  }
  if (wrong.confidence !== null && wrong.confidence >= 0.9) {
    metrics.errors.highTypesafeConfidence.push(wrong);
  }
}

/** @param {ReturnType<typeof summarize>} summary */
export function renderReport(summary) {
  const lines = [
    "# Báo cáo benchmark Jev",
    "",
    `Benchmark được thiết kế cho mẫu tổng hợp 72 câu hỏi có tương quan; lần chạy này chấm ${summary.dataset.totalQuestions} câu. Kết quả không phải calibration production hay kết luận độ tin cậy rộng. Không tính khoảng tin cậy giả định các câu hỏi độc lập.`,
    "",
    "## Coverage",
    "",
    `- Batch hoàn tất: ${summary.coverage.completedBatches}/${summary.coverage.totalBatches}`,
    `- Câu đánh giá được: ${summary.coverage.evaluableQuestions}/${summary.coverage.totalQuestions}`,
    `- Thiếu câu trả lời: ${summary.coverage.missingAnswers}`,
    `- Câu trả lời sai schema/range: ${summary.coverage.invalidAnswers}`,
    `- Lượt API lỗi: ${summary.api.failedAttempts}/${summary.api.attempts}`,
    "",
    "## Choice",
    "",
    `- Accuracy: ${percent(summary.choice.accuracy)} (${summary.choice.correct}/${summary.choice.evaluable})`,
    `- Multiclass Brier (0 tốt nhất, 2 tệ nhất): ${decimal(summary.choice.brier)}`,
    `- Recall nhãn unknown trong phần đánh giá được: ${percent(summary.choice.unknownRecall)} (${summary.choice.unknownCorrect}/${summary.choice.unknownEvaluableExpected}); tổng câu có nhãn unknown: ${summary.choice.unknownTotalExpected}`,
    "",
    "| Nhóm luận lý | Đánh giá được | Accuracy | Brier |",
    "| --- | ---: | ---: | ---: |",
    ...summary.choice.byFamily.map(
      (family) =>
        `| ${markdown(family.family)} | ${family.evaluable}/${family.total} | ${percent(family.accuracy)} | ${decimal(family.brier)} |`,
    ),
    "",
    "### Calibration theo xác suất nhãn được chọn",
    "",
    ...calibrationTable(summary.choice.optionProbabilityCalibration),
    "",
    "### Phân tầng theo `typesafe.confidence`",
    "",
    "Metric này tách khỏi xác suất nhãn được chọn.",
    "",
    ...calibrationTable(summary.choice.typesafeConfidenceStrata),
    "",
    "## Boolean",
    "",
    `- Accuracy tại ngưỡng 0.5: ${percent(summary.boolean.accuracy)} (${summary.boolean.correct}/${summary.boolean.evaluable})`,
    `- Brier: ${decimal(summary.boolean.brier)}`,
    "",
    "## Score",
    "",
    `- MAE: ${decimal(summary.score.mae)}`,
    `- Khớp chính xác: ${percent(summary.score.exactAgreement)}`,
    `- Khớp level gần nhất: ${percent(summary.score.nearestLevelAgreement)}`,
    "",
    "## Lỗi chắc chắn cao",
    "",
    `- Xác suất nhãn/dự đoán >= 0.9 nhưng sai: ${summary.errors.highOptionProbability.length}`,
    `- \`typesafe.confidence\` >= 0.9 nhưng sai: ${summary.errors.highTypesafeConfidence.length}`,
    "",
    "## Tất cả câu sai",
    "",
    ...wrongCasesTable(summary.errors.wrongCases),
    "",
    "## API, token và latency",
    "",
    `- Batch không có kết quả thành công: ${summary.api.failedBatchIds.length}${summary.api.failedBatchIds.length ? ` (${summary.api.failedBatchIds.map(markdown).join(", ")})` : ""}`,
    `- Lỗi theo mã: ${
      Object.entries(summary.api.errorsByCode)
        .map(([code, count]) => `${markdown(code)}=${count}`)
        .join(", ") || "không có"
    }`,
    `- Token: input ${summary.usage.inputTokens}, output ${summary.usage.outputTokens}, total ${summary.usage.totalTokens}`,
    `- Latency thành công (ms): n=${summary.latencyMs.count}, min=${decimal(summary.latencyMs.min)}, p50=${decimal(summary.latencyMs.p50)}, p95=${decimal(summary.latencyMs.p95)}, max=${decimal(summary.latencyMs.max)}, mean=${decimal(summary.latencyMs.mean)}`,
    "",
    "Diễn giải từng metric riêng. Báo cáo không gộp chúng thành một điểm reliability tùy ý.",
  ];
  return `${lines.join("\n")}\n`;
}

/** @param {BenchmarkDataset} dataset */
function validateDataset(dataset) {
  if (!dataset || dataset.version !== 1 || !Array.isArray(dataset.batches)) {
    throw new Error("Dataset must have version 1 and a batches array.");
  }
  const ids = new Set();
  for (const batch of dataset.batches) {
    if (!batch.id || ids.has(batch.id))
      throw new Error(`Invalid or duplicate batch id: ${batch.id}`);
    ids.add(batch.id);
    for (const [questionId, question] of Object.entries(batch.questions)) {
      if (!question || !["choice", "boolean", "score"].includes(question.type)) {
        throw new Error(`Invalid question type: ${batch.id}.${questionId}`);
      }
    }
  }
}

/** @param {BenchmarkRecord} record */
function isErrorRecord(record) {
  return record.isError || (isRecord(record.result) && record.result.ok === false);
}

/** @param {unknown} result @returns {result is {answers: Record<string, unknown>, usage?: Record<string, unknown>, providerMetadata?: Record<string, unknown>}} */
function isSuccessfulResult(result) {
  return isRecord(result) && result.ok !== false && isRecord(result.answers);
}

/** @param {unknown} result */
function errorCode(result) {
  if (isRecord(result) && isRecord(result.error) && typeof result.error.code === "string") {
    return result.error.code;
  }
  return "UNKNOWN_ERROR";
}

/** @param {unknown} result */
function getConfidenceMap(result) {
  if (!isRecord(result) || !isRecord(result.providerMetadata)) return null;
  const typesafe = result.providerMetadata.typesafe;
  if (!isRecord(typesafe) || !isRecord(typesafe.confidence)) return null;
  return typesafe.confidence;
}

/** @param {unknown} answer @param {string[]} options */
function parseChoiceAnswer(answer, options) {
  if (!isRecord(answer) || answer.type !== "choice" || typeof answer.choice !== "string") {
    return null;
  }
  if (!options.includes(answer.choice) || !isRecord(answer.probabilities)) return null;
  /** @type {Record<string, number>} */
  const probabilities = {};
  for (const option of options) {
    const probability = probabilityOrNull(answer.probabilities[option]);
    if (probability === null) return null;
    probabilities[option] = probability;
  }
  return { choice: answer.choice, probabilities };
}

/** @param {unknown} answer */
function parseBooleanAnswer(answer) {
  if (!isRecord(answer) || answer.type !== "boolean") return null;
  return probabilityOrNull(answer.probability);
}

/** @param {unknown} answer @param {number} levelCount */
function parseScoreAnswer(answer, levelCount) {
  if (!isRecord(answer) || answer.type !== "score" || !Number.isFinite(answer.score)) return null;
  const score = /** @type {number} */ (answer.score);
  return score >= 0 && score <= levelCount - 1 ? score : null;
}

/** @param {Record<string, number>} probabilities @param {string} expected */
function multiclassBrier(probabilities, expected) {
  return Object.entries(probabilities).reduce(
    (sum, [option, probability]) => sum + (probability - (option === expected ? 1 : 0)) ** 2,
    0,
  );
}

/** @param {{confidence: number, correct: boolean}[]} points @returns {CalibrationBin[]} */
function calibration(points) {
  return CALIBRATION_BINS.map((bin) => {
    const selected = points.filter(
      (point) =>
        point.confidence >= bin.min &&
        (bin.includeMax ? point.confidence <= bin.max : point.confidence < bin.max),
    );
    return {
      label: bin.label,
      count: selected.length,
      meanConfidence: selected.length
        ? selected.reduce((sum, point) => sum + point.confidence, 0) / selected.length
        : null,
      accuracy: selected.length
        ? selected.filter((point) => point.correct).length / selected.length
        : null,
    };
  });
}

/** @param {BenchmarkBatch} batch @param {string} questionId @param {"choice" | "boolean" | "score"} type @param {ExpectedAnswer} expected @param {string | boolean | number} predicted @param {number | null} probability @param {number | null} confidence @returns {WrongCase} */
function wrongCase(batch, questionId, type, expected, predicted, probability, confidence) {
  return {
    batchId: batch.id,
    questionId,
    category: batch.category,
    type,
    expected: expected.answer,
    predicted,
    probability,
    confidence,
    reason: expected.reason,
  };
}

/** @param {number[]} latencies */
function latencyDistribution(latencies) {
  const values = latencies
    .filter((value) => Number.isFinite(value) && value >= 0)
    .sort((left, right) => left - right);
  if (values.length === 0) {
    return { count: 0, min: null, p50: null, p95: null, max: null, mean: null };
  }
  return {
    count: values.length,
    min: values[0],
    p50: percentile(values, 0.5),
    p95: percentile(values, 0.95),
    max: values.at(-1) ?? null,
    mean: values.reduce((sum, value) => sum + value, 0) / values.length,
  };
}

/** @param {number[]} sorted @param {number} quantile */
function percentile(sorted, quantile) {
  return sorted[Math.ceil(sorted.length * quantile) - 1] ?? null;
}

/** @param {number} numerator @param {number} denominator */
function ratio(numerator, denominator) {
  return denominator === 0 ? null : numerator / denominator;
}

/** @param {number} sum @param {number} count */
function average(sum, count) {
  return count === 0 ? null : sum / count;
}

/** @param {unknown} value */
function probabilityOrNull(value) {
  return typeof value === "number" && Number.isFinite(value) && value >= 0 && value <= 1
    ? value
    : null;
}

/** @param {unknown} value */
function tokenCount(value) {
  return typeof value === "number" && Number.isFinite(value) && value >= 0 ? value : 0;
}

/** @param {unknown} value @returns {value is Record<string, unknown>} */
function isRecord(value) {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

/** @param {CalibrationBin[]} bins */
function calibrationTable(bins) {
  return [
    "| Khoảng | Số câu | Xác suất/confidence TB | Accuracy quan sát |",
    "| --- | ---: | ---: | ---: |",
    ...bins.map(
      (bin) =>
        `| ${bin.label} | ${bin.count} | ${decimal(bin.meanConfidence)} | ${percent(bin.accuracy)} |`,
    ),
  ];
}

/** @param {WrongCase[]} cases */
function wrongCasesTable(cases) {
  if (cases.length === 0) return ["Không có câu sai trong phần đánh giá được."];
  return [
    "| Batch.câu | Nhóm | Loại | Kỳ vọng | Dự đoán | Xác suất | Confidence | Lý do nhãn |",
    "| --- | --- | --- | --- | --- | ---: | ---: | --- |",
    ...cases.map(
      (item) =>
        `| ${markdown(`${item.batchId}.${item.questionId}`)} | ${markdown(item.category)} | ${item.type} | ${markdown(String(item.expected))} | ${markdown(String(item.predicted))} | ${decimal(item.probability)} | ${decimal(item.confidence)} | ${markdown(item.reason)} |`,
    ),
  ];
}

/** @param {number | null} value */
function percent(value) {
  return value === null ? "n/a" : `${(value * 100).toFixed(1)}%`;
}

/** @param {number | null} value */
function decimal(value) {
  return value === null ? "n/a" : value.toFixed(3);
}

/** @param {string} value */
function markdown(value) {
  return value.replaceAll("|", "\\|").replaceAll("\n", " ");
}
