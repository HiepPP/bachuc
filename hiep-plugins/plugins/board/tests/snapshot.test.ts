import assert from "node:assert/strict";
import test from "node:test";
import type { PaseoApi } from "@getpaseo/client";
import { listBoardAgents } from "../server/snapshot";

test("fetches every page and detects repeated cursors", async () => {
  let calls = 0;
  const client = {
    agents: {
      list: async () => ({
        entries: [],
        pageInfo: { hasMore: ++calls < 3, nextCursor: String(calls) },
      }),
    },
  } as unknown as PaseoApi;
  assert.deepEqual(await listBoardAgents(client, new AbortController().signal), {
    agents: [],
    visibleAgentIds: new Set(),
    unreadAgentIds: new Set(),
  });
  assert.equal(calls, 3);
  const repeated = {
    agents: {
      list: async () => ({ entries: [], pageInfo: { hasMore: true, nextCursor: "same" } }),
    },
  } as unknown as PaseoApi;
  await assert.rejects(listBoardAgents(repeated, new AbortController().signal), /Refresh to retry/);
});
test("stopped plugin does not start an SDK request", async () => {
  const controller = new AbortController();
  controller.abort();
  await assert.rejects(listBoardAgents({} as PaseoApi, controller.signal), /Board stopped/);
});

test("snapshot retains the host project key, including shared projects across workspaces", async () => {
  const client = {
    agents: {
      list: async () => ({
        entries: ["one", "two"].map((id) => ({
          agent: {
            id,
            status: "running",
            cwd: `/work/${id}`,
            workspaceId: id,
            labels: id === "two" ? { "paseo.parent-agent-id": "one" } : {},
          },
          project: { projectName: "Atlas", projectKey: "atlas" },
        })),
        pageInfo: { hasMore: false },
      }),
    },
  } as unknown as PaseoApi;
  const { agents: rows } = await listBoardAgents(client, new AbortController().signal);
  assert.deepEqual(
    rows.map((row) => row.parentAgentId),
    [null, "one"],
  );
  assert.deepEqual(
    rows.map((row) => row.projectKey),
    ["atlas", "atlas"],
  );
});

test("collects unarchived membership across pages without treating idle agents as running", async () => {
  let calls = 0;
  const client = {
    agents: {
      list: async (options: { filter: unknown; page: { cursor?: string } }) => {
        assert.deepEqual(options.filter, { includeArchived: false });
        assert.equal(options.page.cursor, calls === 0 ? undefined : "next");
        const status = calls++ === 0 ? "running" : "idle";
        return {
          entries: [{ agent: { id: status, status, labels: {} }, project: null }],
          pageInfo: { hasMore: calls === 1, nextCursor: "next" },
        };
      },
    },
  } as unknown as PaseoApi;
  const result = await listBoardAgents(client, new AbortController().signal);
  assert.deepEqual([...result.visibleAgentIds], ["running", "idle"]);
  assert.deepEqual(
    result.agents.map((agent) => agent.id),
    ["running"],
  );
});

test("a failed directory page rejects instead of returning incomplete membership", async () => {
  let calls = 0;
  const client = {
    agents: {
      list: async () => {
        if (++calls === 2) throw new Error("offline");
        return { entries: [], pageInfo: { hasMore: true, nextCursor: "next" } };
      },
    },
  } as unknown as PaseoApi;
  await assert.rejects(listBoardAgents(client, new AbortController().signal), /offline/);
});

test("collects unread threads from finished and failed attention only", async () => {
  const attention = {
    finished: { requiresAttention: true, attentionReason: "finished" },
    error: { requiresAttention: true, attentionReason: "error" },
    permission: { requiresAttention: true, attentionReason: "permission" },
    read: { requiresAttention: false, attentionReason: null },
  };
  const client = {
    agents: {
      list: async () => ({
        entries: Object.entries(attention).map(([id, flags]) => ({
          agent: { id, status: "idle", labels: {}, ...flags },
          project: null,
        })),
        pageInfo: { hasMore: false },
      }),
    },
  } as unknown as PaseoApi;
  const { unreadAgentIds } = await listBoardAgents(client, new AbortController().signal);
  assert.deepEqual([...unreadAgentIds], ["finished", "error"]);
});
