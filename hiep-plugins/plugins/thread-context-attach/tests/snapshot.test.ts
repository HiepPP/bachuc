import assert from "node:assert/strict";
import test from "node:test";
import type { PaseoApi } from "@getpaseo/client";
import {
  buildSnapshot,
  lastAssistantText,
  MAX_REPLY_CHARS,
  NO_REPLY_TEXT,
  orderThreads,
} from "../server/snapshot";
import { getThreadSnapshot } from "../server/threads";
import type { Thread } from "../shared/threads";

const thread = { id: "agent-1", title: "Fix login", provider: "codex", cwd: "/repo/app" };
const user = (text: string) => ({ item: { type: "user_message", text } });
const assistant = (text: string) => ({ item: { type: "assistant_message", text } });

test("snapshot picks the last assistant message and starts with the header", () => {
  const entries = [
    user("first"),
    assistant("old reply"),
    user("second"),
    assistant("new reply"),
    { item: { type: "tool_call" } },
    assistant("   "),
  ];
  const reply = lastAssistantText(entries);
  assert.equal(reply, "new reply");
  const snapshot = buildSnapshot(thread, reply);
  assert.equal(snapshot.hasReply, true);
  assert.equal(snapshot.truncated, false);
  assert.equal(
    snapshot.text,
    "# Thread: Fix login\nProvider: codex\nCwd: /repo/app\n\n## Last reply\nnew reply",
  );
});

test("snapshot truncates long replies at 20,000 characters and marks the cut", () => {
  const long = "a".repeat(MAX_REPLY_CHARS) + "b".repeat(500);
  const snapshot = buildSnapshot(thread, long);
  assert.equal(snapshot.truncated, true);
  const body = snapshot.text.split("## Last reply\n")[1];
  assert.ok(body.startsWith("a".repeat(MAX_REPLY_CHARS) + "\n\n[Truncated:"));
  assert.ok(!body.includes("b"));
  assert.match(body, /first 20,000 of 20,500 characters/);
  assert.equal(buildSnapshot(thread, "a".repeat(MAX_REPLY_CHARS)).truncated, false);
});

test("a thread with no assistant message gives a clear empty message, not an error", async () => {
  assert.equal(lastAssistantText([user("hello")]), null);
  assert.equal(lastAssistantText([]), null);
  const snapshot = buildSnapshot(thread, null);
  assert.equal(snapshot.hasReply, false);
  assert.ok(snapshot.text.endsWith(`## Last reply\n${NO_REPLY_TEXT}`));

  const pages: unknown[] = [];
  const fake = {
    agents: {
      ref: () => ({
        refresh: async () => ({
          agent: {
            id: "agent-2",
            title: null,
            provider: "claude",
            cwd: "/repo",
            status: "idle",
            updatedAt: "2026-09-23T00:00:00.000Z",
            lastUserMessageAt: null,
          },
        }),
        timeline: {
          refetch: async (options: unknown) => {
            pages.push(options);
            return pages.length === 1
              ? {
                  entries: [user("only prompt")],
                  error: null,
                  hasOlder: true,
                  startCursor: { epoch: "e", seq: 1 },
                }
              : { entries: [], error: null, hasOlder: false, startCursor: null };
          },
        },
      }),
    },
  } as unknown as PaseoApi;
  const result = await getThreadSnapshot(fake, "agent-2");
  assert.equal(result.hasReply, false);
  assert.ok(result.text.startsWith("# Thread: Untitled thread\nProvider: claude\nCwd: /repo\n"));
  assert.ok(result.text.endsWith(NO_REPLY_TEXT));
  assert.equal(pages.length, 2, "pages back once through older history");
});

test("threads are newest first, current workspace first, current agent excluded", () => {
  const make = (id: string, workspaceId: string, at: string): Thread => ({
    id,
    title: id,
    provider: "codex",
    cwd: `/${workspaceId}`,
    workspaceId,
    status: "idle",
    lastActivityAt: at,
  });
  const threads = [
    make("other-new", "w2", "2026-09-23T10:00:00Z"),
    make("here-old", "w1", "2026-09-22T10:00:00Z"),
    make("current", "w1", "2026-09-23T11:00:00Z"),
    make("here-new", "w1", "2026-09-23T09:00:00Z"),
    make("other-old", "w2", "2026-09-21T10:00:00Z"),
  ];
  assert.deepEqual(
    orderThreads(threads, { currentAgentId: "current" }).map((item) => item.id),
    ["here-new", "here-old", "other-new", "other-old"],
  );
  assert.deepEqual(
    orderThreads(threads).map((item) => item.id),
    ["current", "other-new", "here-new", "here-old", "other-old"],
  );
});
