import { test } from "node:test";
import assert from "node:assert/strict";
import { parseHTML } from "linkedom";
import { installComposer } from "../client/composer";
import { composerHost } from "../client/agent-mode";
import type { Doc, DomEvent, El, Key } from "../client/dom";
import { translateSettings } from "../shared/settings";

type Sent = { key: string; metaKey: boolean; value: string | undefined };
function page(value: string) {
  const { document, window } = parseHTML(
    '<html><head></head><body><div data-testid="message-input-root"><textarea data-composer-input></textarea><button id="send" role="button">Send</button></div><p id="other"></p></body></html>',
  );
  // linkedom has no KeyboardEvent; Electron does.
  (window as unknown as Record<string, unknown>).KeyboardEvent = class extends window.Event {
    key: string;
    metaKey = false;
    constructor(type: string, init: { key: string; bubbles?: boolean; cancelable?: boolean }) {
      super(type, init);
      this.key = init.key;
    }
  };
  const field = document.querySelector("textarea") as unknown as El;
  Object.assign(field, {
    __reactFiber$test: { memoizedProps: { voiceAgentId: "0c5be16b-a9e0-44c4-8caa-db8aff761e2f" } },
  });
  field.value = value;
  const sent: Sent[] = [];
  (
    field as unknown as { addEventListener(n: string, h: (e: Sent) => void): void }
  ).addEventListener("keydown", (event) =>
    sent.push({ key: event.key, metaKey: event.metaKey, value: field.value }),
  );
  return { doc: document as unknown as Doc, field, sent, other: document.getElementById("other") };
}
const prepared: { mode: string; text: string; source: string }[] = [];
let selected: typeof follow.cavemanMode = "follow-agent";
const hookApi = {
  bindQueue: async () => ({}),
  cancelQueue: async () => ({}),
  mode: async () => selected,
  prepare: async (input: { mode: typeof selected; text: string; source: string }) => {
    prepared.push(input);
    return { token: "test" };
  },
  cancel: async () => ({}),
};
const fast = { send: 0, verify: 20 };
const follow = translateSettings.schema.parse({});
const wait = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));
function key(target: unknown, init: Partial<Key> = {}) {
  const calls = { prevented: 0, stopped: 0 };
  const event: Key = {
    key: "Enter",
    metaKey: true,
    ctrlKey: false,
    shiftKey: false,
    isComposing: false,
    target,
    preventDefault: () => void calls.prevented++,
    stopImmediatePropagation: () => void calls.stopped++,
    ...init,
  };
  return { event, calls };
}
function deferred() {
  let resolve!: (value: string) => void;
  let reject!: (error: Error) => void;
  const promise = new Promise<string>((ok, fail) => {
    resolve = ok;
    reject = fail;
  });
  return { promise, resolve, reject };
}
function multiHostPage(value: string, hostId: string) {
  const { document, window } = parseHTML(
    '<html><head></head><body><div data-testid="message-input-root"><textarea data-composer-input></textarea><button id="send" role="button">Send</button></div></body></html>',
  );
  (window as unknown as Record<string, unknown>).KeyboardEvent = class extends window.Event {
    key: string;
    metaKey: boolean;
    ctrlKey: boolean;
    constructor(
      type: string,
      init: { key: string; metaKey?: boolean; ctrlKey?: boolean; bubbles?: boolean },
    ) {
      super(type, init);
      this.key = init.key;
      this.metaKey = Boolean(init.metaKey);
      this.ctrlKey = Boolean(init.ctrlKey);
    }
  };
  const field = document.querySelector("textarea") as unknown as El;
  const props = {
    voiceAgentId: "0c5be16b-a9e0-44c4-8caa-db8aff761e2f",
    voiceServerId: hostId,
  };
  Object.assign(field, { __reactFiber$test: { memoizedProps: props } });
  field.value = value;
  return {
    doc: document as unknown as Doc,
    field,
    button: document.querySelector("#send") as unknown as El,
    props,
  };
}

function captureDispatcher(
  doc: Doc,
  field: El,
  button: El,
  deliver: (type: "keydown" | "click", event: Key & DomEvent) => void,
) {
  type Handler = (event: Key & DomEvent) => void;
  const handlers = { keydown: [] as Handler[], click: [] as Handler[] };
  const target = doc as unknown as {
    addEventListener(name: string, handler: Handler, capture?: boolean): void;
    removeEventListener(name: string, handler: Handler, capture?: boolean): void;
  };
  target.addEventListener = (name, handler, capture) => {
    if (capture && (name === "keydown" || name === "click")) handlers[name].push(handler);
  };
  target.removeEventListener = (name, handler, capture) => {
    if (!capture || (name !== "keydown" && name !== "click")) return;
    const index = handlers[name].indexOf(handler);
    if (index >= 0) handlers[name].splice(index, 1);
  };
  let dispatches = 0;
  const dispatch = (type: "keydown" | "click", eventTarget: El, init: Partial<Key> = {}) => {
    if (++dispatches > 20) throw new Error("runaway synthetic replay");
    let stopped = false;
    const event = {
      key: type === "keydown" ? "Enter" : "",
      metaKey: false,
      ctrlKey: false,
      shiftKey: false,
      isComposing: false,
      target: eventTarget,
      preventDefault() {},
      stopPropagation() {},
      stopImmediatePropagation() {
        stopped = true;
      },
      ...init,
    } as Key & DomEvent;
    for (const handler of handlers[type]) {
      handler(event);
      if (stopped) return false;
    }
    deliver(type, event);
    return true;
  };
  field.dispatchEvent = ((event: Key & { type?: string }) => {
    if (event.type !== "keydown") return true;
    return dispatch("keydown", field, {
      key: event.key,
      metaKey: event.metaKey,
      ctrlKey: event.ctrlKey,
      shiftKey: event.shiftKey,
      isComposing: event.isComposing,
      keyCode: event.keyCode,
    });
  }) as El["dispatchEvent"];
  button.click = () => void dispatch("click", button);
  return {
    keydown: (init: Partial<Key> = {}) => dispatch("keydown", field, init),
    click: () => dispatch("click", button),
  };
}

async function eventually(check: () => boolean) {
  for (let attempt = 0; attempt < 20; attempt++) {
    if (check()) return;
    await settle();
  }
  assert.fail("timed out waiting for composer replay");
}
const badge = (doc: Doc) =>
  (doc as unknown as { querySelector(selector: string): El | null }).querySelector(
    "[data-prompt-translate-badge]",
  );
const settle = () => new Promise((resolve) => setTimeout(resolve, 0));

for (const destination of ["host-a", "host-b"])
  for (const order of [
    ["host-a", "host-b"],
    ["host-b", "host-a"],
  ] as const)
    test(`${destination} composer owns Enter and Send with ${order.join(" then ")} listeners`, async () => {
      const { doc, field, button } = multiHostPage("keyboard draft", destination);
      const delivered = { keydown: 0, click: 0 };
      const dispatcher = captureDispatcher(doc, field, button, (type) => {
        delivered[type]++;
        field.value = "";
      });
      const calls = new Map<string, string[]>([
        ["host-a", []],
        ["host-b", []],
      ]);
      const composers = order.map((hostId) => {
        let rpcCalls = 0;
        const bounded = <T>(value: T) => {
          if (++rpcCalls > 8) return Promise.reject<T>(new Error("runaway composer RPC loop"));
          return Promise.resolve(value);
        };
        return installComposer(
          {
            ...hookApi,
            enhance: async (text) => text,
            mode: async () => {
              calls.get(hostId)!.push("mode");
              return bounded("follow-agent" as const);
            },
            prepare: async ({ text }) => {
              calls.get(hostId)!.push(`prepare:${text}`);
              return bounded({ token: `${hostId}-token` });
            },
          },
          {
            enabled: () => true,
            settings: () => follow,
            owns: (node) => composerHost(node) === hostId,
          },
          doc,
          { send: 0, verify: 20 },
        );
      });
      try {
        dispatcher.keydown();
        await eventually(() => delivered.keydown === 1);
        field.value = "button draft";
        dispatcher.click();
        await eventually(() => delivered.click === 1);
        assert.deepEqual(delivered, { keydown: 1, click: 1 });
        assert.deepEqual(calls.get(destination), [
          "mode",
          "prepare:keyboard draft",
          "mode",
          "mode",
          "prepare:button draft",
          "mode",
        ]);
        assert.deepEqual(calls.get(destination === "host-a" ? "host-b" : "host-a"), []);
      } finally {
        for (const composer of composers) composer.stop();
      }
    });

test("composer host ownership follows React's active fiber buffer", () => {
  const { field } = multiHostPage("draft", "stale-host");
  const activeRoot = {};
  const staleRoot = { stateNode: { current: activeRoot } };
  Object.assign(field, {
    __reactFiber$test: {
      memoizedProps: { voiceServerId: "stale-host" },
      return: staleRoot,
      alternate: {
        memoizedProps: { voiceServerId: "active-host" },
        return: activeRoot,
      },
    },
  });
  assert.equal(composerHost(field), "active-host");
});

test("host switch cancels a pending enhancement before replay", async () => {
  const { doc, field, button, props } = multiHostPage("draft", "host-a");
  let delivered = 0;
  const dispatcher = captureDispatcher(doc, field, button, () => delivered++);
  const pendingEnhancement = deferred();
  const composer = installComposer(
    { ...hookApi, enhance: () => pendingEnhancement.promise },
    {
      enabled: () => true,
      settings: () => follow,
      owns: (node) => composerHost(node) === "host-a",
    },
    doc,
    { send: 0, verify: 20 },
  );
  try {
    dispatcher.keydown({ metaKey: true });
    props.voiceServerId = "host-b";
    pendingEnhancement.resolve("enhanced");
    await wait(5);
    assert.equal(delivered, 0);
    assert.equal(field.value, "draft");
  } finally {
    composer.stop();
  }
});

test("Escape from another host does not cancel the local enhancement", async () => {
  const { doc, field, button, props } = multiHostPage("draft", "host-a");
  const delivered: string[] = [];
  const dispatcher = captureDispatcher(doc, field, button, (_type, event) => {
    delivered.push(event.key);
    if (event.key === "Enter") field.value = "";
  });
  const pendingEnhancement = deferred();
  const composer = installComposer(
    { ...hookApi, enhance: () => pendingEnhancement.promise },
    {
      enabled: () => true,
      settings: () => follow,
      owns: (node) => composerHost(node) === "host-a",
    },
    doc,
    { send: 0, verify: 20 },
  );
  try {
    dispatcher.keydown({ metaKey: true });
    props.voiceServerId = "host-b";
    dispatcher.keydown({ key: "Escape" });
    props.voiceServerId = "host-a";
    pendingEnhancement.resolve("enhanced");
    await eventually(() => delivered.includes("Enter"));
    assert.deepEqual(delivered, ["Escape", "Enter"]);
  } finally {
    composer.stop();
  }
});

test("Cmd+Enter enhances the draft, then sends it with a plain Enter", async () => {
  const { doc, field, sent } = page("sửa lỗi");
  const pending = deferred();
  const requests: string[] = [];
  const composer = installComposer(
    { ...hookApi, enhance: (text) => (requests.push(text), pending.promise) },
    { enabled: () => true, settings: () => follow },
    doc,
    fast,
  );
  const { event, calls } = key(field);
  composer.onKeydown(event);
  composer.onKeydown(key(field).event);
  assert.deepEqual(calls, { prevented: 1, stopped: 1 });
  assert.deepEqual(requests, ["sửa lỗi"]);
  assert.equal(badge(doc)?.textContent, "Enhancing… (Esc để hủy)");
  pending.resolve("Fix the bug.");
  await wait(5);
  assert.equal(sent.length, 1);
  assert.equal(sent[0].value, "Fix the bug.");
  // The host clears the composer after a successful send.
  field.value = "";
  await wait(30);
  assert.equal(badge(doc), null);
  composer.stop();
});

test("a send the host does not accept leaves the prompt and asks for Enter", async () => {
  const { doc, field } = page("sửa lỗi");
  const composer = installComposer(
    { ...hookApi, enhance: async () => "Fix the bug." },
    { enabled: () => true, settings: () => follow },
    doc,
    fast,
  );
  composer.onKeydown(key(field).event);
  await wait(40);
  assert.equal(field.value, "Fix the bug.");
  assert.equal(badge(doc)?.textContent, "Chưa gửi được, nhấn Enter để gửi");
  composer.stop();
  assert.equal(badge(doc), null);
});

test("other keys, other targets, IME, and the disabled setting are untouched", () => {
  const { doc, field, other } = page("x");
  let enabled = true;
  const requests: string[] = [];
  const composer = installComposer(
    { ...hookApi, enhance: async (text) => (requests.push(text), text) },
    { enabled: () => enabled, settings: () => follow },
    doc,
  );
  for (const init of [{ shiftKey: true }, { isComposing: true }, { keyCode: 229 }, { key: "a" }]) {
    const { event, calls } = key(field, init);
    composer.onKeydown(event);
    assert.deepEqual(calls, { prevented: 0, stopped: 0 });
  }
  composer.onKeydown(key(other).event);
  enabled = false;
  composer.onKeydown(key(field).event);
  assert.equal(requests.length, 0);
  composer.stop();
});

test("Ctrl+Enter works too, and an edited draft is never overwritten", async () => {
  const { doc, field, sent } = page("bản nháp");
  const pending = deferred();
  const composer = installComposer(
    { ...hookApi, enhance: () => pending.promise },
    { enabled: () => true, settings: () => follow },
    doc,
    fast,
  );
  composer.onKeydown(key(field, { metaKey: false, ctrlKey: true }).event);
  field.value = "bản nháp đã sửa";
  pending.resolve("Draft.");
  await settle();
  assert.equal(field.value, "bản nháp đã sửa");
  assert.deepEqual(sent, []);
  composer.stop();
});

test("Escape cancels, and errors keep the draft with a message", async () => {
  const { doc, field, sent } = page("hủy");
  const first = deferred();
  const second = deferred();
  const queue = [first.promise, second.promise];
  const composer = installComposer(
    { ...hookApi, enhance: () => queue.shift()! },
    { enabled: () => true, settings: () => follow },
    doc,
  );
  composer.onKeydown(key(field).event);
  const escape = key(field, { key: "Escape", metaKey: false });
  composer.onKeydown(escape.event);
  assert.deepEqual(escape.calls, { prevented: 1, stopped: 1 });
  first.resolve("Cancelled.");
  await settle();
  assert.equal(field.value, "hủy");
  assert.equal(badge(doc), null);
  composer.onKeydown(key(field).event);
  second.reject(new Error("timeout"));
  await settle();
  assert.equal(field.value, "hủy");
  assert.equal(badge(doc)?.textContent, "Enhance lỗi: timeout");
  assert.equal(sent.filter((event) => !event.metaKey).length, 0);
  composer.stop();
  assert.equal(badge(doc), null);
});

test("ordinary turns snapshot selected mode without changing text", async () => {
  const { doc, field, sent } = page("Giải thích git status.");
  const composer = installComposer(
    { ...hookApi, enhance: async (text) => text },
    { enabled: () => true, settings: () => follow },
    doc,
  );
  try {
    for (const mode of ["follow-agent", "wenyan-ultra", "lite", "follow-agent"] as const) {
      selected = mode;
      composer.onKeydown(key(field, { metaKey: false }).event);
      await settle();
      assert.equal(prepared.at(-1)?.mode, mode);
      assert.equal(sent.at(-1)?.value, "Giải thích git status.");
      assert.equal(field.value, "Giải thích git status.");
    }
  } finally {
    composer.stop();
  }
});

test("button send waits for hook preparation and keeps original text", async () => {
  const { doc, field } = page("click turn");
  const composer = installComposer(
    { ...hookApi, enhance: async (text) => text },
    { enabled: () => true, settings: () => follow },
    doc,
  );
  const button = field.parentElement!.querySelector("#send")!;
  let clicked = 0;
  button.addEventListener("click", () => {
    clicked++;
  });
  composer.onClick({
    target: button,
    preventDefault() {},
    stopPropagation() {},
    stopImmediatePropagation() {},
  });
  assert.equal(clicked, 0);
  await settle();
  assert.equal(clicked, 1);
  assert.equal(field.value, "click turn");
  composer.stop();
});

test("Enter leaves compact drafts and autocomplete selection untouched", () => {
  const { doc, field } = page("draft");
  const settings = translateSettings.schema.parse({ cavemanMode: "lite" });
  const composer = installComposer(
    { ...hookApi, enhance: async (text) => text },
    { enabled: () => true, settings: () => settings },
    doc,
  );
  try {
    const view = doc.defaultView!;
    view.innerWidth = 600;
    composer.onKeydown(key(field, { metaKey: false }).event);
    assert.equal(field.value, "draft");
    view.innerWidth = 1000;
    const popover = doc.createElement("div");
    popover.setAttribute("data-testid", "composer-autocomplete-popover");
    doc.body.append(popover);
    composer.onKeydown(key(field, { metaKey: false }).event);
    assert.equal(field.value, "draft");
  } finally {
    composer.stop();
  }
});

test("enhancement reads the latest mode at send, including a change after completion", async () => {
  const { doc, field, sent } = page("Giải thích git status.");
  const pending = deferred();
  let settings = translateSettings.schema.parse({ cavemanMode: "wenyan-ultra" });
  selected = "wenyan-ultra";
  const composer = installComposer(
    { ...hookApi, enhance: () => pending.promise },
    { enabled: () => true, settings: () => settings },
    doc,
    { send: 20, verify: 100 },
  );
  try {
    composer.onKeydown(key(field).event);
    settings = translateSettings.schema.parse({ cavemanMode: "lite" });
    pending.resolve("Explain git status.");
    await settle();
    settings = follow;
    selected = "follow-agent";
    await wait(30);
    assert.equal(sent.length, 1);
    assert.equal(sent[0].value, "Explain git status.");
    assert.equal(prepared.at(-1)?.mode, "follow-agent");
    assert.equal(prepared.at(-1)?.source, "Giải thích git status.");
  } finally {
    composer.stop();
  }
});

test("manual command survives enhancement despite a conflicting dropdown", async () => {
  const { doc, field, sent } = page("/caveman lite\n\nGiải thích git status.");
  const composer = installComposer(
    { ...hookApi, enhance: async () => "/caveman lite\n\nExplain git status." },
    { enabled: () => true, settings: () => follow },
    doc,
    fast,
  );
  try {
    composer.onKeydown(key(field).event);
    await wait(10);
    assert.equal(sent[0].value, "/caveman lite\n\nExplain git status.");
  } finally {
    composer.stop();
  }
});

test("selection changing during prepare cancels the old snapshot", async () => {
  const { doc, field, sent } = page("unchanged");
  selected = "lite";
  const cancelled: string[] = [];
  let count = 0;
  const composer = installComposer(
    {
      ...hookApi,
      enhance: async (text) => text,
      prepare: async () => {
        count++;
        if (count === 1) selected = "ultra";
        return { token: String(count) };
      },
      cancel: async (_id, token) => {
        cancelled.push(token);
      },
    },
    { enabled: () => true, settings: () => follow },
    doc,
  );
  composer.onKeydown(key(field, { metaKey: false }).event);
  await settle();
  assert.deepEqual(cancelled, ["1"]);
  assert.equal(count, 2);
  assert.equal(sent[0].value, "unchanged");
  composer.stop();
});

test("prepare failure does not send or modify the draft", async () => {
  const { doc, field, sent } = page("original");
  const composer = installComposer(
    {
      ...hookApi,
      enhance: async (text) => text,
      prepare: async () => {
        throw Error("hook unavailable");
      },
    },
    { enabled: () => true, settings: () => follow },
    doc,
  );
  composer.onKeydown(key(field, { metaKey: false }).event);
  await settle();
  assert.equal(sent.length, 0);
  assert.equal(field.value, "original");
  assert.match(badge(doc)?.textContent ?? "", /hook unavailable/);
  composer.stop();
});

test("Stop and voice buttons are never intercepted", () => {
  const { doc, field } = page("");
  const composer = installComposer(
    { ...hookApi, enhance: async (text) => text },
    { enabled: () => true, settings: () => follow },
    doc,
  );
  const button = field.parentElement!.querySelector("#send")!;
  let prevented = 0;
  const event = {
    target: button,
    preventDefault() {
      prevented++;
    },
    stopPropagation() {},
    stopImmediatePropagation() {},
  };
  composer.onClick(event);
  field.value = "draft while running";
  Object.assign(button, {
    __reactFiber$test: { memoizedProps: { onDefaultSendAction() {}, canPressLoadingButton: true } },
  });
  composer.onClick(event);
  assert.equal(prevented, 0);
  composer.stop();
});

test("switching agents while enhancement runs never sends into the new conversation", async () => {
  const { doc, field, sent } = page("draft");
  const result = deferred();
  const composer = installComposer(
    { ...hookApi, enhance: () => result.promise },
    { enabled: () => true, settings: () => follow },
    doc,
    fast,
  );
  composer.onKeydown(key(field).event);
  Object.assign(field, {
    __reactFiber$test: { memoizedProps: { voiceAgentId: "00000000-0000-4000-8000-000000000099" } },
  });
  result.resolve("enhanced");
  await wait(10);
  assert.equal(sent.length, 0);
  assert.equal(field.value, "draft");
  composer.stop();
});

test("queue edit waits for exact snapshot cancellation before restoring draft", async () => {
  const { doc, field } = page("same");
  const row = doc.createElement("div");
  const edit = doc.createElement("button");
  edit.setAttribute("aria-label", "Edit queued message");
  Object.assign(edit, {
    __reactFiber$test: {
      memoizedProps: {
        item: { id: "queue-one", text: "same" },
        onEdit() {},
        onSendNow() {},
        editLabel: "Edit queued message",
      },
    },
  });
  row.append(edit);
  let bound = "";
  let cancelled = "";
  let restored = false;
  const cancel = deferred();
  const composer = installComposer(
    {
      ...hookApi,
      enhance: async (x) => x,
      bindQueue: async (_agent, token, id) => {
        bound = `${token}:${id}`;
      },
      cancelQueue: async (_agent, id) => {
        cancelled = id;
        await cancel.promise;
      },
    },
    { enabled: () => true, settings: () => follow },
    doc,
  );
  field.addEventListener("keydown", () => {
    field.value = "";
    field.parentElement!.append(row);
  });
  composer.onKeydown(key(field, { metaKey: false }).event);
  await wait(10);
  assert.equal(bound, "test:queue-one");
  edit.addEventListener("click", () => {
    restored = true;
  });
  composer.onClick({
    target: edit,
    preventDefault() {},
    stopPropagation() {},
    stopImmediatePropagation() {},
  });
  await settle();
  assert.equal(cancelled, "queue-one");
  assert.equal(restored, false);
  cancel.resolve("");
  await settle();
  assert.equal(restored, true);
  composer.stop();
});

test("new-thread draft sends unchanged without calling agent mode RPCs", async () => {
  const { doc, field, sent } = page("first prompt");
  Object.assign(field, {
    __reactFiber$test: { memoizedProps: { voiceAgentId: "new-workspace" } },
  });
  let modeCalls = 0;
  const composer = installComposer(
    {
      ...hookApi,
      enhance: async (text) => text,
      mode: async () => {
        modeCalls++;
        return "follow-agent";
      },
    },
    { enabled: () => true, settings: () => follow },
    doc,
    fast,
  );
  try {
    composer.onKeydown(key(field, { metaKey: false }).event);
    await settle();
    assert.equal(modeCalls, 2);
    assert.equal(sent.length, 1);
    assert.equal(sent[0].value, "first prompt");
  } finally {
    composer.stop();
  }
});

for (const [mode, original, expected] of [
  ["lite", "hello", "/caveman lite\n\nhello"],
  ["wenyan-ultra", "$caveman off\n\nhello", "$caveman off\n\nhello"],
] as const)
  test(`draft first send: ${mode} respects explicit commands`, async () => {
    const { doc, field, sent } = page(original);
    Object.assign(field, {
      __reactFiber$test: { memoizedProps: { voiceAgentId: "new-workspace" } },
    });
    let snapshots = 0;
    const composer = installComposer(
      {
        ...hookApi,
        enhance: async (t) => t,
        mode: async () => mode,
        prepare: async () => {
          snapshots++;
          return { token: "bad" };
        },
      },
      { enabled: () => true, settings: () => follow },
      doc,
      fast,
    );
    try {
      composer.onKeydown(key(field, { metaKey: false }).event);
      await wait(10);
      assert.equal(sent[0]?.value, expected);
      assert.equal(snapshots, 0);
    } finally {
      composer.stop();
    }
  });

test("unsent first turn replaces its generated command when selection changes", async () => {
  const { doc, field, sent } = page("hello");
  Object.assign(field, { __reactFiber$test: { memoizedProps: { voiceAgentId: "new-workspace" } } });
  let mode: typeof selected = "lite";
  const composer = installComposer(
    { ...hookApi, enhance: async (t) => t, mode: async () => mode },
    { enabled: () => true, settings: () => follow },
    doc,
    fast,
  );
  try {
    composer.onKeydown(key(field, { metaKey: false }).event);
    await wait(10);
    mode = "wenyan-ultra";
    composer.onKeydown(key(field, { metaKey: false }).event);
    await wait(10);
    assert.equal(sent[1]?.value, "/caveman wenyan-ultra\n\nhello");
    mode = "follow-agent";
    composer.onKeydown(key(field, { metaKey: false }).event);
    await wait(10);
    assert.equal(sent[2]?.value, "hello");
  } finally {
    composer.stop();
  }
});

test("new-thread enhancement uses the mode selected while enhancement runs", async () => {
  const { doc, field, sent } = page("hello");
  Object.assign(field, { __reactFiber$test: { memoizedProps: { voiceAgentId: "new-workspace" } } });
  const result = deferred();
  let mode: typeof selected = "lite";
  const composer = installComposer(
    { ...hookApi, enhance: () => result.promise, mode: async () => mode },
    { enabled: () => true, settings: () => follow },
    doc,
    fast,
  );
  try {
    composer.onKeydown(key(field).event);
    mode = "wenyan-ultra";
    result.resolve("enhanced");
    await wait(15);
    assert.equal(sent[0]?.value, "/caveman wenyan-ultra\n\nenhanced");
  } finally {
    composer.stop();
  }
});
