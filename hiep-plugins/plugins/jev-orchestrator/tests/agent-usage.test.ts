import test from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, mkdir, writeFile, rm } from "node:fs/promises";
import path from "node:path";
import { tmpdir } from "node:os";
import { parseCodexUsage, parseClaudeUsage, readAgentUsage } from "../server/agent-usage";
const counts = (input: number, output: number) => ({
  input_tokens: input,
  cached_input_tokens: input / 2,
  cache_write_input_tokens: 0,
  output_tokens: output,
  reasoning_output_tokens: output / 2,
  total_tokens: input + output,
});
const event = (type: string) => ({ type: "event_msg", payload: { type } });
test("Codex uses final thread cumulative totals and response IDs, not sum of snapshots or last call", () => {
  const record = (id: string, input: number, output: number) => ({
    type: "token_usage_record",
    payload: { response_id: id, thread_token_usage: counts(input, output) },
  });
  const rows = [
    event("task_started"),
    record("a", 100, 10),
    record("a", 100, 10),
    record("b", 250, 30),
    {
      type: "event_msg",
      payload: {
        type: "token_count",
        info: { total_token_usage: counts(250, 30), last_token_usage: counts(150, 20) },
      },
    },
    event("task_complete"),
  ];
  const u = parseCodexUsage(rows);
  assert.equal(u.complete, true);
  assert.equal(u.totalTokens, 280);
  assert.equal(u.requests, 2);
  assert.equal(u.reasoningTokens, 15);
  assert.equal(parseCodexUsage([...rows, event("task_started")]).complete, false);
  assert.equal(
    parseCodexUsage([...rows, record("c", 50, 2), event("task_complete")]).complete,
    false,
  );
});
test("Codex supports older cumulative token_count without inventing missing fields", () => {
  const u = parseCodexUsage([
    {
      type: "event_msg",
      payload: {
        type: "token_count",
        info: { total_token_usage: { input_tokens: 20, output_tokens: 4, total_tokens: 24 } },
      },
    },
    event("task_complete"),
  ]);
  assert.equal(u.totalTokens, 24);
  assert.equal(u.complete, true);
  assert.equal(u.cachedInputTokens, null);
  assert.equal(u.requests, null);
});
test("Claude deduplicates streaming chunks; repeated cached prefixes of distinct calls still count", () => {
  const message = (id: string, requestId: string | undefined, out: number) => ({
    type: "assistant",
    uuid: Math.random().toString(),
    requestId,
    message: {
      id,
      stop_reason: "end_turn",
      usage: {
        input_tokens: 10,
        cache_read_input_tokens: 100,
        cache_creation_input_tokens: 20,
        output_tokens: out,
        output_tokens_details: { thinking_tokens: 3 },
      },
    },
  });
  const u = parseClaudeUsage(
    [
      message("m1", undefined, 1),
      message("m1", "r1", 10),
      message("m1", "r1", 10),
      message("m2", "r2", 20),
    ],
    true,
  );
  assert.equal(u.complete, true);
  assert.equal(u.requests, 2);
  assert.equal(u.inputTokens, 260);
  assert.equal(u.outputTokens, 30);
  assert.equal(u.totalTokens, 290);
  assert.equal(u.cachedInputTokens, 200);
  assert.equal(u.cacheWriteTokens, 40);
  assert.equal(u.reasoningTokens, 6);
  assert.equal(parseClaudeUsage([message("m1", "r1", 10)], false).complete, false);
  assert.equal(
    parseClaudeUsage([{ type: "assistant", message: { usage: { input_tokens: 1 } } }], true)
      .totalTokens,
    null,
  );
});
test("native collector verifies session identity and supports archived Codex logs", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "jev-usage-"));
  const id = "11111111-2222-3333-4444-555555555555";
  try {
    await mkdir(path.join(root, "archived_sessions"));
    await writeFile(
      path.join(root, "archived_sessions", `rollout-date-${id}.jsonl`),
      [
        { type: "session_meta", payload: { id, cwd: root } },
        {
          type: "event_msg",
          payload: { type: "token_count", info: { total_token_usage: counts(20, 4) } },
        },
        event("task_complete"),
      ]
        .map((x) => JSON.stringify(x))
        .join("\n"),
    );
    const snapshot = {
      provider: "codex",
      cwd: root,
      status: "closed",
      runtimeInfo: { sessionId: id },
    };
    assert.equal((await readAgentUsage(snapshot, { codexHome: root })).totalTokens, 24);
    assert.equal(
      (await readAgentUsage({ ...snapshot, cwd: "/wrong" }, { codexHome: root })).complete,
      false,
    );
    assert.equal(
      (
        await readAgentUsage(
          { ...snapshot, runtimeInfo: { sessionId: "../../secrets" } },
          { codexHome: root },
        )
      ).totalTokens,
      null,
    );
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("Claude idle snapshot cannot mark usage complete before final assistant record is persisted", () => {
  const usage = {
    input_tokens: 2,
    cache_read_input_tokens: 100,
    cache_creation_input_tokens: 0,
    output_tokens: 5,
  };
  const tool = {
    type: "assistant",
    requestId: "r1",
    message: { id: "m1", usage, stop_reason: "tool_use" },
  };
  assert.equal(parseClaudeUsage([tool], true).complete, false);
  const final = {
    type: "assistant",
    requestId: "r2",
    message: { id: "m2", usage, stop_reason: "end_turn" },
  };
  assert.equal(parseClaudeUsage([tool, final], true).complete, true);
  assert.equal(parseClaudeUsage([tool, final], true).totalTokens, 214);
});
