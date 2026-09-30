import { test } from "node:test";
import assert from "node:assert/strict";
import { parseHTML } from "linkedom";
import { installQueueSnapshots, type QueueApi } from "../client/queue";
import type { Doc } from "../client/dom";
import type { SkillId } from "../shared/catalog";

const AGENT = "00000000-0000-4000-8000-000000000011";
const settle = () => new Promise((resolve) => setTimeout(resolve, 5));

function page(agentId = AGENT) {
  const { document } = parseHTML(
    '<html><head></head><body><div data-testid="message-input-root"><textarea></textarea></div></body></html>',
  );
  Object.assign(document.querySelector("textarea")!, {
    __reactFiber$test: { memoizedProps: { voiceAgentId: agentId } },
  });
  const addRow = (id: string, text: string) => {
    const edit = document.createElement("button");
    edit.setAttribute("aria-label", "Edit queued message");
    Object.assign(edit, {
      __reactFiber$test: {
        memoizedProps: {
          item: { id, text },
          onEdit() {},
          onSendNow() {},
          editLabel: "Edit queued message",
        },
      },
    });
    document.querySelector('[data-testid="message-input-root"]')!.append(edit);
    return edit;
  };
  return { document, addRow };
}

function fakeApi(skills: SkillId[] | null = ["watchtower"]) {
  const calls: string[] = [];
  const api: QueueApi = {
    skills: () => skills ?? undefined,
    prepare: async ({ text, skills }) => {
      calls.push(`prepare:${text}:${skills.join(",")}`);
      return { token: "t1" };
    },
    bindQueue: async (_agent, token, id) => void calls.push(`bind:${token}:${id}`),
    cancelQueue: async (_agent, id) => void calls.push(`cancel:${id}`),
  };
  return { api, calls };
}

test("a new queue row gets a snapshot bound to its ID", async () => {
  const { document, addRow } = page();
  const { api, calls } = fakeApi(["watchtower", "chase-goal-claude"]);
  const queue = installQueueSnapshots(api, document as unknown as Doc, undefined);
  addRow("queue-1", "queued prompt");
  queue.scan();
  await settle();
  assert.deepEqual(calls, [
    "prepare:queued prompt:watchtower,chase-goal-claude",
    "bind:t1:queue-1",
  ]);
  queue.scan();
  await settle();
  assert.equal(calls.length, 2, "a seen row is not snapshotted twice");
  queue.stop();
});

test("rows present before install are left alone", async () => {
  const { document, addRow } = page();
  addRow("old", "earlier prompt");
  const { api, calls } = fakeApi();
  const queue = installQueueSnapshots(api, document as unknown as Doc, undefined);
  queue.scan();
  await settle();
  assert.deepEqual(calls, []);
  queue.stop();
});

test("rows in new-thread composers or with unloaded skills are skipped", async () => {
  const draft = page("new-workspace");
  const first = fakeApi();
  const queueA = installQueueSnapshots(first.api, draft.document as unknown as Doc, undefined);
  draft.addRow("q", "x");
  queueA.scan();
  const loaded = page();
  const second = fakeApi(null);
  const queueB = installQueueSnapshots(second.api, loaded.document as unknown as Doc, undefined);
  loaded.addRow("q", "x");
  queueB.scan();
  await settle();
  assert.deepEqual([...first.calls, ...second.calls], []);
  queueA.stop();
  queueB.stop();
});

test("editing a queued row cancels its snapshot after binding, without blocking the click", async () => {
  const { document, addRow } = page();
  const { api, calls } = fakeApi();
  const queue = installQueueSnapshots(api, document as unknown as Doc, undefined);
  const edit = addRow("queue-1", "queued");
  queue.scan();
  let prevented = false;
  queue.onClick({
    target: edit,
    preventDefault: () => void (prevented = true),
    stopPropagation() {},
  });
  await settle();
  assert.equal(prevented, false);
  assert.deepEqual(calls, ["prepare:queued:watchtower", "bind:t1:queue-1", "cancel:queue-1"]);
  queue.stop();
});

test("no keydown listener is installed, so Enter reaches the host with no delay", () => {
  const { document } = page();
  const types: string[] = [];
  const doc = new Proxy(document, {
    get(target, key) {
      if (key === "addEventListener")
        return (type: string, ...rest: unknown[]) => {
          types.push(type);
          return (target.addEventListener as (...args: unknown[]) => void)(type, ...rest);
        };
      const value = Reflect.get(target, key);
      return typeof value === "function" ? value.bind(target) : value;
    },
  });
  const queue = installQueueSnapshots(fakeApi().api, doc as unknown as Doc, undefined);
  assert.deepEqual(types, ["click"]);
  queue.stop();
});
