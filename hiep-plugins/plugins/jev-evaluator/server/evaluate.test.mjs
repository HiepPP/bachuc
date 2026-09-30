import assert from "node:assert/strict";
import { mkdtemp, rmdir, unlink, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StdioClientTransport } from "@modelcontextprotocol/sdk/client/stdio.js";

import {
  evaluateJev,
  JEV_MODEL,
  MAX_INPUT_BYTES,
  SafeEvaluationError,
  validateJevInput,
} from "./evaluate.mjs";

const serverEntry = path.join(path.dirname(fileURLToPath(import.meta.url)), "mcp.mjs");

test("forwards all question types and returns only public evaluation data", async () => {
  const input = {
    state: { task: "Choose an agent" },
    questions: {
      route: {
        type: "choice",
        instructions: "Who should own this?",
        criteria: { codex: "Backend work", claude: { area: "UI" } },
      },
      risk: {
        type: "score",
        instructions: ["Score risk"],
        criteria: ["low", "medium", "high"],
      },
      retry: {
        type: "boolean",
        instructions: "Should the agent retry?",
        criteria: { true: "Retry", false: "Stop" },
      },
    },
  };
  let received;
  const result = await evaluateJev(input, {
    model: /** @type {import("ai").Experimental_EvaluationModel} */ ("injected-model"),
    evaluateFn: async (options) => {
      received = options;
      return /** @type {any} */ ({
        answers: {
          route: { type: "choice", choice: "codex", probabilities: { codex: 0.8, claude: 0.2 } },
          risk: { type: "score", score: 1.25 },
          retry: { type: "boolean", probability: 0.7 },
        },
        usage: { inputTokens: 100, outputTokens: 12, totalTokens: 112 },
        rounding: { probabilityDecimals: 2 },
        providerMetadata: {
          typesafe: { confidence: { route: 0.91, risk: 0.84 }, planningReasoning: "omit" },
          gateway: { requestId: "omit" },
        },
        response: { body: { secret: "omit" } },
      });
    },
  });

  const call =
    /** @type {{maxRetries: number, state: unknown, questions: unknown, abortSignal: AbortSignal}} */ (
      /** @type {unknown} */ (received)
    );
  assert.equal(call.maxRetries, 0);
  assert.equal(call.state, input.state);
  assert.equal(call.questions, input.questions);
  assert.equal(call.abortSignal.aborted, false);
  assert.deepEqual(result, {
    model: JEV_MODEL,
    answers: {
      route: { type: "choice", choice: "codex", probabilities: { codex: 0.8, claude: 0.2 } },
      risk: { type: "score", score: 1.25 },
      retry: { type: "boolean", probability: 0.7 },
    },
    usage: { inputTokens: 100, outputTokens: 12, totalTokens: 112 },
    rounding: { probabilityDecimals: 2 },
    providerMetadata: { typesafe: { confidence: { route: 0.91, risk: 0.84 } } },
  });
});

test("rejects malformed and oversized inputs before calling the SDK", () => {
  assert.throws(
    () => validateJevInput({ state: {}, questions: {} }),
    (error) => error instanceof SafeEvaluationError && error.code === "INVALID_INPUT",
  );
  assert.throws(
    () =>
      validateJevInput({
        state: {},
        questions: {
          risk: {
            type: "score",
            instructions: "Risk?",
            criteria: Array.from({ length: 11 }, (_, index) => String(index)),
          },
        },
      }),
    /2 to 10 ordered score levels/,
  );
  assert.throws(
    () =>
      validateJevInput({
        state: {},
        questions: {
          route: {
            type: "choice",
            instructions: "Route?",
            criteria: Object.fromEntries(
              Array.from({ length: 256 }, (_, index) => [`option-${index}`, null]),
            ),
          },
        },
      }),
    /1 to 255 choices/,
  );
  assert.throws(
    () =>
      validateJevInput({
        state: { value: Number.POSITIVE_INFINITY },
        questions: { valid: { type: "boolean", instructions: "Valid?" } },
      }),
    /JSON-compatible/,
  );
  assert.throws(
    () =>
      validateJevInput({
        state: "x".repeat(MAX_INPUT_BYTES),
        questions: { valid: { type: "boolean", instructions: "Valid?" } },
      }),
    /must not exceed/,
  );
});

test("maps SDK errors without leaking provider messages", async () => {
  const secret = "vck_secret_should_not_escape";
  await assert.rejects(
    evaluateJev(
      {
        state: "test",
        questions: { valid: { type: "boolean", instructions: "Valid?" } },
      },
      {
        evaluateFn: async () => {
          throw Object.assign(new Error(`credential ${secret}`), { statusCode: 403 });
        },
      },
    ),
    (error) => {
      assert(error instanceof SafeEvaluationError);
      assert.equal(error.code, "ACCESS_DENIED");
      assert.equal(error.status, 403);
      assert.equal(error.message.includes(secret), false);
      return true;
    },
  );
});

test("aborts evaluation at the configured timeout", async () => {
  await assert.rejects(
    evaluateJev(
      {
        state: "test",
        questions: { valid: { type: "boolean", instructions: "Valid?" } },
      },
      {
        timeoutMs: 10,
        evaluateFn: ({ abortSignal }) =>
          new Promise((_, reject) => {
            abortSignal.addEventListener("abort", () => reject(abortSignal.reason), { once: true });
          }),
      },
    ),
    (error) => error instanceof SafeEvaluationError && error.code === "TIMEOUT",
  );
});

test("serves one Jev tool over real stdio without network access", async () => {
  const tempDirectory = await mkdtemp(path.join(os.tmpdir(), "paseo-jev-test-"));
  const configPath = path.join(tempDirectory, "config.json");
  const credentialPath = path.join(tempDirectory, "typesafe-ai.json");
  await writeFile(configPath, "{}");
  await writeFile(credentialPath, JSON.stringify({ apiKey: "fake-test-key" }));
  const client = new Client({ name: "jev-evaluator-test", version: "1.0.0" });
  const transport = new StdioClientTransport({
    command: process.execPath,
    args: [serverEntry],
    env: { PASEO_JEV_CONFIG_PATH: configPath },
    stderr: "pipe",
  });
  try {
    await client.connect(transport);
    const tools = await client.listTools();
    assert.deepEqual(
      tools.tools.map((tool) => tool.name),
      ["jev_evaluate"],
    );

    const result = await client.callTool({
      name: "jev_evaluate",
      arguments: { state: {}, questions: {} },
    });
    assert.equal(result.isError, true);
    const body = JSON.parse(/** @type {any} */ (result).content[0].text);
    assert.equal(body.error.code, "INVALID_INPUT");
    assert.equal(JSON.stringify(body).includes("vck_"), false);
  } finally {
    await client.close();
    await unlink(configPath);
    await unlink(credentialPath);
    await rmdir(tempDirectory);
  }
});
