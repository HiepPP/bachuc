import { readFile } from "node:fs/promises";
import path from "node:path";

import { createTypeSafeAi } from "@ai-sdk/typesafe-ai";

import { evaluateJev, JEV_MODEL, SafeEvaluationError } from "./evaluate.mjs";

/**
 * Read the Typesafe key stored beside the Paseo config file.
 *
 * @param {string} configPath
 */
export async function readTypesafeApiKey(configPath) {
  let config;
  try {
    config = JSON.parse(
      await readFile(path.join(path.dirname(configPath), "typesafe-ai.json"), "utf8"),
    );
  } catch {
    throw new SafeEvaluationError(
      "CONFIG_ERROR",
      500,
      "Unable to read Typesafe credentials for the Jev evaluator.",
    );
  }
  const apiKey = config?.apiKey;
  if (typeof apiKey !== "string" || apiKey.length === 0) {
    throw new SafeEvaluationError(
      "AUTH_CONFIGURATION_ERROR",
      500,
      "Add apiKey to typesafe-ai.json in the Paseo home directory.",
    );
  }
  return apiKey;
}

/**
 * Evaluate with the Typesafe key. Other plugins' workers import this file by path, so they share
 * this plugin's SDK install, credential lookup, and error mapping.
 *
 * @param {string} configPath
 * @param {unknown} input
 * @param {{signal?: AbortSignal, timeoutMs?: number}} [options]
 */
export async function evaluateWithTypesafe(configPath, input, options = {}) {
  const apiKey = await readTypesafeApiKey(configPath);
  return evaluateJev(input, {
    ...options,
    model: createTypeSafeAi({ apiKey }).evaluationModel(JEV_MODEL),
  });
}
