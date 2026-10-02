import assert from "node:assert/strict";
import test from "node:test";
import { focusRing, keyboardFocus } from "../client/focus";

test("the focus ring follows keyboard focus, not clicks", () => {
  const element = (visible: boolean) => ({
    matches: (selector: string) => visible && selector === ":focus-visible",
  });
  assert.equal(keyboardFocus({ nativeEvent: { target: element(true) } }), true);
  assert.equal(keyboardFocus({ nativeEvent: { target: element(false) } }), false);
  assert.equal(keyboardFocus({ target: element(false) }), false);
  // Native targets are node handles; focus there only comes from a keyboard.
  assert.equal(keyboardFocus({ nativeEvent: { target: 42 } }), true);
  assert.equal(keyboardFocus(undefined), true);
  // Engines without :focus-visible keep the ring rather than lose it.
  assert.equal(
    keyboardFocus({
      target: {
        matches: () => {
          throw new Error("unsupported selector");
        },
      },
    }),
    true,
  );
  assert.deepEqual(focusRing("green", -2), {
    outlineStyle: "solid",
    outlineWidth: 2,
    outlineColor: "green",
    outlineOffset: -2,
  });
});
