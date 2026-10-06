import assert from "node:assert/strict";
import { randomUUID } from "node:crypto";
import test from "node:test";
import { cleanTitle, htmlReplyRowSchema, MAX_TITLE_LENGTH, rowDataBytes } from "../shared/row";

const ROW_LIMIT_BYTES = 1024;

test("keeps every row under 1 KiB, even for the worst-case title", () => {
  const worstTitles = [
    "語".repeat(500),
    '"'.repeat(500),
    "\\".repeat(500),
    "\u0001".repeat(500),
    "😀".repeat(500),
  ];
  for (const raw of worstTitles) {
    const row = htmlReplyRowSchema.parse({ id: randomUUID(), title: cleanTitle(raw) });
    assert.ok(rowDataBytes(row) < ROW_LIMIT_BYTES, `${rowDataBytes(row)} bytes for ${raw[0]}`);
  }
});

test("makes a one-line title of at most 200 characters", () => {
  assert.equal(cleanTitle("  Sales\n\tby\u0000 region  "), "Sales by region");
  assert.equal(cleanTitle("x".repeat(300)).length, MAX_TITLE_LENGTH);
});

test("accepts only plain page ids in the row", () => {
  assert.equal(htmlReplyRowSchema.safeParse({ id: randomUUID(), title: "t" }).success, true);
  assert.equal(htmlReplyRowSchema.safeParse({ id: "../x", title: "t" }).success, false);
});
