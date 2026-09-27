import { test } from "node:test";
import assert from "node:assert/strict";
import { writeFileSync, unlinkSync } from "node:fs";
import { randomUUID } from "node:crypto";
import { tmpdir } from "node:os";
import path from "node:path";
import { readDependencies } from "../server/dependencies";

test("missing registration, disabled Board, and missing evaluator files are detected", () => {
  const file = path.join(tmpdir(), `npa-dependencies-${randomUUID()}.json`);
  assert.deepEqual(readDependencies(file), { board: false, evaluator: false });
  try {
    writeFileSync(
      file,
      JSON.stringify({
        plugins: { board: { enabled: false }, "jev-evaluator": { path: "/missing-evaluator" } },
      }),
    );
    assert.deepEqual(readDependencies(file), { board: false, evaluator: false });
    writeFileSync(file, JSON.stringify({ plugins: { board: { enabled: true } } }));
    assert.deepEqual(readDependencies(file), { board: true, evaluator: false });
  } finally {
    unlinkSync(file);
  }
});
