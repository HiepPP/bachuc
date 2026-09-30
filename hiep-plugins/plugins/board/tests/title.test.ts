import assert from "node:assert/strict";
import test from "node:test";
import type { PaseoApi } from "@getpaseo/client";

import { createRunStore } from "../server/store";
import { firstPromptTitle, isCommandTitle, titleFromPrompt } from "../server/title";

const agent = {
  id: "demo",
  workspaceId: "workspace",
  parentAgentId: null,
  provider: "claude",
  cwd: "/demo/project",
  title: "$caveman ultra",
};

function paseoWith(texts: string[]) {
  const entries = texts.map((text) => ({ item: { type: "user_message", text } }));
  return {
    agents: { ref: () => ({ timeline: { refetch: async () => ({ entries }) } }) },
  } as unknown as Pick<PaseoApi, "agents">;
}

test("Caveman command lines are recognized as titles", () => {
  for (const title of [
    "$caveman ultra",
    "/caveman",
    "$caveman:caveman wenyan-lite",
    " $caveman  full ",
  ])
    assert.equal(isCommandTitle(title), true, title);
  for (const title of ["Fix $caveman title", "$caveman ultra fix board", "Example task"])
    assert.equal(isCommandTitle(title), false, title);
});

test("the prompt title skips Caveman lines and matches the host length", () => {
  assert.equal(titleFromPrompt("$caveman ultra\n\n  enhance   board UI\nmore"), "enhance board UI");
  assert.equal(titleFromPrompt(`$caveman lite\n\n${"x".repeat(80)}`), "x".repeat(60));
  assert.equal(titleFromPrompt("$caveman ultra\n\n"), null);
});

test("the first prompt title is used only for the prompt that produced the host title", async () => {
  const goal = "$caveman ultra\n\nVerify board titles";
  assert.equal(
    await firstPromptTitle(paseoWith([goal]), "demo", "$caveman ultra"),
    "Verify board titles",
  );
  assert.equal(await firstPromptTitle(paseoWith([goal]), "demo", "$caveman lite"), null);
  assert.equal(await firstPromptTitle(paseoWith([]), "demo", "$caveman ultra"), null);
});

test("Board shows the prompt goal for Caveman titles and keeps it across turns", () => {
  const store = createRunStore();
  store.start(agent, "first");
  assert.deepEqual(store.pendingPromptTitles(), [{ agentId: "demo", title: "$caveman ultra" }]);
  store.setPromptTitle("demo", "$caveman ultra", "Verify board titles");
  store.end(agent, "first", { kind: "completed" });
  store.start(agent, "second");
  assert.equal(store.snapshot().runs[0].title, "Verify board titles");
  assert.equal(store.displayTitle("demo"), "Verify board titles");
  assert.equal("promptTitle" in store.snapshot().runs[0], false);
  assert.deepEqual(store.pendingPromptTitles(), []);

  const restored = createRunStore();
  restored.restore(JSON.parse(JSON.stringify(store.exportState())));
  assert.equal(restored.snapshot().runs[0].title, "Verify board titles");
});

test("renamed threads and failed lookups keep the host title", () => {
  const store = createRunStore();
  store.start(agent, "first");
  store.setPromptTitle("demo", "$caveman ultra", null);
  assert.equal(store.snapshot().runs[0].title, "$caveman ultra");
  assert.deepEqual(store.pendingPromptTitles(), []);
  store.setPromptTitle("demo", "$caveman ultra", "Goal");
  store.end({ ...agent, title: "Renamed thread" }, "first", { kind: "completed" });
  assert.equal(store.snapshot().runs[0].title, "Renamed thread");
});
