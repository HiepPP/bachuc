import assert from "node:assert/strict";
import test from "node:test";
import { setTimeout as delay } from "node:timers/promises";
import { Engine } from "../server/engine";
import { Store } from "../server/store";
import type { Driver, Judge, Profile } from "../server/types";
import { missingUsage, type TokenUsage } from "../server/usage";
import { taskSchema } from "../shared/contracts";

const cwd = process.cwd();
const profile: Profile = {
  id: "worker",
  name: "Worker",
  provider: "codex",
  model: "test-model",
};
const task = taskSchema.parse({
  id: "usage",
  goal: "Verify evaluation accounting.",
  acceptance: "Usage is recorded once.",
  kind: "implementation",
  files: ["server/engine.ts"],
  allowedProfileIds: [profile.id],
  shareWithJev: true,
  discovery: "never",
  review: "never",
  checks: [{ argv: ["node", "-e", "process.exit(0)"] }],
});
const usage = (source: string, inputTokens: number, outputTokens: number): TokenUsage => ({
  source,
  complete: true,
  inputTokens,
  cachedInputTokens: null,
  cacheWriteTokens: null,
  outputTokens,
  reasoningTokens: null,
  totalTokens: inputTokens + outputTokens,
  requests: 1,
  notes: [],
});

function harness(judge: Judge, childUsage?: TokenUsage) {
  const store = new Store();
  const driver: Driver = {
    profiles: async () => [profile],
    launch: async () => "child",
    wait: async () => ({ status: "idle", output: "done", usage: childUsage }),
    archive: async () => {},
    notify: async () => {},
  };
  return new Engine(store, driver, judge, 1, async () => [
    { argv: ["check"], exitCode: 0, output: "ok", durationMs: 1, timedOut: false },
  ]);
}

async function completed(engine: Engine) {
  await engine.submit("parent", cwd, { tasks: [task] });
  for (let count = 0; count < 200; count++) {
    const job = engine.list("parent")[0];
    if (job?.notificationCompleteAt) return job;
    await delay(5);
  }
  throw new Error("Job notification did not complete.");
}

test("records each evaluation once and uses complete child usage for metrics", async () => {
  let inputState: Record<string, unknown> | undefined;
  const evaluatorUsage = usage("evaluation", 4, 2);
  const childUsage = usage("child", 7, 3);
  const engine = harness(async (_phase, state) => {
    inputState = state;
    return {
      profileId: profile.id,
      discovery: false,
      risk: "low",
      category: "implementation",
      usage: evaluatorUsage,
    };
  }, childUsage);

  const job = await completed(engine);
  assert.equal(job.evaluations?.length, 1);
  assert.deepEqual(job.evaluations?.[0].usage, evaluatorUsage);
  assert.equal(job.attempts[0].usage, childUsage);
  assert.equal(engine.store.metrics[0].tokens, 10);
  assert.equal("evaluations" in (inputState ?? {}), false);
});

test("retains billed usage and error when judgment validation fails", async () => {
  const billed = usage("evaluation", 8, 3);
  const engine = harness(async () => {
    throw Object.assign(new Error("Invalid Jev judgment."), { usage: billed });
  });

  const job = await completed(engine);
  assert.equal(job.status, "needs_input");
  assert.equal(job.evaluations?.length, 1);
  assert.equal(job.evaluations?.[0].error, "Invalid Jev judgment.");
  assert.deepEqual(job.evaluations?.[0].usage, billed);
});

test("keeps failed calls without usage unknown instead of zero", async () => {
  const engine = harness(async () => {
    throw new Error("Evaluation interrupted.");
  });

  const job = await completed(engine);
  const recorded = job.evaluations?.[0];
  assert.equal(recorded?.usage.complete, false);
  assert.equal(recorded?.usage.inputTokens, null);
  assert.equal(recorded?.usage.outputTokens, null);
  assert.equal(recorded?.usage.totalTokens, null);
  assert.deepEqual(
    recorded?.usage,
    missingUsage("jev-evaluation", "Evaluation failed without reported token usage."),
  );
});
