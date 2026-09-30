#!/usr/bin/env node

import { readFile } from "node:fs/promises";
import path from "node:path";

import { createTypeSafeAi } from "@ai-sdk/typesafe-ai";
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { z } from "zod";

import { evaluateJev, JEV_MODEL, SafeEvaluationError } from "./evaluate.mjs";

/** @type {z.ZodType<import("ai").JSONValue>} */
const jsonValueSchema = z.lazy(() =>
  z.union([
    z.null(),
    z.boolean(),
    z.number().finite(),
    z.string(),
    z.array(jsonValueSchema),
    z.record(z.string(), jsonValueSchema),
  ]),
);
const jsonInputSchema = z.union([
  z.string(),
  z.array(jsonValueSchema),
  z.record(z.string(), jsonValueSchema),
]);
const descriptionSchema = z.union([jsonInputSchema, z.null()]);
const questionSchema = z.discriminatedUnion("type", [
  z.object({
    type: z.literal("choice"),
    instructions: jsonInputSchema,
    criteria: z
      .record(z.string(), descriptionSchema)
      .refine((criteria) => Object.keys(criteria).length >= 1, "Add at least one choice.")
      .refine((criteria) => Object.keys(criteria).length <= 255, "Use at most 255 choices."),
  }),
  z.object({
    type: z.literal("score"),
    instructions: jsonInputSchema,
    criteria: z.array(descriptionSchema).min(2).max(10),
  }),
  z.object({
    type: z.literal("boolean"),
    instructions: jsonInputSchema,
    criteria: z
      .object({ true: descriptionSchema.optional(), false: descriptionSchema.optional() })
      .strict()
      .optional(),
  }),
]);

const server = new McpServer({ name: "paseo-jev-evaluator", version: "0.1.0" });

server.registerTool(
  "jev_evaluate",
  {
    description:
      "Ask Jev to evaluate named boolean, choice, or score questions against shared JSON state. Use for routing, completion checks, retry decisions, and risk scores; Jev does not generate text.",
    inputSchema: {
      state: jsonInputSchema.describe("Shared JSON state Jev should evaluate."),
      questions: z
        .record(z.string(), questionSchema)
        .describe("Nonempty map of question IDs to typed evaluation questions."),
    },
    annotations: {
      readOnlyHint: true,
      destructiveHint: false,
      idempotentHint: true,
      openWorldHint: true,
    },
  },
  async (input, extra) => {
    try {
      const apiKey = await readTypesafeApiKey();
      const provider = createTypeSafeAi({ apiKey });
      const result = await evaluateJev(input, {
        model: provider.evaluationModel(JEV_MODEL),
        signal: extra.signal,
      });
      return {
        content: [{ type: "text", text: JSON.stringify(result) }],
        structuredContent: result,
      };
    } catch (error) {
      const safeError =
        error instanceof SafeEvaluationError
          ? error
          : new SafeEvaluationError(
              "INTERNAL_ERROR",
              500,
              "Jev evaluator failed before the request could complete.",
            );
      const result = {
        ok: false,
        error: {
          code: safeError.code,
          status: safeError.status,
          message: safeError.message,
        },
      };
      return {
        content: [{ type: "text", text: JSON.stringify(result) }],
        structuredContent: result,
        isError: true,
      };
    }
  },
);

async function readTypesafeApiKey() {
  const configPath = process.env.PASEO_JEV_CONFIG_PATH;
  if (!configPath || !path.isAbsolute(configPath)) {
    throw new SafeEvaluationError(
      "CONFIG_ERROR",
      500,
      "PASEO_JEV_CONFIG_PATH must be an absolute path.",
    );
  }

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

const transport = new StdioServerTransport();
server.connect(transport).catch(() => {
  console.error("Jev evaluator MCP failed to start.");
  process.exitCode = 1;
});
