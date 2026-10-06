import assert from "node:assert/strict";
import { mkdtemp, readdir } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";
import { createHtmlStore, htmlProblem, MAX_HTML_BYTES } from "../server/store";

async function store() {
  const dir = await mkdtemp(path.join(tmpdir(), "html-reply-"));
  return { dir, store: createHtmlStore(dir) };
}

test("saves a page and reads it back by id", async () => {
  const { store: pages } = await store();
  await pages.save("page-1", "<h1>Chart</h1>");

  assert.equal(await pages.read("page-1"), "<h1>Chart</h1>");
});

test("returns null for an unknown id, so the row can show page not found", async () => {
  const { store: pages } = await store();

  assert.equal(await pages.read("missing"), null);
});

test("never reaches the file system with a malformed id", async () => {
  const { dir, store: pages } = await store();

  assert.equal(await pages.read("../secret"), null);
  await assert.rejects(pages.save("../escape", "<p>x</p>"), /Invalid page id/);
  assert.deepEqual(await readdir(dir), []);
});

test("accepts a page of exactly 1 MiB and rejects one byte more", async () => {
  const { store: pages } = await store();
  const atLimit = "a".repeat(MAX_HTML_BYTES);

  assert.equal(htmlProblem(atLimit), null);
  await pages.save("big", atLimit);
  assert.match(htmlProblem(`${atLimit}a`) ?? "", /limit is 1048576 bytes \(1 MiB\)/);
  await assert.rejects(pages.save("too-big", `${atLimit}a`), /1 MiB/);
  assert.equal(await pages.read("too-big"), null);
});

test("counts bytes, not characters, against the limit", () => {
  assert.notEqual(htmlProblem("語".repeat(MAX_HTML_BYTES / 3 + 1)), null);
});

test("rejects an empty page", () => {
  assert.equal(htmlProblem(" \n "), "The page is empty.");
});

test("removes a page, and ignores one that is already gone", async () => {
  const { store: pages } = await store();
  await pages.save("page-2", "<p>x</p>");
  await pages.remove("page-2");
  await pages.remove("page-2");

  assert.equal(await pages.read("page-2"), null);
});
