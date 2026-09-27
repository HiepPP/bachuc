import { test } from "node:test";
import assert from "node:assert/strict";
import { installBoardEvents, subscribeSendResult } from "../client/events";

test("Board events open the Board and report send outcomes, keeping an early failure", () => {
  const document = new EventTarget();

  Object.defineProperty(globalThis, "document", { value: document, configurable: true });
  let opened = 0;
  const cleanup = installBoardEvents(
    () => opened++,
    () => "local",
  );
  try {
    document.dispatchEvent(
      new CustomEvent("paseo-board:v2:open", { detail: { serverId: "local" } }),
    );
    assert.equal(opened, 1);
    document.dispatchEvent(
      new CustomEvent("paseo-board:v2:open", { detail: { serverId: "remote" } }),
    );
    document.dispatchEvent(new Event("paseo-board:v2:open"));
    assert.equal(opened, 1);
    // Failure before the Board mounts is delivered on the next subscription only.
    document.dispatchEvent(
      new CustomEvent("paseo-board:v2:send-failed", { detail: { serverId: "local" } }),
    );
    const results: boolean[] = [];
    const unsubscribe = subscribeSendResult((sent) => results.push(sent));
    document.dispatchEvent(
      new CustomEvent("paseo-board:v2:sent", { detail: { serverId: "local" } }),
    );
    unsubscribe();
    assert.deepEqual(results, [false, true]);
    const later: boolean[] = [];
    subscribeSendResult((sent) => later.push(sent))();
    assert.deepEqual(later, []);
  } finally {
    cleanup();
    Reflect.deleteProperty(globalThis, "document");
  }
});

test("missing local Board never opens remote Board, including legacy events", () => {
  const document = new EventTarget();
  Object.defineProperty(globalThis, "document", { value: document, configurable: true });
  let opened = 0;
  const cleanup = installBoardEvents(
    () => opened++,
    () => "remote",
  );
  try {
    document.dispatchEvent(
      new CustomEvent("paseo-board:v2:open", { detail: { serverId: "local" } }),
    );
    document.dispatchEvent(new Event("paseo-board:open"));
    assert.equal(opened, 0);
    document.dispatchEvent(
      new CustomEvent("paseo-board:v2:open", { detail: { serverId: "remote" } }),
    );
    assert.equal(opened, 1);
  } finally {
    cleanup();
    Reflect.deleteProperty(globalThis, "document");
  }
});
