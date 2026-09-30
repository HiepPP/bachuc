import assert from "node:assert/strict";
import test from "node:test";
import { bindBoardShortcut } from "../client/shortcut";

test("Cmd+D opens Board once, suppresses dictation, and unregisters on cleanup", () => {
  let listener:
    | Parameters<Parameters<typeof bindBoardShortcut>[0]["addEventListener"]>[1]
    | undefined;
  let opens = 0;
  let prevented = 0;
  let stopped = 0;
  const cleanup = bindBoardShortcut(
    {
      addEventListener(_type, fn, capture) {
        assert.equal(capture, true);
        listener = fn;
      },
      removeEventListener(_type, fn, capture) {
        assert.equal(fn, listener);
        assert.equal(capture, true);
        listener = undefined;
      },
    },
    () => {
      opens++;
    },
  );
  const event = {
    key: "d",
    metaKey: true,
    ctrlKey: false,
    altKey: false,
    shiftKey: false,
    repeat: false,
    isComposing: false,
    preventDefault() {
      prevented++;
    },
    stopImmediatePropagation() {
      stopped++;
    },
  };
  listener!(event);
  listener!({ ...event, repeat: true });
  listener!({ ...event, metaKey: false });
  listener!({ ...event, shiftKey: true });
  listener!({ ...event, isComposing: true });
  assert.equal(opens, 1);
  assert.equal(prevented, 2);
  assert.equal(stopped, 2);
  cleanup();
  assert.equal(listener, undefined);
});
