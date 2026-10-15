import assert from "node:assert/strict";
import test from "node:test";
import { createPrefetchIntent, PREFETCH_INTENT_MS } from "../client/prefetch";

function setup(t: test.TestContext) {
  t.mock.timers.enable({ apis: ["setTimeout"] });
  const state = { calls: 0 };
  const prefetch = () => {
    state.calls += 1;
  };
  const tick = (ms: number) => t.mock.timers.tick(ms);
  return { intent: createPrefetchIntent(), prefetch, state, tick };
}

test("hover warms the thread only after 120 ms of rest", (t) => {
  const { intent, prefetch, state, tick } = setup(t);
  assert.equal(PREFETCH_INTENT_MS, 120);
  intent.hoverIn(prefetch);
  tick(119);
  assert.equal(state.calls, 0);
  tick(1);
  assert.equal(state.calls, 1);
  tick(1_000);
  assert.equal(state.calls, 1);
});

test("hover out before 120 ms cancels the warm-up", (t) => {
  const { intent, prefetch, state, tick } = setup(t);
  intent.hoverIn(prefetch);
  tick(100);
  intent.hoverOut();
  tick(1_000);
  assert.equal(state.calls, 0);
  // A new hover starts a fresh 120 ms wait.
  intent.hoverIn(prefetch);
  tick(119);
  assert.equal(state.calls, 0);
  tick(1);
  assert.equal(state.calls, 1);
});

test("press-in warms at once and replaces a pending hover warm-up", (t) => {
  const { intent, prefetch, state, tick } = setup(t);
  intent.hoverIn(prefetch);
  tick(50);
  intent.pressIn(prefetch);
  assert.equal(state.calls, 1);
  tick(1_000);
  assert.equal(state.calls, 1);
});

test("cancel stops a pending warm-up", (t) => {
  const { intent, prefetch, state, tick } = setup(t);
  intent.hoverIn(prefetch);
  intent.cancel();
  tick(1_000);
  assert.equal(state.calls, 0);
});
