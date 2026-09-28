import assert from "node:assert/strict";
import test from "node:test";
import { mkdtemp, readFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StdioClientTransport } from "@modelcontextprotocol/sdk/client/stdio.js";
import { createRunStore, runStateSchema } from "../server/store";
import { createBridge, withBoardMcp, type RemoveResult } from "../server/bridge";

const agent = {
  id: "parent",
  workspaceId: "workspace",
  parentAgentId: null,
  provider: "claude",
  cwd: "/demo/project",
  title: "Parent",
};
const child = { ...agent, id: "child", parentAgentId: "parent", title: "Child" };
const grandchild = { ...agent, id: "grandchild", parentAgentId: "child", title: "Grandchild" };
const visible = (store: ReturnType<typeof createRunStore>) =>
  store
    .snapshot()
    .runs.map((run) => run.agentId)
    .sort();

test("board_remove removes finished threads and descendants, never other threads", () => {
  const store = createRunStore();
  store.end(agent, "p1", { kind: "completed" });
  store.end(child, "c1", { kind: "completed" });
  store.end(grandchild, "g1", { kind: "completed" });
  store.end({ ...agent, id: "other" }, "o1", { kind: "completed" });

  assert.deepEqual(store.removeByAgent("child", "parent"), { result: "forbidden" });
  assert.deepEqual(store.removeByAgent("other", "grandchild"), { result: "forbidden" });
  assert.deepEqual(store.removeByAgent("parent", "missing"), { result: "not_found" });
  assert.deepEqual(store.removeByAgent("parent", "child"), { result: "removed", count: 2 });
  assert.deepEqual(visible(store), ["other", "parent"]);
  assert.deepEqual(store.removeByAgent("parent", "child"), { result: "not_found" });
  assert.deepEqual(store.removeByAgent("parent", "parent"), { result: "removed", count: 1 });
  assert.deepEqual(visible(store), ["other"]);
});

test("a running thread is removed only when its turn completes", () => {
  const store = createRunStore();
  store.start(agent, "t1");
  assert.deepEqual(store.removeByAgent("parent", "parent"), { result: "scheduled" });
  assert.deepEqual(visible(store), ["parent"]);
  assert.equal(store.snapshot().runs[0].hasOwnProperty("removeOnFinish"), false);
  // The flag survives a save and restore.
  const restored = createRunStore();
  restored.restore(runStateSchema.parse(JSON.parse(JSON.stringify(store.exportState()))));
  restored.end(agent, "t1", { kind: "completed" });
  assert.deepEqual(visible(restored), []);

  // Failed turns stay visible and drop the request.
  store.end(agent, "t1", { kind: "failed", error: { message: "boom" } });
  assert.deepEqual(visible(store), ["parent"]);
  store.start(agent, "t2");
  store.end(agent, "t2", { kind: "completed" });
  assert.deepEqual(visible(store), ["parent"]);

  // A new turn clears a pending request, and a removed thread returns on its next turn.
  store.start(agent, "t3");
  store.removeByAgent("parent", "parent");
  store.start(agent, "t4");
  store.end(agent, "t4", { kind: "completed" });
  assert.deepEqual(visible(store), ["parent"]);
  store.removeByAgent("parent", "parent");
  store.start(agent, "t5");
  assert.deepEqual(visible(store), ["parent"]);
});

test("bridge accepts only bound tokens without an Origin and passes the caller", async (t) => {
  const file = path.join(await mkdtemp(path.join(tmpdir(), "board-bridge-")), "bridge.json");
  const calls: string[][] = [];
  const bridge = createBridge(async (callerId, agentId) => {
    calls.push([callerId, agentId]);
    return { result: "scheduled" };
  }, file);
  t.after(() => bridge.close());
  const url = await bridge.ready;
  const token = bridge.issue();
  const post = (headers: Record<string, string>, body: unknown = { action: "remove" }) =>
    fetch(url, { method: "POST", headers, body: JSON.stringify(body) });
  assert.equal((await post({ Authorization: `Bearer ${token}` })).status, 403);
  assert.equal(bridge.bind(token, "parent"), true);
  assert.equal(bridge.bind(token, "other"), false);
  assert.equal((await post({})).status, 403);
  assert.equal(
    (await post({ Authorization: `Bearer ${token}`, Origin: "https://evil.test" })).status,
    403,
  );
  assert.equal(
    (await post({ Authorization: `Bearer ${token}` }, { action: "archive" })).status,
    400,
  );
  const own = await post({ Authorization: `Bearer ${token}` });
  assert.deepEqual(await own.json(), { result: "scheduled" });
  await post(
    { Authorization: `Bearer ${token}` },
    { action: "remove", input: { agentId: "child" } },
  );
  assert.deepEqual(calls, [
    ["parent", "parent"],
    ["parent", "child"],
  ]);
  assert.deepEqual(JSON.parse(await readFile(file, "utf8")).leases, [
    [token, { agentId: "parent" }],
  ]);
  bridge.revoke("parent");
  assert.equal((await post({ Authorization: `Bearer ${token}` })).status, 403);
});

test("create hook injects one board MCP server for Claude and Codex only", () => {
  let issued = 0;
  const issue = () => `token-${++issued}`;
  const request: {
    env?: Record<string, string>;
    config: { provider: string; mcpServers?: Record<string, unknown> };
  } = { env: { KEEP: "1" }, config: { provider: "claude", mcpServers: { other: {} } } };
  const injected = withBoardMcp(request, "/plugins/board", "http://127.0.0.1:1/mcp", issue);
  const board = injected.config.mcpServers!.board as {
    env: Record<string, string>;
    args: string[];
  };
  assert.equal(injected.env?.KEEP, "1");
  assert.equal(injected.env?.PASEO_BOARD_TOKEN, "token-1");
  assert.equal(board.env.PASEO_BOARD_TOKEN, "token-1");
  assert.equal(board.env.PASEO_BOARD_URL, "http://127.0.0.1:1/mcp");
  assert.equal(board.args.at(-1), "/plugins/board/server/mcp.ts");
  assert.ok("other" in injected.config.mcpServers!);
  assert.equal(withBoardMcp(injected, "/plugins/board", "u", issue), injected);
  const other = { config: { provider: "opencode" } };
  assert.equal(withBoardMcp(other, "/plugins/board", "u", issue), other);
  assert.equal(issued, 1);
});

test("stdio MCP exposes board_remove and reports refused removals as errors", async () => {
  const results: RemoveResult[] = [{ result: "removed", count: 1 }, { result: "forbidden" }];
  const bridge = createBridge(async () => results.shift()!);
  const token = bridge.issue();
  bridge.bind(token, "parent");
  const transport = new StdioClientTransport({
    command: process.execPath,
    args: [
      "--import",
      path.resolve("node_modules/tsx/dist/loader.mjs"),
      path.resolve("server/mcp.ts"),
    ],
    env: { PASEO_BOARD_URL: await bridge.ready, PASEO_BOARD_TOKEN: token },
    stderr: "pipe",
  });
  const client = new Client({ name: "board-offline-test", version: "1" });
  try {
    await client.connect(transport);
    assert.deepEqual(
      (await client.listTools()).tools.map((tool) => tool.name),
      ["board_remove"],
    );
    const removed = await client.callTool({ name: "board_remove", arguments: {} });
    assert.notEqual(removed.isError, true);
    assert.deepEqual(removed.structuredContent, { result: "removed", count: 1 });
    const refused = await client.callTool({
      name: "board_remove",
      arguments: { agentId: "other" },
    });
    assert.equal(refused.isError, true);
  } finally {
    await client.close();
    bridge.close();
  }
});
