import { test } from "node:test";
import assert from "node:assert/strict";
import { CHANGE_BAR_SQUARES, changeBar } from "../shared/change-bar";
import { buildHunks, countChanges } from "../shared/diff";

function counts(bar: ReturnType<typeof changeBar>) {
  return {
    add: bar.filter((square) => square === "add").length,
    remove: bar.filter((square) => square === "remove").length,
  };
}

test("a write fills all 7 squares green", () => {
  const { added, removed } = countChanges(buildHunks({ content: "one\ntwo\nthree\n" }));
  assert.equal(CHANGE_BAR_SQUARES, 7);
  assert.deepEqual(changeBar(added, removed), Array(7).fill("add"));
});

test("a pure delete fills all 7 squares red", () => {
  const { added, removed } = countChanges(buildHunks({ oldString: "a\nb", newString: "" }));
  assert.deepEqual(changeBar(added, removed), Array(7).fill("remove"));
});

test("a mixed edit splits the squares by share, green first", () => {
  assert.deepEqual(counts(changeBar(4, 3)), { add: 4, remove: 3 });
  assert.deepEqual(changeBar(4, 3), ["add", "add", "add", "add", "remove", "remove", "remove"]);
  assert.deepEqual(counts(changeBar(1, 1)), { add: 4, remove: 3 });
});

test("a small side keeps one square when both sides changed", () => {
  assert.deepEqual(counts(changeBar(100, 1)), { add: 6, remove: 1 });
  assert.deepEqual(counts(changeBar(1, 100)), { add: 1, remove: 6 });
});

test("no changes draw no squares", () => {
  assert.deepEqual(changeBar(0, 0), []);
});
