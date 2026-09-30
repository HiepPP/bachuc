import { stripCavemanMode } from "../shared/caveman";
import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtemp } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { hash, Store } from "../server/store";
import { createService } from "../server/service";
import type { ChatRequest } from "../server/llm";
import { translateSettings, type TranslateSettings } from "../shared/settings";

async function setup(overrides: Partial<TranslateSettings> = {}) {
  const dir = await mkdtemp(path.join(tmpdir(), "pt-service-"));
  const store = new Store(path.join(dir, "cache.json"), undefined, 5);
  const requests: ChatRequest[] = [];
  const service = createService({
    store,
    complete: async (request) => {
      requests.push(request);
      return request.mode === "translate" ? `EN:${request.text}` : `PROMPT:${request.text}`;
    },
    settings: async () => ({ ...translateSettings.schema.parse({}), ...overrides }),
  });
  return { service, requests, store };
}

test("cacheOnly never calls the model, and a real translate fills the cache", async () => {
  const { service, requests, store } = await setup();
  assert.deepEqual(await service.translate({ text: "xin chào", cacheOnly: true }), {
    translation: null,
  });
  assert.deepEqual(await service.translate({ text: "xin chào", cacheOnly: false }), {
    translation: "EN:xin chào",
  });
  assert.deepEqual(await service.translate({ text: "xin chào", cacheOnly: true }), {
    translation: "EN:xin chào",
  });
  assert.equal(requests.length, 1);
  assert.equal(requests[0].model, "google/gemini-2.5-flash-lite");
  store.close();
});

test("English text is rejected and translation off returns null without a call", async () => {
  const { service, requests, store } = await setup();
  await assert.rejects(service.translate({ text: "hello", cacheOnly: false }), {
    message: "Text has no Vietnamese to translate",
  });
  const off = await setup({ translate: false });
  assert.deepEqual(await off.service.translate({ text: "xin chào", cacheOnly: false }), {
    translation: null,
  });
  assert.equal(requests.length + off.requests.length, 0);
  store.close();
  off.store.close();
});

test("enhance uses the enhance model and records the original", async () => {
  const { service, requests, store } = await setup();
  const { prompt } = await service.enhance({ deferCaveman: true, text: "sửa lỗi" });
  assert.equal(prompt, "PROMPT:sửa lỗi");
  assert.equal(requests[0].model, "google/gemini-2.5-flash-lite");
  assert.deepEqual(await service.original({ text: prompt }), { original: "sửa lỗi" });
  assert.deepEqual(await service.original({ text: "sửa lỗi" }), { original: null });
  const off = await setup({ enhanceShortcut: false });
  await assert.rejects(off.service.enhance({ deferCaveman: true, text: "sửa lỗi" }), {
    message: "Enhance shortcut is off",
  });
  store.close();
  off.store.close();
});

test("reply preference keeps mode commands and is skipped for English or when disabled", async () => {
  const { service, requests, store } = await setup();
  for (const mode of ["/caveman ultra", "/caveman wenyan-ultra", "tiếng Hoa giản thể"]) {
    const draft = `sửa lỗi; ${mode}`;
    const { prompt } = await service.enhance({ deferCaveman: true, text: draft });
    assert.equal(prompt, `PROMPT:${draft}`);
  }
  assert.deepEqual(await service.enhance({ deferCaveman: true, text: "fix the bug" }), {
    prompt: "PROMPT:fix the bug",
  });
  assert.equal(requests.length, 4);
  store.close();

  const disabled = await setup({ matchReplyLanguage: false });
  assert.deepEqual(await disabled.service.enhance({ deferCaveman: true, text: "sửa lỗi" }), {
    prompt: "PROMPT:sửa lỗi",
  });
  disabled.store.close();
});

test("changing reply preference reuses cached enhancement and maps both sent prompts", async () => {
  const dir = await mkdtemp(path.join(tmpdir(), "pt-reply-"));
  const store = new Store(path.join(dir, "cache.json"), undefined, 5);
  await store.run(
    hash("enhance", "openrouter", "google/gemini-2.5-flash-lite", "sửa lỗi"),
    async () => "Old rewrite without reply language",
  );
  let enabled = true;
  let calls = 0;
  const service = createService({
    store,
    complete: async () => {
      calls++;
      return "Fix the bug";
    },
    settings: async () => ({
      ...translateSettings.schema.parse({}),
      matchReplyLanguage: enabled,
    }),
  });
  const withPreference = (await service.enhance({ deferCaveman: true, text: "sửa lỗi" })).prompt;
  enabled = false;
  const withoutPreference = (await service.enhance({ deferCaveman: true, text: "sửa lỗi" })).prompt;
  assert.equal(withoutPreference, "Fix the bug");
  assert.equal(withPreference, withoutPreference);
  assert.equal(calls, 1);
  assert.deepEqual(await service.original({ text: withPreference }), { original: "sửa lỗi" });
  assert.deepEqual(await service.original({ text: withoutPreference }), { original: "sửa lỗi" });
  store.close();
});

test("selected Caveman modes never add commands to enhanced prompts", async () => {
  const cases = [
    ["lite", "/caveman lite"],
    ["full", "/caveman"],
    ["ultra", "/caveman ultra"],
    ["wenyan-lite", "/caveman wenyan-lite"],
    ["wenyan-full", "/caveman wenyan"],
    ["wenyan-ultra", "/caveman wenyan-ultra"],
  ] as const;
  for (const [cavemanMode] of cases) {
    const { service, store } = await setup({ cavemanMode, matchReplyLanguage: false });
    const prompt = (await service.enhance({ text: "Fix the bug" })).prompt;
    assert.equal(prompt, "PROMPT:Fix the bug");
    assert.equal(stripCavemanMode(prompt), "PROMPT:Fix the bug");
    store.close();
  }
});

test("mode and script preferences stay out of enhanced prompt text", async () => {
  const { service, store } = await setup({
    cavemanMode: "wenyan-ultra",
    chineseScript: "simplified",
  });
  const selected = (await service.enhance({ text: "sửa lỗi" })).prompt;
  assert.equal(selected, "PROMPT:sửa lỗi");
  assert.match(stripCavemanMode(selected), /^PROMPT:sửa lỗi/);
  assert.doesNotMatch(selected, /prompt-translate:response-mode/);
  assert.deepEqual(await service.original({ text: selected }), { original: "sửa lỗi" });

  const explicit = (await service.enhance({ text: "sửa lỗi; /caveman ultra" })).prompt;
  assert.match(explicit, /^PROMPT:sửa lỗi; \/caveman ultra/);
  assert.doesNotMatch(explicit, /\/caveman wenyan-ultra|Simplified Chinese characters/);
  store.close();
});

test("deferred enhancement has no mode and decorated prompt still resolves the original", async () => {
  const { service, store } = await setup({
    cavemanMode: "wenyan-ultra",
    matchReplyLanguage: false,
  });
  try {
    const { prompt } = await service.enhance({ text: "sửa lỗi", deferCaveman: true });
    assert.equal(prompt, "PROMPT:sửa lỗi");
    for (const cavemanMode of ["lite", "follow-agent"] as const) {
      const sent: string = `/caveman ${cavemanMode}\n\n[prompt-translate:response-mode]\nlegacy\n[/prompt-translate:response-mode]\n\n${prompt}`;
      assert.deepEqual(await service.original({ text: sent }), { original: "sửa lỗi" });
      assert.deepEqual(await service.original({ text: sent.replace(/^\/caveman/, "$caveman") }), {
        original: "sửa lỗi",
      });
    }
  } finally {
    store.close();
  }
});

test("deferred enhancement restores an omitted manual command before recording original", async () => {
  const { store } = await setup();
  const service = createService({
    store,
    settings: async () => translateSettings.schema.parse({}),
    complete: async () => "Explain git status.",
  });
  try {
    const draft = "/caveman lite\n\nGiải thích git status.";
    const { prompt } = await service.enhance({ text: draft, deferCaveman: true });
    assert.match(prompt, /^\/caveman lite\n\nExplain git status/);
    assert.deepEqual(await service.original({ text: prompt }), { original: draft });
  } finally {
    store.close();
  }
});
