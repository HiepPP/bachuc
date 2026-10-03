import { test } from "node:test";
import assert from "node:assert/strict";
import { createNoteCache } from "../client/notes";

test("each distinct text is looked up once across renders", async () => {
  const calls: string[] = [];
  const lookup = createNoteCache({
    translate: async (text) => {
      calls.push(`translate:${text}`);
      return "Explain";
    },
    original: async (text) => {
      calls.push(`original:${text}`);
      if (text === "Fix the bug.") return "fix teh bug";
      return text === "Explain the bug" ? "Giải thích lỗi" : null;
    },
  });
  for (let render = 0; render < 5; render++) {
    assert.deepEqual(await lookup("Giải thích", false), { label: "EN", text: "Explain" });
    assert.deepEqual(await lookup("Explain the bug", false), {
      label: "VI gốc",
      text: "Giải thích lỗi",
    });
    assert.deepEqual(await lookup("Fix the bug.", false), { label: "EN gốc", text: "fix teh bug" });
    assert.equal(await lookup("Plain English", false), null);
  }
  assert.deepEqual(calls, [
    "translate:Giải thích",
    "original:Explain the bug",
    "original:Fix the bug.",
    "original:Plain English",
  ]);
});

test("a failed lookup is retried on the next render", async () => {
  let attempts = 0;
  const lookup = createNoteCache({
    translate: async () => {
      attempts++;
      if (attempts === 1) throw new Error("offline");
      return "Explain";
    },
    original: async () => null,
  });
  assert.equal(await lookup("Giải thích", false), null);
  assert.deepEqual(await lookup("Giải thích", false), { label: "EN", text: "Explain" });
  assert.equal(attempts, 2);
});
