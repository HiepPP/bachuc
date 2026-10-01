// Runs in a standalone Node process: the AI SDK fails inside the bundled plugin subprocess.
import { POLICY, QUESTIONS, type GateState, type Verdict } from "./policy";

// The evaluator plugin owns the SDK, the credential lookup, and the request; see its README.
const EVALUATOR = new URL("../../jev-evaluator/server/typesafe.mjs", import.meta.url).href;

async function run(state: GateState, configFile: string): Promise<Verdict> {
  const { evaluateWithTypesafe } = await import(EVALUATOR);
  const result = await evaluateWithTypesafe(configFile, {
    state: {
      policy: POLICY,
      command: state.command,
      tool: state.tool,
      cwd_relative: state.cwdRelative,
    },
    questions: QUESTIONS,
  });
  const readOnly = result.answers.readOnly as { probability?: unknown };
  const action = result.answers.action as {
    choice?: unknown;
    probabilities?: Record<string, number>;
  };
  const choice = action.choice;
  if (typeof readOnly.probability !== "number") throw new Error("Invalid readOnly answer.");
  if (choice !== "allow" && choice !== "deny" && choice !== "escalate")
    throw new Error("Invalid action answer.");
  const probability = action.probabilities?.[choice];
  if (typeof probability !== "number") throw new Error("Missing action probability.");
  // Boolean answers carry only P(true); derive the value and the confidence of that value.
  const value = readOnly.probability >= 0.5;
  return {
    readOnly: { value, probability: value ? readOnly.probability : 1 - readOnly.probability },
    action: { choice, probability },
  };
}

let input = "";
for await (const chunk of process.stdin) {
  input += chunk;
  if (Buffer.byteLength(input) > 64000) throw new Error("Input too large");
}
try {
  const { state, configFile } = JSON.parse(input);
  process.stdout.write(JSON.stringify(await run(state, configFile)));
} catch {
  process.stdout.write(JSON.stringify({ error: "Jev Typesafe evaluation failed" }));
  process.exitCode = 1;
}
