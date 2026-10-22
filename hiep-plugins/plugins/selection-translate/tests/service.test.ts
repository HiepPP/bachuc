import assert from "node:assert/strict";
import { test } from "node:test";
import { mkdtemp } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { createService } from "../server/service";
import { Store } from "../server/store";

async function service(answers: string[]) {
  const dir = await mkdtemp(path.join(tmpdir(), "selection-translate-"));
  const calls: string[] = [];
  const svc = createService({
    store: new Store(path.join(dir, "cache.json")),
    complete: async ({ text }) => {
      calls.push(text);
      return answers.shift() ?? "";
    },
    settings: async () => ({ provider: "openrouter", model: "m" }),
  });
  return { svc, calls };
}

test("translates and caches the same text", async () => {
  const { svc, calls } = await service(["Xin chào"]);
  assert.deepEqual(await svc.translate({ text: "Hello" }), { translation: "Xin chào" });
  assert.deepEqual(await svc.translate({ text: "Hello" }), { translation: "Xin chào" });
  assert.equal(calls.length, 1);
});

test("trims before the cache and the model", async () => {
  const { svc, calls } = await service(["Xin chào"]);
  await svc.translate({ text: "  Hello \n" });
  assert.deepEqual(await svc.translate({ text: "Hello" }), { translation: "Xin chào" });
  assert.deepEqual(calls, ["Hello"]);
});

test("rejects empty and over-long text", async () => {
  const { svc } = await service([]);
  await assert.rejects(svc.translate({ text: "   " }), /empty/i);
  await assert.rejects(svc.translate({ text: "a".repeat(4001) }), /4000 characters/);
});
