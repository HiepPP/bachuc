import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { locate } from "../server/locate";

const dir = mkdtempSync(path.join(tmpdir(), "edit-diffs-"));
const file = path.join(dir, "x.ts");
writeFileSync(file, "line1\nline2\nconst a = 2;\nline4\nfoo(bar);\n");

test("finds applied new text, then pending old text, and resolves relative paths", async () => {
  const { located } = await locate({
    filePath: "x.ts",
    cwd: dir,
    hunks: [
      { oldText: "const a = 1;", newText: "const a = 2;" },
      { oldText: "line4", newText: "line4 changed" },
      { oldText: "bar", newText: "baz" },
    ],
  });
  assert.deepEqual(located, [
    { line: 3, side: "new" },
    { line: 4, side: "old" },
    { line: 5, side: "old" },
  ]);
});

test("returns null for text or files it cannot find", async () => {
  assert.deepEqual(
    (await locate({ filePath: file, cwd: null, hunks: [{ oldText: "x1", newText: "x2" }] }))
      .located,
    [null],
  );
  assert.deepEqual(
    (await locate({ filePath: "missing.ts", cwd: null, hunks: [{ oldText: "", newText: "a" }] }))
      .located,
    [null],
  );
});
