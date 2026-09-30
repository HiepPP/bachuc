import { test } from "node:test";
import assert from "node:assert/strict";
import { parseHTML } from "linkedom";
import { installBubbles, type BubbleApi } from "../client/bubble";
import type { Doc, El } from "../client/dom";

type Message = { text: string; timestamp?: number };
function page(messages: Message[]) {
  const { document, window } = parseHTML(
    `<html><head></head><body>${messages
      .map(
        () =>
          '<div data-testid="user-message"><div><div data-message-text="true"></div></div></div>',
      )
      .join("")}</body></html>`,
  );
  const texts = Array.from(
    document.querySelectorAll('[data-message-text="true"]'),
  ) as unknown as El[];
  texts.forEach((node, index) => {
    const { text, timestamp } = messages[index];
    node.textContent = text;
    if (timestamp !== undefined)
      (node as unknown as Record<string, unknown>)["__reactFiber$test"] = {
        memoizedProps: { children: text },
        return: { memoizedProps: { message: text, timestamp } },
      };
  });
  return { doc: document as unknown as Doc, window, texts };
}
const settle = () => new Promise((resolve) => setTimeout(resolve, 0));
const annotation = (node: El) =>
  node.nextElementSibling?.hasAttribute("data-prompt-translate") ? node.nextElementSibling : null;

function api(overrides: Partial<BubbleApi> = {}) {
  const calls: { text: string; cacheOnly: boolean }[] = [];
  const value: BubbleApi = {
    translate: async (text, cacheOnly) => {
      calls.push({ text, cacheOnly });
      return cacheOnly ? null : `EN ${text}`;
    },
    original: async () => null,
    ...overrides,
  };
  return { value, calls };
}
const on = { enabled: () => true, activeSince: () => 1_000 };

test("a new Vietnamese prompt gets an EN annotation; English gets nothing", async () => {
  const { doc, texts } = page([
    { text: "sửa lỗi", timestamp: 2_000 },
    { text: "fix the bug", timestamp: 2_000 },
  ]);
  const { value, calls } = api();
  const bubbles = installBubbles(value, on, doc);
  await settle();
  assert.deepEqual(calls, [{ text: "sửa lỗi", cacheOnly: false }]);
  assert.equal(annotation(texts[0])?.textContent, "ENEN sửa lỗi");
  assert.equal(annotation(texts[1]), null);
  bubbles.stop();
  assert.equal(annotation(texts[0]), null);
  assert.equal(doc.querySelectorAll("[data-prompt-translate-style]").length, 0);
});

test("old or unidentifiable prompts are cache-only and stay bare on a miss", async () => {
  const { doc, texts } = page([{ text: "cũ", timestamp: 500 }, { text: "không rõ" }]);
  const { value, calls } = api();
  const bubbles = installBubbles(value, on, doc);
  await settle();
  assert.deepEqual(calls, [
    { text: "cũ", cacheOnly: true },
    { text: "không rõ", cacheOnly: true },
  ]);
  assert.equal(annotation(texts[0]), null);
  assert.equal(annotation(texts[1]), null);
  bubbles.stop();
});

test("an enhanced prompt shows its Vietnamese original", async () => {
  const { doc, texts } = page([{ text: "Fix the save button.", timestamp: 2_000 }]);
  const { value, calls } = api({ original: async () => "sửa nút lưu" });
  const bubbles = installBubbles(value, on, doc);
  await settle();
  assert.equal(annotation(texts[0])?.textContent, "VI gốcsửa nút lưu");
  assert.equal(calls.length, 0);
  bubbles.stop();
});

test("a failed new translation offers a manual retry", async () => {
  const { doc, window, texts } = page([{ text: "lỗi mạng", timestamp: 2_000 }]);
  let fail = true;
  const bubbles = installBubbles(
    {
      translate: async (text) => {
        if (fail) throw new Error("offline");
        return `EN ${text}`;
      },
      original: async () => null,
    },
    on,
    doc,
  );
  await settle();
  assert.equal(annotation(texts[0])?.textContent, "Không dịch được · Thử lại");
  fail = false;
  annotation(texts[0])!.querySelector("button")!.dispatchEvent(new window.Event("click"));
  await settle();
  assert.equal(annotation(texts[0])?.textContent, "ENEN lỗi mạng");
  bubbles.stop();
});

test("disabled translation calls nothing, and changed text is processed again", async () => {
  const { doc, texts } = page([{ text: "một", timestamp: 2_000 }]);
  let enabled = false;
  const { value, calls } = api();
  const bubbles = installBubbles(value, { enabled: () => enabled, activeSince: () => 1_000 }, doc);
  await settle();
  assert.equal(calls.length, 0);
  enabled = true;
  bubbles.scan();
  await settle();
  texts[0].textContent = "hai lần";
  bubbles.scan();
  await settle();
  assert.deepEqual(
    calls.map((call) => call.text),
    ["một", "hai lần"],
  );
  assert.equal(annotation(texts[0])?.textContent, "ENEN hai lần");
  bubbles.stop();
});
