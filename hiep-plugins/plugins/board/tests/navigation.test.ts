import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { runInNewContext } from "node:vm";
import test from "node:test";
import ts from "typescript";

function compile(path: string) {
  return ts.transpileModule(readFileSync(new URL(path, import.meta.url), "utf8"), {
    compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX },
  }).outputText;
}

type Listener = (event: Record<string, unknown>) => void;

// Two host installations evaluate the same bundles in one window.
function harness() {
  const listeners: Listener[] = [];
  const opened: string[] = [];
  const context = {
    globalThis: {} as Record<string, unknown>,
    navigator: { userAgent: "Mozilla/5.0 (Macintosh; Mac OS X) Electron/38" },
    window: {
      addEventListener: (_type: string, listener: Listener) => listeners.push(listener),
      removeEventListener: (_type: string, listener: Listener) =>
        listeners.splice(listeners.indexOf(listener), 1),
    },
  };
  const shortcut: { installBoardShortcut?: (open: () => void) => () => void } = {};
  runInNewContext(compile("../client/shortcut.ts"), { ...context, exports: shortcut });
  const cleanups = ["macmini", "macair"].map((host) =>
    shortcut.installBoardShortcut!(() => opened.push(host)),
  );
  const press = () => {
    const event = {
      key: "d",
      metaKey: true,
      ctrlKey: false,
      altKey: false,
      shiftKey: false,
      repeat: false,
      isComposing: false,
      preventDefault() {},
      stopImmediatePropagation() {},
    };
    for (const listener of listeners.slice()) listener(event);
  };
  return { opened, press, cleanups, listeners };
}

test("one installation owns the Board shortcut, so a press opens one Board", () => {
  const h = harness();
  h.press();
  assert.deepEqual(h.opened, ["macmini"]);
  assert.equal(h.listeners.length, 1);
});

test("the owner's cleanup releases the shortcut", () => {
  const h = harness();
  for (const cleanup of h.cleanups) cleanup();
  assert.equal(h.listeners.length, 0);
});
