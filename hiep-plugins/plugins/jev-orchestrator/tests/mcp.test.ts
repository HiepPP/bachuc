import test from "node:test";
import assert from "node:assert/strict";
import path from "node:path";
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StdioClientTransport } from "@modelcontextprotocol/sdk/client/stdio.js";
import { Engine } from "../server/engine";
import { Store } from "../server/store";
import { createBridge } from "../server/bridge";

test("stdio MCP submits, retrieves, and deduplicates a bound parent job", async () => {
  const cwd = process.cwd();
  const profile = { id: "test", name: "Test", provider: "codex", model: "fixture" };
  let launches = 0;
  const engine = new Engine(
    new Store(),
    {
      profiles: async () => [profile],
      launch: async () => `child-${++launches}`,
      wait: async () => ({ status: "idle", output: "ok" }),
      archive: async () => {},
      notify: async () => {},
    },
    async () => ({ profileId: "test", discovery: false, risk: "low", category: "lookup" }),
    1,
    async () => [],
  );
  const bridge = createBridge(engine, async () => ({ cwd }));
  const token = bridge.issue(cwd);
  bridge.bind(token, "parent", cwd);
  const transport = new StdioClientTransport({
    command: process.execPath,
    args: [
      "--import",
      path.resolve("node_modules/tsx/dist/loader.mjs"),
      path.resolve("server/mcp.ts"),
    ],
    env: { PASEO_ORCH_URL: await bridge.ready, PASEO_ORCH_TOKEN: token },
    stderr: "pipe",
  });
  const client = new Client({ name: "orchestrator-offline-test", version: "1" });
  try {
    await client.connect(transport);
    assert.deepEqual((await client.listTools()).tools.map((t) => t.name).sort(), [
      "cancel_delegation",
      "delegate_task",
      "native_delegation_status",
      "orchestrator_status",
      "prepare_native_delegate",
    ]);
    const input = {
      tasks: [
        {
          id: "sample",
          goal: "Read bounded test evidence",
          acceptance: "Return relevant source evidence",
          kind: "research",
          files: ["README.md"],
          allowedProfileIds: ["test"],
          shareWithJev: true,
        },
      ],
    };
    assert.notEqual(
      (await client.callTool({ name: "delegate_task", arguments: input })).isError,
      true,
    );
    assert.notEqual(
      (await client.callTool({ name: "delegate_task", arguments: input })).isError,
      true,
    );
    const status = await client.callTool({ name: "orchestrator_status", arguments: {} });
    assert.notEqual(status.isError, true);
    assert.equal(launches, 1);
    assert.match(JSON.stringify(status.structuredContent), /unverified/);
  } finally {
    await client.close();
    bridge.close();
    engine.stop();
  }
});
