import { createTypeSafeAi } from "@ai-sdk/typesafe-ai";
import { experimental_evaluate as sdkEvaluate } from "ai";

export const JEV_MODEL = "jev-latest";
export const MAX_INPUT_BYTES = 128 * 1024;
export const DEFAULT_TIMEOUT_MS = 45_000;

/** @typedef {import("ai").JSONValue} JsonValue */
/** @typedef {string | JsonValue[] | {[key: string]: JsonValue}} JsonInput */
/** @typedef {string | JsonValue[] | {[key: string]: JsonValue} | null} JsonDescription */
/**
 * @typedef {{type: "choice", instructions: JsonInput, criteria: Record<string, JsonDescription>} | {type: "score", instructions: JsonInput, criteria: JsonDescription[]} | {type: "boolean", instructions: JsonInput, criteria?: {true?: JsonDescription, false?: JsonDescription}}} JevQuestion
 */
/** @typedef {{state: JsonInput, questions: Record<string, JevQuestion>}} JevInput */
/**
 * @typedef {(options: {model: import("ai").Experimental_EvaluationModel, state: import("@ai-sdk/provider").Experimental_EvaluationModelV4Input, questions: Record<string, import("ai").Experimental_EvaluationQuestion>, maxRetries: number, abortSignal: AbortSignal}) => Promise<import("ai").Experimental_EvaluationResult<Record<string, import("ai").Experimental_EvaluationQuestion>>>} EvaluateFunction
 */

export class SafeEvaluationError extends Error {
  /**
   * @param {string} code
   * @param {number} status
   * @param {string} message
   */
  constructor(code, status, message) {
    super(message);
    this.name = "SafeEvaluationError";
    this.code = code;
    this.status = status;
  }
}

/**
 * Validate and evaluate one shared state with Jev.
 *
 * @param {unknown} input
 * @param {{model?: import("ai").Experimental_EvaluationModel, evaluateFn?: EvaluateFunction, signal?: AbortSignal, timeoutMs?: number}} [options]
 */
export async function evaluateJev(input, options = {}) {
  const validated = validateJevInput(input);
  const timeoutMs = options.timeoutMs ?? DEFAULT_TIMEOUT_MS;
  if (!Number.isInteger(timeoutMs) || timeoutMs < 1) {
    throw new SafeEvaluationError("INVALID_INPUT", 400, "timeoutMs must be a positive integer.");
  }

  const controller = new AbortController();
  let timedOut = false;
  const abortFromCaller = () => controller.abort(options.signal?.reason);
  if (options.signal?.aborted) abortFromCaller();
  else options.signal?.addEventListener("abort", abortFromCaller, { once: true });

  const timer = setTimeout(() => {
    timedOut = true;
    controller.abort();
  }, timeoutMs);
  timer.unref?.();

  try {
    const result = await (options.evaluateFn ?? sdkEvaluate)({
      model: options.model ?? createTypeSafeAi().evaluationModel(JEV_MODEL),
      state: validated.state,
      questions: validated.questions,
      maxRetries: 0,
      abortSignal: controller.signal,
    });
    return toPublicResult(result);
  } catch (error) {
    if (error instanceof SafeEvaluationError) throw error;
    if (timedOut) {
      throw new SafeEvaluationError("TIMEOUT", 504, `Jev evaluation exceeded ${timeoutMs} ms.`);
    }
    if (options.signal?.aborted) {
      throw new SafeEvaluationError("CANCELLED", 499, "Jev evaluation was cancelled.");
    }
    throw safeSdkError(error);
  } finally {
    clearTimeout(timer);
    options.signal?.removeEventListener("abort", abortFromCaller);
  }
}

/** @param {unknown} input @returns {JevInput} */
export function validateJevInput(input) {
  if (!isPlainRecord(input)) invalid("input must be an object.");
  const { state, questions } = input;
  if (!isJsonInput(state)) {
    invalid("state must be a JSON-compatible string, object, or array.");
  }
  if (!isPlainRecord(questions) || Object.keys(questions).length === 0) {
    invalid("questions must be a nonempty map of named questions.");
  }

  for (const [id, question] of Object.entries(questions)) {
    if (id.length === 0) invalid("question IDs must not be empty.");
    if (!isPlainRecord(question) || !isJsonInput(question.instructions)) {
      invalid(`questions.${id}.instructions must be a JSON-compatible string, object, or array.`);
    }
    switch (question.type) {
      case "choice":
        validateChoice(id, question.criteria);
        break;
      case "score":
        validateScore(id, question.criteria);
        break;
      case "boolean":
        validateBoolean(id, question.criteria);
        break;
      default:
        invalid(`questions.${id}.type must be choice, score, or boolean.`);
    }
  }

  const encoded = JSON.stringify(input);
  if (Buffer.byteLength(encoded, "utf8") > MAX_INPUT_BYTES) {
    invalid(`input must not exceed ${MAX_INPUT_BYTES} bytes as JSON.`);
  }
  return /** @type {JevInput} */ (input);
}

/** @param {string} id @param {unknown} criteria */
function validateChoice(id, criteria) {
  if (
    !isPlainRecord(criteria) ||
    Object.keys(criteria).length === 0 ||
    Object.keys(criteria).length > 255
  ) {
    invalid(`questions.${id}.criteria must contain 1 to 255 choices.`);
  }
  for (const [option, description] of Object.entries(criteria)) {
    if (option.length === 0) invalid(`questions.${id}.criteria contains an empty option name.`);
    if (!isDescription(description)) {
      invalid(`questions.${id}.criteria.${option} must be a JSON description or null.`);
    }
  }
}

/** @param {string} id @param {unknown} criteria */
function validateScore(id, criteria) {
  if (!Array.isArray(criteria) || criteria.length < 2 || criteria.length > 10) {
    invalid(`questions.${id}.criteria must contain 2 to 10 ordered score levels.`);
  }
  if (!criteria.every(isDescription)) {
    invalid(`questions.${id}.criteria levels must be JSON descriptions or null.`);
  }
}

/** @param {string} id @param {unknown} criteria */
function validateBoolean(id, criteria) {
  if (criteria === undefined) return;
  if (
    !isPlainRecord(criteria) ||
    Object.keys(criteria).some((key) => key !== "true" && key !== "false") ||
    !Object.values(criteria).every(isDescription)
  ) {
    invalid(
      `questions.${id}.criteria may only describe true and false with JSON descriptions or null.`,
    );
  }
}

/** @param {unknown} result */
function toPublicResult(result) {
  if (!isPlainRecord(result) || !isPlainRecord(result.answers)) {
    throw new SafeEvaluationError(
      "INVALID_RESPONSE",
      502,
      "Jev returned an invalid evaluation result.",
    );
  }
  const usage = isPlainRecord(result.usage) ? result.usage : {};
  const output = {
    model: JEV_MODEL,
    answers: result.answers,
    usage: {
      inputTokens: finiteNumberOrNull(usage.inputTokens),
      outputTokens: finiteNumberOrNull(usage.outputTokens),
      totalTokens: finiteNumberOrNull(usage.totalTokens),
    },
  };

  if (isPlainRecord(result.rounding) && isJsonValue(result.rounding)) {
    Object.assign(output, { rounding: result.rounding });
  }
  const metadata = result.providerMetadata;
  const confidence =
    isPlainRecord(metadata) && isPlainRecord(metadata.typesafe)
      ? metadata.typesafe.confidence
      : undefined;
  if (confidence !== undefined && isJsonValue(confidence)) {
    Object.assign(output, { providerMetadata: { typesafe: { confidence } } });
  }
  return output;
}

/** @param {unknown} error */
function safeSdkError(error) {
  const statusCode =
    error !== null && typeof error === "object" && "statusCode" in error
      ? error.statusCode
      : undefined;
  const status =
    typeof statusCode === "number" &&
    Number.isInteger(statusCode) &&
    statusCode >= 400 &&
    statusCode <= 599
      ? statusCode
      : 502;
  if (status === 401) {
    return new SafeEvaluationError(
      "AUTHENTICATION_FAILED",
      status,
      "Typesafe AI rejected the configured credential.",
    );
  }
  if (status === 403) {
    return new SafeEvaluationError(
      "ACCESS_DENIED",
      status,
      "Typesafe denied access; check key permissions and model access.",
    );
  }
  if (status === 402) {
    return new SafeEvaluationError(
      "PAYMENT_REQUIRED",
      status,
      "Typesafe AI requires available credits.",
    );
  }
  if (status === 429) {
    return new SafeEvaluationError(
      "RATE_LIMITED",
      status,
      "Typesafe AI rate limit reached. Retry later.",
    );
  }
  if (status === 400 || status === 404 || status === 422) {
    return new SafeEvaluationError(
      "GATEWAY_REQUEST_REJECTED",
      status,
      "Typesafe AI rejected the Jev evaluation request.",
    );
  }
  return new SafeEvaluationError(
    "GATEWAY_UNAVAILABLE",
    status,
    "Jev evaluation is temporarily unavailable.",
  );
}

/** @param {string} message @returns {never} */
function invalid(message) {
  throw new SafeEvaluationError("INVALID_INPUT", 400, message);
}

/** @param {unknown} value */
function finiteNumberOrNull(value) {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

/** @param {unknown} value @returns {value is Record<string, unknown>} */
function isPlainRecord(value) {
  if (value === null || typeof value !== "object" || Array.isArray(value)) return false;
  const prototype = Object.getPrototypeOf(value);
  return prototype === Object.prototype || prototype === null;
}

/** @param {unknown} value */
function isJsonInput(value) {
  return (
    (typeof value === "string" || Array.isArray(value) || isPlainRecord(value)) &&
    isJsonValue(value)
  );
}

/** @param {unknown} value */
function isDescription(value) {
  return value === null || isJsonInput(value);
}

/** @param {unknown} value @param {Set<object>} [ancestors] @returns {value is JsonValue} */
function isJsonValue(value, ancestors = new Set()) {
  if (value === null || typeof value === "string" || typeof value === "boolean") return true;
  if (typeof value === "number") return Number.isFinite(value);
  if (typeof value !== "object" || (!Array.isArray(value) && !isPlainRecord(value))) return false;
  if (ancestors.has(value)) return false;
  ancestors.add(value);
  /** @type {boolean} */
  const valid = Array.isArray(value)
    ? value.every((item) => isJsonValue(item, ancestors))
    : Object.getOwnPropertySymbols(value).length === 0 &&
      Object.values(value).every((item) => isJsonValue(item, ancestors));
  ancestors.delete(value);
  return valid;
}
