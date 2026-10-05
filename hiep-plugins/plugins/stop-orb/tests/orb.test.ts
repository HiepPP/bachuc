import { test } from "node:test";
import assert from "node:assert/strict";
import { isDarkSurface, solvingOrb } from "../shared/orb";

test("sizes the solving orb so its sphere matches the composer icons", () => {
  const orb = solvingOrb(16);
  assert.equal(orb.size, 20);
  assert.ok(Math.abs(orb.size * orb.scale * 0.82 - 16) < 1e-9);
});

test("reads light and dark surfaces", () => {
  assert.equal(isDarkSurface("#FCFCFB"), false);
  assert.equal(isDarkSurface("#1e1e2e"), true);
  assert.equal(isDarkSurface("rgb(255, 255, 255)"), false);
  assert.equal(isDarkSurface("not-a-color"), true);
});
