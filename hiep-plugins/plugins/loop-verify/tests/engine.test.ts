import assert from "node:assert/strict";
import { test } from "node:test";
import {
  Engine,
  ROOT_LABEL,
  VERIFY_LABEL,
  type CreateRequest,
  type LoopAgent,
  type VerifyResult,
  endsWithQuestion,
} from "../server/engine";
import { Store } from "../server/store";
import type { LoopStatus } from "../shared/status";

function setup(results: VerifyResult[], labels: Record<string, string>) {
  const created: CreateRequest[] = [];
  const published: { target: string; rowId: string; status: LoopStatus }[] = [];
  const agents = new Map<string, LoopAgent>([
    [
      "root",
      {
        id: "root",
        cwd: "/repo",
        provider: "claude",
        model: "haiku",
        modeId: "bypassPermissions",
        thinkingOptionId: null,
        labels,
      },
    ],
  ]);
  const store = new Store();
  let next = 0;
  const engine = new Engine(
    store,
    {
      inspect: async (id) => agents.get(id) ?? null,
      create: async (request) => {
        created.push(request);
      },
      publish: async (target, rowId, status) => {
        published.push({ target, rowId, status });
      },
    },
    async () => results.shift() ?? { exitCode: 0, output: "" },
    () => {},
    () => `child-${++next}`,
  );
  return { engine, store, created, published };
}

const fail = { exitCode: 1, output: "1 test failed" };

test("ignores agents without the verify label", async () => {
  const { engine, created } = setup([fail], {});
  await engine.ended({
    agentId: "root",
    outcome: "completed",
    firstUserMessage: "goal",
    lastAssistantMessage: null,
  });
  assert.equal(created.length, 0);
});

test("stops when verify passes", async () => {
  const { engine, store, created } = setup([{ exitCode: 0, output: "" }], {
    [VERIFY_LABEL]: "npm test",
  });
  await engine.ended({
    agentId: "root",
    outcome: "completed",
    firstUserMessage: "goal",
    lastAssistantMessage: null,
  });
  assert.equal(created.length, 0);
  assert.equal(store.byRoot("root")?.status, "passed");
});

test("starts a fresh child agent with the goal and verify output", async () => {
  const { engine, created } = setup([fail], { [VERIFY_LABEL]: "npm test" });
  await engine.ended({
    agentId: "root",
    outcome: "completed",
    firstUserMessage: "Fix login",
    lastAssistantMessage: null,
  });
  assert.equal(created.length, 1);
  const [request] = created;
  assert.equal(request.agentId, "child-1");
  assert.equal(request.parentId, "root");
  assert.deepEqual(request.config, { provider: "claude/haiku", modeId: "bypassPermissions" });
  assert.deepEqual(request.labels, { [ROOT_LABEL]: "root" });
  assert.match(request.prompt, /^Fix login/);
  assert.match(request.prompt, /1 test failed/);
});

test("gives up after the max rounds", async () => {
  const { engine, store, created } = setup([fail, fail, fail], {
    [VERIFY_LABEL]: "npm test",
    "loop-verify-max": "3",
  });
  await engine.ended({
    agentId: "root",
    outcome: "completed",
    firstUserMessage: "goal",
    lastAssistantMessage: null,
  });
  await engine.ended({
    agentId: "child-1",
    outcome: "completed",
    firstUserMessage: null,
    lastAssistantMessage: null,
  });
  await engine.ended({
    agentId: "child-2",
    outcome: "completed",
    firstUserMessage: null,
    lastAssistantMessage: null,
  });
  assert.equal(created.length, 2);
  assert.equal(store.byRoot("root")?.status, "exhausted");
  assert.equal(store.byRoot("root")?.history.length, 3);
});

test("a canceled turn stops the loop", async () => {
  const { engine, store, created } = setup([fail], { [VERIFY_LABEL]: "npm test" });
  await engine.ended({
    agentId: "root",
    outcome: "completed",
    firstUserMessage: "goal",
    lastAssistantMessage: null,
  });
  await engine.ended({
    agentId: "child-1",
    outcome: "canceled",
    firstUserMessage: null,
    lastAssistantMessage: null,
  });
  assert.equal(created.length, 1);
  assert.equal(store.byRoot("root")?.status, "stopped");
});

test("a finished loop is not re-armed by follow-up turns", async () => {
  const { engine, created } = setup([{ exitCode: 0, output: "" }, fail], {
    [VERIFY_LABEL]: "npm test",
  });
  await engine.ended({
    agentId: "root",
    outcome: "completed",
    firstUserMessage: "goal",
    lastAssistantMessage: null,
  });
  await engine.ended({
    agentId: "root",
    outcome: "completed",
    firstUserMessage: "more",
    lastAssistantMessage: null,
  });
  assert.equal(created.length, 0);
});

const question = "Should I create CHANGELOG.md, or only edit sum.mjs?";

test("detects a trailing question", () => {
  assert.equal(endsWithQuestion(question), true);
  assert.equal(endsWithQuestion("- Or keep it as-is?**\n"), true);
  assert.equal(endsWithQuestion("Fixed. Why? Because a - b was wrong."), false);
  assert.equal(endsWithQuestion(null), false);
});

test("pauses instead of starting a round when the agent asks a question", async () => {
  const { engine, store, created } = setup([fail], { [VERIFY_LABEL]: "npm test" });
  await engine.ended({
    agentId: "root",
    outcome: "completed",
    firstUserMessage: "goal",
    lastAssistantMessage: question,
  });
  assert.equal(created.length, 0);
  assert.equal(store.byRoot("root")?.status, "paused");
});

test("a reply to the paused agent resumes the same round", async () => {
  const { engine, store, created } = setup([fail, fail], { [VERIFY_LABEL]: "npm test" });
  const turn = { agentId: "root", outcome: "completed", firstUserMessage: "goal" } as const;
  await engine.ended({ ...turn, lastAssistantMessage: question });
  await engine.ended({ ...turn, firstUserMessage: "yes", lastAssistantMessage: "Done." });
  assert.equal(created.length, 1);
  assert.equal(created[0].title, "loop-verify round 2/5");
  assert.equal(store.byRoot("root")?.status, "running");
});

test("verify passing wins over a trailing question", async () => {
  const { engine, store } = setup([{ exitCode: 0, output: "" }], { [VERIFY_LABEL]: "npm test" });
  await engine.ended({
    agentId: "root",
    outcome: "completed",
    firstUserMessage: "goal",
    lastAssistantMessage: "All green. Anything else?",
  });
  assert.equal(store.byRoot("root")?.status, "passed");
});

test("archiving a paused agent stops the loop", async () => {
  const { engine, store } = setup([fail], { [VERIFY_LABEL]: "npm test" });
  await engine.ended({
    agentId: "root",
    outcome: "completed",
    firstUserMessage: "goal",
    lastAssistantMessage: question,
  });
  engine.stop("root");
  assert.equal(store.byRoot("root")?.status, "stopped");
});

test("publishes a status row to the finished agent and the root", async () => {
  const { engine, published } = setup([fail, { exitCode: 0, output: "" }], {
    [VERIFY_LABEL]: "npm test",
  });
  await engine.ended({
    agentId: "root",
    outcome: "completed",
    firstUserMessage: "goal",
    lastAssistantMessage: null,
  });
  assert.deepEqual(
    published.map(({ target, status }) => [
      target,
      status.status,
      status.round,
      status.nextAgentId,
    ]),
    [["root", "running", 1, "child-1"]],
  );
  assert.equal(published[0].status.output, "1 test failed");
  await engine.ended({
    agentId: "child-1",
    outcome: "completed",
    firstUserMessage: null,
    lastAssistantMessage: null,
  });
  assert.deepEqual(
    published.slice(1).map(({ target, status }) => [target, status.status, status.round]),
    [
      ["child-1", "passed", 2],
      ["root", "passed", 2],
    ],
  );
});

test("publishes the paused state", async () => {
  const { engine, published } = setup([fail], { [VERIFY_LABEL]: "npm test" });
  await engine.ended({
    agentId: "root",
    outcome: "completed",
    firstUserMessage: "goal",
    lastAssistantMessage: question,
  });
  assert.equal(published.at(-1)?.status.status, "paused");
  assert.equal(published.at(-1)?.status.exitCode, 1);
});

test("every verify result gets a new row on the root", async () => {
  const { engine, published } = setup([fail, { exitCode: 0, output: "" }], {
    [VERIFY_LABEL]: "npm test",
  });
  const turn = {
    outcome: "completed",
    firstUserMessage: null,
    lastAssistantMessage: null,
  } as const;
  await engine.ended({ ...turn, agentId: "root", firstUserMessage: "goal" });
  await engine.ended({ ...turn, agentId: "child-1" });
  const rootRows = published.filter(({ target }) => target === "root").map(({ rowId }) => rowId);
  assert.deepEqual(rootRows, ["loop-verify-1", "loop-verify-2"]);
});
