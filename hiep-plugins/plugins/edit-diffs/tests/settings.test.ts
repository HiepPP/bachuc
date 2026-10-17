import { test } from "node:test";
import assert from "node:assert/strict";
import { displaySettings } from "../shared/settings";

test("diffs start collapsed when nothing is saved", () => {
  assert.deepEqual(displaySettings.schema.parse({}), { autoExpand: false });
});

test("a saved choice wins over the default", () => {
  assert.deepEqual(displaySettings.schema.parse({ autoExpand: true }), { autoExpand: true });
});
