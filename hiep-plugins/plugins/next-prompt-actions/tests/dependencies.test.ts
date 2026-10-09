import { test } from "node:test";
import assert from "node:assert/strict";
import { writeFileSync, unlinkSync } from "node:fs";
import { randomUUID } from "node:crypto";
import { tmpdir } from "node:os";
import path from "node:path";
import { dependencyWarning, readDependencies } from "../server/dependencies";

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

test("a disabled evaluator is unavailable even with its files, and warns nothing", () => {
  const file = path.join(tmpdir(), `npa-dependencies-${randomUUID()}.json`);
  const root = path.resolve(import.meta.dirname, "../../jev-evaluator");
  try {
    writeFileSync(
      file,
      JSON.stringify({
        plugins: { board: {}, "jev-evaluator": { path: root, enabled: false } },
      }),
    );
    const disabled = readDependencies(file);
    assert.deepEqual(disabled, { board: true, evaluator: false, evaluatorOff: true });
    assert.equal(dependencyWarning(disabled), "");
    assert.match(dependencyWarning({ board: true, evaluator: false }), /Jev evaluator unavailable/);
  } finally {
    unlinkSync(file);
  }
});
