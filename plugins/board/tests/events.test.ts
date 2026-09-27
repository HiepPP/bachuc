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

const settle = () => new Promise<void>((resolve) => setImmediate(resolve));

test("host lookup recovers after initial RPC failure and shares concurrent retry", async () => {
  const document = new EventTarget();
  Object.defineProperty(globalThis, "document", { value: document, configurable: true });
  let calls = 0;
  let opened = 0;
  let resolveHost!: (host: string) => void;
  const cleanup = installBoardEvents(
    () => opened++,
    () => {
      calls++;
      if (calls === 1) return Promise.reject(new Error("Transport disconnected"));
      return new Promise<string>((resolve) => {
        resolveHost = resolve;
      });
    },
  );
  try {
    await settle();
    document.dispatchEvent(
      new CustomEvent("paseo-board:v2:open", { detail: { serverId: "remote" } }),
    );
    document.dispatchEvent(
      new CustomEvent("paseo-board:v2:open", { detail: { serverId: "local" } }),
    );
    assert.equal(calls, 2);
    resolveHost("local");
    await settle();
    assert.equal(opened, 1);
    document.dispatchEvent(
      new CustomEvent("paseo-board:v2:open", { detail: { serverId: "local" } }),
    );
    await settle();
    assert.equal(opened, 2);
    assert.equal(calls, 2);
  } finally {
    cleanup();
    Reflect.deleteProperty(globalThis, "document");
  }
});

test("host lookup completion after cleanup never opens Board", async () => {
  const document = new EventTarget();
  Object.defineProperty(globalThis, "document", { value: document, configurable: true });
  let opened = 0;
  let resolveHost!: (host: string) => void;
  const cleanup = installBoardEvents(
    () => opened++,
    () =>
      new Promise<string>((resolve) => {
        resolveHost = resolve;
      }),
  );
  try {
    document.dispatchEvent(
      new CustomEvent("paseo-board:v2:open", { detail: { serverId: "local" } }),
    );
    cleanup();
    resolveHost("local");
    await settle();
    assert.equal(opened, 0);
  } finally {
    cleanup();
    Reflect.deleteProperty(globalThis, "document");
  }
});
