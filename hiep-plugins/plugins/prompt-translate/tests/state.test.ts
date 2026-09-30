import { test } from "node:test";
import assert from "node:assert/strict";
import { current } from "../client/state";

test("switching translation back on moves activeSince forward and notifies", async () => {
  let changes = 0;
  current.onChange = () => void changes++;
  current.apply({ ...current.values, translate: false });
  const before = current.activeSince;
  await new Promise((resolve) => setTimeout(resolve, 5));
  current.apply({ ...current.values, translate: true });
  assert.ok(current.activeSince > before);
  assert.equal(changes, 2);
  current.onChange = undefined;
});
