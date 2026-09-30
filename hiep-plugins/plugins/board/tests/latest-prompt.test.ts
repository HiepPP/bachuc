import assert from "node:assert/strict";
import test from "node:test";
import {
  PROMPT_TOP_INSET_PX,
  revealLatestPrompt,
  type PromptTarget,
} from "../client/latest-prompt";

// A transcript whose latest prompt sits `promptTop` px from the viewport top at scrollTop 0.
function harness(options: { promptTop: number | null; room?: number }) {
  let time = 0;
  let pending: (() => void) | null = null;
  let inputListener: (() => void) | null = null;
  let changeListener: (() => void) | null = null;
  const view = { scrollTop: 0, promptTop: options.promptTop, room: options.room ?? 0 };
  const scrolls: number[] = [];
  const env = {
    now: () => time,
    frame(callback: () => void) {
      pending = callback;
      return 1;
    },
    cancelFrame() {
      pending = null;
    },
    find(): PromptTarget | null {
      if (view.promptTop === null) return null;
      return {
        offset: view.promptTop - view.scrollTop,
        room: view.room,
        scrollBy(delta) {
          scrolls.push(delta);
          view.scrollTop += delta;
          view.room -= delta;
        },
      };
    },
    onChange(listener: () => void) {
      changeListener = listener;
      return () => {
        changeListener = null;
      };
    },
    onUserInput(listener: () => void) {
      inputListener = listener;
      return () => {
        inputListener = null;
      };
    },
  };
  return {
    view,
    scrolls,
    env,
    advance(ms: number, step = 16) {
      for (let elapsed = 0; elapsed < ms && pending; elapsed += step) {
        time += step;
        const callback = pending;
        pending = null;
        callback();
      }
    },
    change: () => changeListener?.(),
    input: () => inputListener?.(),
    get active() {
      return pending !== null || inputListener !== null || changeListener !== null;
    },
  };
}

test("places the latest prompt as soon as the transcript is in the DOM", () => {
  const h = harness({ promptTop: null });
  revealLatestPrompt(h.env);
  h.view.promptTop = -900;
  h.change();
  assert.deepEqual(h.scrolls, [-900 - PROMPT_TOP_INSET_PX], "no frame or settle wait");
});

test("undoes Paseo's bottom re-pin in the same change, before a paint", () => {
  const h = harness({ promptTop: -900 });
  revealLatestPrompt(h.env);
  h.view.scrollTop = 0;
  h.change();
  assert.equal(h.scrolls.length, 2);
  assert.equal(h.view.promptTop! - h.view.scrollTop, PROMPT_TOP_INSET_PX);
});

test("follows the prompt down when rows above it grow", () => {
  const h = harness({ promptTop: -900, room: 500 });
  revealLatestPrompt(h.env);
  h.view.promptTop! += 50;
  h.change();
  assert.deepEqual(h.scrolls.at(-1), 50);
});

test("leaves a prompt that is already on screen in a short thread alone", () => {
  const h = harness({ promptTop: 120 });
  revealLatestPrompt(h.env);
  h.advance(3000);
  assert.deepEqual(h.scrolls, []);
  assert.equal(h.active, false);
});

test("stops holding the prompt after the thread has opened", () => {
  const h = harness({ promptTop: -900 });
  revealLatestPrompt(h.env);
  h.advance(2000);
  assert.equal(h.active, false);
  h.view.scrollTop = 0;
  h.change();
  assert.equal(h.scrolls.length, 1);
});

test("user input cancels the reveal", () => {
  const h = harness({ promptTop: null });
  revealLatestPrompt(h.env);
  h.input();
  h.view.promptTop = -900;
  h.change();
  h.advance(1000);
  assert.deepEqual(h.scrolls, []);
  assert.equal(h.active, false);
});

test("gives up when no transcript appears", () => {
  const h = harness({ promptTop: null });
  revealLatestPrompt(h.env);
  h.advance(6000);
  assert.equal(h.active, false);
});
