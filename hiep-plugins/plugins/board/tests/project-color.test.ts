import assert from "node:assert/strict";
import test from "node:test";
import { allocateColors, projectColors } from "../shared/project-colors";

test("new projects maximize hue distance and never recolor existing IDs", () => {
  const initial = allocateColors({}, ["b", "a"]);
  assert.deepEqual(initial, { a: 0, b: 180 });
  const restored = projectColors.schema.parse(JSON.parse(JSON.stringify({ hues: initial })));
  const next = allocateColors(restored.hues, ["c", "a", "b"]);
  assert.deepEqual(next, { a: 0, b: 180, c: 90 });
  assert.deepEqual(initial, { a: 0, b: 180 });
  assert.equal(allocateColors(next, ["c", "b", "a"]), next);
  assert.deepEqual(allocateColors({}, ["b", "a", "b"]), initial);
});
