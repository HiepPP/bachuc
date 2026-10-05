import { test } from "node:test";
import assert from "node:assert/strict";
import {
  applyLocations,
  buildHunks,
  countChanges,
  lineDiff,
  needsLocating,
  numberRows,
  parseUnifiedDiff,
  sideText,
} from "../shared/diff";

test("line diff keeps shared lines as context", () => {
  assert.deepEqual(lineDiff("a\nb\nc", "a\nB\nc\nd"), [
    { kind: "context", text: "a" },
    { kind: "remove", text: "b" },
    { kind: "add", text: "B" },
    { kind: "context", text: "c" },
    { kind: "add", text: "d" },
  ]);
});

test("unified diff hunks number removed rows by old line and others by new line", () => {
  const diff = [
    "diff --git a/x.ts b/x.ts",
    "--- a/x.ts",
    "+++ b/x.ts",
    "@@ -10,3 +10,3 @@ fn",
    " keep",
    "-old",
    "+new",
    " tail",
    "@@ -40 +40,2 @@",
    "-gone",
    "+one",
    "+two",
  ].join("\n");
  const hunks = parseUnifiedDiff(diff);
  assert.equal(hunks.length, 2);
  assert.deepEqual(
    numberRows(hunks[0]).map((row) => [row.kind, row.line]),
    [
      ["context", 10],
      ["remove", 11],
      ["add", 11],
      ["context", 12],
    ],
  );
  assert.deepEqual(
    numberRows(hunks[1]).map((row) => row.line),
    [40, 40, 41],
  );
  assert.deepEqual(countChanges(hunks), { added: 3, removed: 2 });
});

test("a removed line that looks like a file header stays a row inside a counted hunk", () => {
  const hunks = parseUnifiedDiff("@@ -1,2 +1,2 @@\n--- a\n+++ b\n");
  assert.deepEqual(hunks[0].rows, [
    { kind: "remove", text: "-- a" },
    { kind: "add", text: "++ b" },
  ]);
});

test("bare Codex hunks have no starts until located", () => {
  const hunks = parseUnifiedDiff("--- a/x\n+++ b/x\n@@\n ctx\n-a\n+b\n+c\n@@\n-d\n+e");
  assert.deepEqual(
    hunks.map((hunk) => [hunk.oldStart, hunk.newStart]),
    [
      [null, null],
      [null, null],
    ],
  );
  assert.equal(sideText(hunks[0], "new"), "ctx\nb\nc");
  assert.equal(sideText(hunks[0], "old"), "ctx\na");
  // The first hunk adds a net line, so the second hunk's old start trails its new start by one.
  const located = applyLocations(hunks, [
    { line: 5, side: "new" },
    { line: 20, side: "new" },
  ]);
  assert.deepEqual(
    located.map((hunk) => [hunk.oldStart, hunk.newStart]),
    [
      [5, 5],
      [19, 20],
    ],
  );
});

test("old and new strings become one hunk, numbered after locating", () => {
  const hunks = buildHunks({ oldString: "const a = 1;", newString: "const a = 2;\nconst b = 3;" });
  assert.equal(hunks.length, 1);
  assert.deepEqual(
    numberRows(hunks[0]).map((row) => row.line),
    [null, null, null],
  );
  const [located] = applyLocations(hunks, [{ line: 7, side: "new" }]);
  assert.deepEqual(
    numberRows(located).map((row) => [row.kind, row.line]),
    [
      ["remove", 7],
      ["add", 7],
      ["add", 8],
    ],
  );
});

test("written content becomes added rows numbered from line 1, with no locating", () => {
  const hunks = buildHunks({ content: "one\ntwo\n" });
  assert.deepEqual(
    numberRows(hunks[0]).map((row) => [row.kind, row.line]),
    [
      ["add", 1],
      ["add", 2],
    ],
  );
  assert.equal(needsLocating(hunks), false);
  assert.deepEqual(buildHunks({ content: "" }), []);
});
