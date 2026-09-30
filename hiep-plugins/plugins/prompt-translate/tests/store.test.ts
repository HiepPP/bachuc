import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { Store } from "../server/store";

async function file() {
  return path.join(await mkdtemp(path.join(tmpdir(), "pt-store-")), "nested", "cache.json");
}

test("concurrent runs for one key share a single task", async () => {
  const store = new Store(await file(), undefined, 5);
  let calls = 0;
  const task = async () => {
    calls++;
    await new Promise((resolve) => setTimeout(resolve, 10));
    return "v";
  };
  assert.deepEqual(await Promise.all([store.run("k", task), store.run("k", task)]), ["v", "v"]);
  assert.equal(await store.run("k", task), "v");
  assert.equal(calls, 1);
  store.close();
});

test("a failed task is not cached and can run again", async () => {
  const store = new Store(await file(), undefined, 5);
  await assert.rejects(
    store.run("k", async () => Promise.reject(new Error("boom"))),
    { message: "boom" },
  );
  assert.equal(await store.run("k", async () => "ok"), "ok");
  store.close();
});

test("least recently used entries are evicted", async () => {
  const store = new Store(await file(), { cache: 2, pairs: 2 }, 5);
  await store.run("a", async () => "1");
  await store.run("b", async () => "2");
  await store.get("a");
  await store.run("c", async () => "3");
  assert.equal(await store.get("a"), "1");
  assert.equal(await store.get("b"), undefined);
  store.close();
});

test("values and pairs persist, and a corrupt file starts empty", async () => {
  const target = await file();
  const first = new Store(target, undefined, 5);
  await first.run("k", async () => "v");
  await first.pair("Enhanced prompt", "Bản gốc");
  await first.flush();
  const second = new Store(target, undefined, 5);
  assert.equal(await second.get("k"), "v");
  assert.equal(await second.original("Enhanced prompt"), "Bản gốc");
  assert.equal(await second.original("Other"), null);
  await writeFile(target, "{not json");
  const third = new Store(target, undefined, 5);
  assert.equal(await third.get("k"), undefined);
  first.close();
  second.close();
  third.close();
});
