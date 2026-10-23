// Run with plain node: the AI SDK fails inside the daemon's plugin subprocess.
import { appendFileSync, existsSync, mkdirSync, readFileSync } from "node:fs";
import { homedir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = fileURLToPath(new URL("../../../../../", import.meta.url));
const dataDir = path.join(repoRoot, ".dev", "decider-probe");
const questionsPath = fileURLToPath(new URL("../questions.json", import.meta.url));
const resultsPath = path.join(dataDir, "results-jev.jsonl");
const configFile = process.env.PROBE_JEV_CONFIG ?? path.join(homedir(), ".bachuc", "config.json");
const limit = process.env.PROBE_LIMIT ? Number(process.env.PROBE_LIMIT) : Infinity;

// The evaluator plugin owns the SDK, the credential lookup, and the request.
const EVALUATOR = new URL(
  "../../../../../hiep-plugins/plugins/jev-evaluator/server/typesafe.mjs",
  import.meta.url,
).href;

interface ProbeEvent {
  id: string;
  state: unknown;
}

function readJsonl<T>(file: string): T[] {
  if (!existsSync(file)) return [];
  return readFileSync(file, "utf8")
    .split("\n")
    .filter((line) => line.length > 0)
    .map((line) => JSON.parse(line) as T);
}

const spec = JSON.parse(readFileSync(questionsPath, "utf8")) as {
  version: string;
  questions: Record<string, { type: string; [key: string]: unknown }>;
};
const questions = Object.fromEntries(
  Object.entries(spec.questions).map(([id, question]) => [
    id,
    question.type === "noul" ? { ...question, type: "boolean" } : question,
  ]),
);

async function main() {
  mkdirSync(dataDir, { recursive: true });
  const events = readJsonl<ProbeEvent>(path.join(dataDir, "events.jsonl")).slice(0, limit);
  const done = new Set(readJsonl<{ id: string }>(resultsPath).map((r) => r.id));

  for (const event of events) {
    if (done.has(event.id)) continue;
    const started = performance.now();
    let answers: Record<string, unknown> = {};
    let inputTokens: number | null = null;
    let error: string | null = null;
    try {
      const { evaluateWithTypesafe } = await import(EVALUATOR);
      const result = await evaluateWithTypesafe(configFile, { state: event.state, questions });
      const readOnly = result.answers.readOnly as { probability?: unknown };
      const action = result.answers.action as {
        choice?: unknown;
        probabilities?: Record<string, number>;
      };
      if (typeof readOnly?.probability !== "number") throw new Error("Invalid readOnly answer.");
      if (action?.choice !== "allow" && action?.choice !== "deny" && action?.choice !== "escalate")
        throw new Error("Invalid action answer.");
      const { allow, deny, escalate } = action.probabilities ?? {};
      if (typeof allow !== "number" || typeof deny !== "number" || typeof escalate !== "number")
        throw new Error("Missing action probabilities.");
      answers = {
        readOnly: { p_true: readOnly.probability },
        action: { choice: action.choice, probabilities: { allow, deny, escalate } },
      };
      inputTokens = typeof result.usage?.inputTokens === "number" ? result.usage.inputTokens : null;
    } catch (caught) {
      answers = {};
      error = caught instanceof Error ? caught.message : String(caught);
    }
    const line = {
      backend: "jev",
      id: event.id,
      question_id: "both",
      question_version: spec.version,
      input_tokens: inputTokens,
      client_ms: performance.now() - started,
      server_ms: null,
      answers,
      error,
    };
    appendFileSync(resultsPath, `${JSON.stringify(line)}\n`);
    console.log(`${event.id} ${error ? `error: ${error}` : "ok"}`);
  }
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
