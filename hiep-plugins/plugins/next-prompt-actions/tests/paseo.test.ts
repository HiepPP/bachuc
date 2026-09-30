import { test } from "node:test";
import assert from "node:assert/strict";
import type { PaseoApi } from "@getpaseo/client";
import { createDriver } from "../server/paseo";
import { Engine } from "../server/engine";
import { Store } from "../server/store";

const scope = { serverId: "host", workspaceId: "workspace", agentId: "agent" };
function fixture() {
  const entries = Array.from({ length: 1000 }, (_, i) => ({
    seqStart: i + 500,
    seqEnd: i + 500,
    timestamp: new Date(i * 1000).toISOString(),
    item: {
      type: i === 998 ? "user_message" : i === 999 ? "assistant_message" : "tool_call",
      text: i === 999 ? "## Next Steps\n```text\nprompt: Verify the result.\n```" : "test",
    },
  }));
  const page = {
    entries,
    epoch: "epoch",
    hasOlder: true,
    hasNewer: false,
    gap: false,
    error: null as string | null,
  };
  const created: unknown[] = [];
  const api = {
    agents: {
      create: async (options: unknown) => {
        created.push(options);
      },
      ref: () => ({
        refresh: async () => {},
        workspaceId: "workspace",
        cwd: "/repo",
        archivedAt: null,
        current: () => ({
          provider: "claude",
          model: "opus",
          currentModeId: "bypassPermissions",
          thinkingOptionId: null,
        }),
        status: "idle",
        timeline: { refetch: async () => page },
      }),
    },
  } as unknown as PaseoApi;
  return {
    page,
    created,
    driver: createDriver(() => api, "host"),
    engine: new Engine(
      new Store(),
      createDriver(() => api, "host"),
      async () => false,
    ),
  };
}
test("long thread offers latest-turn suggestions despite omitted older history", async () => {
  const { engine } = fixture();
  assert.equal((await engine.inspect(scope)).candidates[0]?.text, "Verify the result.");
});
test("tail without a user boundary cannot offer suggestions", async () => {
  const { page, engine } = fixture();
  page.entries[998].item.type = "tool_call";
  assert.deepEqual((await engine.inspect(scope)).candidates, []);
});
for (const flag of ["gap", "hasNewer", "error"] as const)
  test(`reject ${flag} even with a user boundary`, async () => {
    const { page, engine } = fixture();
    if (flag === "error") page.error = "failed";
    else page[flag] = true;
    await assert.rejects(engine.inspect(scope), /Timeline is incomplete/);
  });
test("new-thread start copies directory, provider, model and mode into one create call", async () => {
  const { driver, created } = fixture();
  await driver.start(scope, "Audit the logs.", "next-prompt-key");
  assert.deepEqual(created, [
    {
      cwd: "/repo",
      config: { provider: "claude/opus", modeId: "bypassPermissions" },
      prompt: "Audit the logs.",
      idempotencyKey: "next-prompt-key",
    },
  ]);
});
