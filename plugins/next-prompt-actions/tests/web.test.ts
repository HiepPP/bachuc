import { test } from "node:test";
import assert from "node:assert/strict";
import { parseHTML } from "linkedom";
import { binding, install, type Node, type Binding } from "../client/web";
import type { Snapshot } from "../shared/contracts";
import { Engine, type Current } from "../server/engine";
import { Store } from "../server/store";
test("v1 radio groups and checkboxes enforce selection, preview exact prompts, send once and clean up", async () => {
  const value = {
    version: 1,
    prompts: [
      { id: "implement", prompt: "Implement the layout.", why: "Apply the design." },
      { id: "review", prompt: "Review only. Do not change code." },
      { id: "risks", prompt: "List remaining risks." },
    ],
    exclusiveGroups: [["implement", "review"]],
    allowedCombinations: [["review", "risks"]],
  };
  const block = JSON.stringify(value);
  const message = "## What Next\n~~~next-prompts\n" + block + "\n~~~";
  const current: Current = {
    epoch: "v1",
    complete: true,
    busy: false,
    rows: [
      { type: "user_message", id: "0", text: "Suggest next steps.", timestamp: 1 },
      { type: "assistant_message", id: "1", text: message, timestamp: 100 },
    ],
  };
  const sent: string[] = [];
  const engine = new Engine(
    new Store(),
    {
      read: async () => structuredClone(current),
      send: async (_scope, text) => {
        sent.push(text);
      },
    },
    async () => false,
  );
  const { document, window } = parseHTML(
    '<html><head></head><body><textarea data-composer-input="">draft</textarea><div data-testid="assistant-message"><div data-paseo-markdown-tag="pre"><span data-paseo-markdown-tag="code"></span><button id="copy">Copy</button></div></div></body></html>',
  );
  document.querySelector('[data-paseo-markdown-tag="code"]')!.textContent = block;
  const previous = Object.getOwnPropertyDescriptor(globalThis, "MutationObserver");
  Object.defineProperty(globalThis, "MutationObserver", {
    value: window.MutationObserver,
    configurable: true,
  });
  const cleanup = install(
    {
      inspect: (scope) => engine.inspect(scope),
      send: async (scope, key) => ({ sent: await engine.send(scope, key) }),
    },
    document as unknown as Parameters<typeof install>[1],
    () => ({ ...context, message }),
  );
  try {
    await pause();
    const radios = document.querySelectorAll('input[type="radio"]');
    const checkbox = document.querySelector('input[type="checkbox"]')!;
    assert.equal(radios.length, 2);
    assert.ok(checkbox);
    assert.equal(radios[0].closest("fieldset"), radios[1].closest("fieldset"));
    assert.notEqual(radios[0].closest("fieldset"), checkbox.closest("fieldset"));
    assert.equal(checkbox.closest("fieldset")!.querySelector("legend")!.textContent, "Follow-up");
    const send = document.querySelector(".npa-selection-send")!;
    assert.equal(send.disabled, true, "no implicit selection");
    const select = (node: typeof checkbox) => {
      node.checked = true;
      node.dispatchEvent(new window.Event("change"));
    };
    select(radios[0]);
    assert.equal(checkbox.disabled, true, "undeclared combination is locked");
    assert.equal(
      checkbox.closest(".npa-choice")!.textContent!.includes("Not with selection"),
      true,
    );
    select(checkbox);
    assert.equal(checkbox.checked, false);
    assert.equal(send.textContent, "Send selected (1)");
    select(radios[1]);
    assert.equal(checkbox.disabled, false);
    select(checkbox);
    assert.equal(radios[0].checked, false, "exclusive choice replaced");
    assert.equal(radios[1].checked, true);
    assert.equal(send.disabled, false);
    document.querySelector(".npa-selection-edit")!.dispatchEvent(new window.Event("click"));
    assert.equal(
      document.querySelector("textarea")!.value,
      "1. Review only. Do not change code.\n2. List remaining risks.",
    );
    send.dispatchEvent(new window.Event("click"));
    send.dispatchEvent(new window.Event("click"));
    await pause();
    assert.deepEqual(sent, ["1. Review only. Do not change code.\n2. List remaining risks."]);
    assert.equal(send.disabled, true);
    assert.equal(document.querySelector('[data-paseo-markdown-tag="code"]')!.textContent, block);
    assert.ok(document.querySelector("#copy"));
  } finally {
    cleanup();
    engine.close();
    assert.equal(document.querySelector("[data-next-prompt-actions]"), null);
    assert.equal(document.querySelector("[data-npa-block]"), null);
    if (previous) Object.defineProperty(globalThis, "MutationObserver", previous);
    else Reflect.deleteProperty(globalThis, "MutationObserver");
  }
});

const context: Binding = {
  agentId: "a",
  serverId: "h",
  workspaceId: "w",
  message: "## Next Steps\n```\nprompt: Test UI.\n```",
  timestamp: 100,
};
const snapshot: Snapshot = {
  enabled: false,
  busy: false,
  note: "",
  candidates: [
    {
      key: "key",
      block: "prompt: Test UI.",
      text: "Test UI.",
      source: context.message,
      timestamp: 100,
      state: "ready",
    },
  ],
};
const pause = () => new Promise((r) => setTimeout(r, 170));

test("DOM button preserves code/copy/draft; sends once and cleans up on disable", async () => {
  const { document, window } = parseHTML(
    '<html><head></head><body><textarea>unsent draft</textarea><div data-testid="assistant-message"><div data-paseo-markdown-tag="pre"><span data-paseo-markdown-tag="code">prompt: Test UI.</span><button id="copy">Copy</button></div></div></body></html>',
  );
  const previous = Object.getOwnPropertyDescriptor(globalThis, "MutationObserver");
  Object.defineProperty(globalThis, "MutationObserver", {
    value: window.MutationObserver,
    configurable: true,
  });
  let sends = 0;
  // Paseo may split one canonical response into a separate fence-only Markdown row.
  let currentContext: Binding | null = { ...context, message: "```\nprompt: Test UI.\n```" };
  const cleanup = install(
    {
      inspect: async () => snapshot,
      send: async (_scope, key) => {
        assert.equal(key, "key");
        sends++;
        return { sent: true };
      },
    },
    document as unknown as Parameters<typeof install>[1],
    () => currentContext,
  );
  try {
    await pause();
    assert.equal(document.querySelectorAll(".npa-send").length, 1);
    assert.equal(document.querySelectorAll('[role="switch"]').length, 0);
    const send = document.querySelector(".npa-send")!;
    send.dispatchEvent(new window.Event("click"));
    send.dispatchEvent(new window.Event("click"));
    await pause();
    assert.equal(sends, 1);
    assert.equal(document.querySelector("textarea")!.textContent, "unsent draft");
    assert.equal(
      document.querySelector('[data-paseo-markdown-tag="code"]')!.textContent,
      "prompt: Test UI.",
    );
    assert.ok(document.querySelector("#copy"));
    currentContext = { ...context, agentId: "other" };
    document.querySelector(".npa-send")!.dispatchEvent(new window.Event("click"));
    assert.equal(sends, 1);
  } finally {
    cleanup();
    assert.equal(document.querySelectorAll("[data-next-prompt-actions]").length, 0);
    assert.equal(document.querySelectorAll("[data-npa-block]").length, 0);
    if (previous) Object.defineProperty(globalThis, "MutationObserver", previous);
    else Reflect.deleteProperty(globalThis, "MutationObserver");
  }
});
test("legacy prompts render individual controls without bulk actions and hide the raw fence", async () => {
  const block = "prompt: First.\nprompt: Second.";
  const many: Snapshot = {
    ...snapshot,
    candidates: ["First.", "Second."].map((text, i) => ({
      ...snapshot.candidates[0],
      key: `key${i}`,
      block,
      text,
      why: i ? undefined : "Pick **this** now.",
    })),
  };
  const { document, window } = parseHTML(
    `<html><head></head><body><div data-testid="assistant-message"><div data-paseo-markdown-tag="pre"><span data-paseo-markdown-tag="code">${block}</span></div></div></body></html>`,
  );
  const previous = Object.getOwnPropertyDescriptor(globalThis, "MutationObserver");
  Object.defineProperty(globalThis, "MutationObserver", {
    value: window.MutationObserver,
    configurable: true,
  });
  const sent: (string | string[])[] = [];
  const cleanup = install(
    {
      inspect: async () => many,
      send: async (_scope, key) => {
        sent.push(key);
        return { sent: true };
      },
    },
    document as unknown as Parameters<typeof install>[1],
    () => context,
  );
  try {
    await pause();
    const labels = document.querySelectorAll(".npa-row .npa-label");
    assert.deepEqual(
      Array.from(labels, (label: Node) => label.textContent),
      ["First.Pick this now.", "Second."],
    );
    assert.equal(document.querySelectorAll(".npa-why").length, 1);
    assert.equal(document.querySelector(".npa-why strong")!.textContent, "this");
    assert.equal(document.querySelectorAll(".npa-row .npa-edit").length, 2);
    assert.equal(document.querySelector(".npa-all"), null);
    const pre = document.querySelector('[data-paseo-markdown-tag="pre"]')!;
    assert.equal(pre.getAttribute("data-npa-block"), "multiple");
    assert.match(
      document.querySelector("style")!.textContent!,
      /\[data-npa-block\] > :not\(\[data-next-prompt-actions\]\) \{display:none!important;\}/,
    );
    document.querySelectorAll(".npa-send")[1].dispatchEvent(new window.Event("click"));
    await pause();
    assert.deepEqual(sent, ["key1"]);
  } finally {
    cleanup();
    if (previous) Object.defineProperty(globalThis, "MutationObserver", previous);
    else Reflect.deleteProperty(globalThis, "MutationObserver");
  }
});
test("sending hook runs on click, before the acknowledgement, with its outcome", async () => {
  const { document, window } = parseHTML(
    '<html><head></head><body><div data-testid="assistant-message"><div data-paseo-markdown-tag="pre"><span data-paseo-markdown-tag="code">prompt: Test UI.</span></div></div></body></html>',
  );
  const previous = Object.getOwnPropertyDescriptor(globalThis, "MutationObserver");
  Object.defineProperty(globalThis, "MutationObserver", {
    value: window.MutationObserver,
    configurable: true,
  });
  let acknowledge!: (value: { sent: boolean }) => void;
  const outcomes: Promise<boolean>[] = [];
  const cleanup = install(
    {
      inspect: async () => snapshot,
      send: () => new Promise((resolve) => (acknowledge = resolve)),
      sending: (outcome) => outcomes.push(outcome),
    },
    document as unknown as Parameters<typeof install>[1],
    () => context,
  );
  try {
    await pause();
    document.querySelector(".npa-send")!.dispatchEvent(new window.Event("click"));
    assert.equal(outcomes.length, 1);
    acknowledge({ sent: false });
    assert.equal(await outcomes[0], false);
  } finally {
    cleanup();
    if (previous) Object.defineProperty(globalThis, "MutationObserver", previous);
    else Reflect.deleteProperty(globalThis, "MutationObserver");
  }
});
test("Commit replaces Send and immediately sends the skill once without changing drafts", async () => {
  const block = "prompt: Run tests.\nwhy: Before commit.\nprompt: Commit the fix. Do not push.";
  const message = `## Next Steps\n\`\`\`\n${block}\n\`\`\``;
  let currentContext: Binding = { ...context, message };
  const current: Current = {
    epoch: "one",
    busy: false,
    complete: true,
    rows: [
      { type: "user_message", text: "Fix the issue.", id: "0", timestamp: 1 },
      { type: "assistant_message", text: message, id: "1", timestamp: 100 },
    ],
  };
  const sent: string[] = [];
  let acknowledge!: () => void;
  const outcomes: Promise<boolean>[] = [];
  const engine = new Engine(
    new Store(),
    {
      read: async () => structuredClone(current),
      async send(scope, text) {
        assert.equal(scope.agentId, context.agentId);
        sent.push(text);
        await new Promise<void>((resolve) => {
          acknowledge = resolve;
        });
      },
    },
    async () => false,
  );
  const { document, window } = parseHTML(
    `<html><head></head><body><textarea data-composer-input="">unsent draft</textarea><div data-testid="assistant-message"><div data-paseo-markdown-tag="pre"><span data-paseo-markdown-tag="code">${block}</span></div></div></body></html>`,
  );
  const previous = Object.getOwnPropertyDescriptor(globalThis, "MutationObserver");
  Object.defineProperty(globalThis, "MutationObserver", {
    value: window.MutationObserver,
    configurable: true,
  });
  const cleanup = install(
    {
      inspect: (scope) => engine.inspect(scope),
      send: async (scope, key) => ({ sent: await engine.send(scope, key) }),
      sending: (outcome) => outcomes.push(outcome),
    },
    document as unknown as Parameters<typeof install>[1],
    () => currentContext,
  );
  try {
    await pause();
    const commit = document.querySelector(".npa-commit")!;
    assert.equal(document.querySelectorAll(".npa-commit").length, 1);
    assert.equal(commit.closest(".npa-row")!.querySelectorAll("button").length, 2);
    assert.equal(commit.textContent, "Commit");
    assert.equal(
      commit.closest(".npa-row")!.querySelector(".npa-label")!.textContent,
      "Commit the fix. Do not push.",
    );
    assert.equal(document.querySelector(".npa-all .npa-commit"), null);
    assert.equal(document.querySelector(".npa-all"), null, "Git actions cannot be batch-sent");
    assert.equal(commit.disabled, false);
    currentContext = { ...currentContext, agentId: "other" };
    commit.dispatchEvent(new window.Event("click"));
    assert.equal(outcomes.length, 0);
    currentContext = { ...context, message };
    commit.dispatchEvent(new window.Event("click"));
    commit.dispatchEvent(new window.Event("click"));
    document.querySelectorAll(".npa-send")[1].dispatchEvent(new window.Event("click"));
    assert.equal(outcomes.length, 1, "Board navigation starts before the send acknowledgement");
    assert.equal(commit.disabled, true);
    await pause();
    assert.deepEqual(sent, [
      "/commit --no-push\nCommit the fix. Do not push.\nCommit only the described changes. Preserve unrelated work.",
    ]);
    assert.equal(document.querySelector("textarea")!.value, "unsent draft");
    acknowledge();
    assert.equal(await outcomes[0], true);
    await pause();
    assert.equal(commit.disabled, true);
    assert.equal(document.querySelectorAll(".npa-send")[1].textContent, "Sent");
    commit.dispatchEvent(new window.Event("click"));
    assert.equal(sent.length, 1);
  } finally {
    cleanup();
    engine.close();
    if (previous) Object.defineProperty(globalThis, "MutationObserver", previous);
    else Reflect.deleteProperty(globalThis, "MutationObserver");
  }
});
test("Git cards show one matching primary action while mentions keep Send", async () => {
  const prompts = [
    "Commit the fix.",
    "Commit and push the fix.",
    "Push the existing commit.",
    "Review commit abc123.",
    "Không commit hoặc push.",
  ];
  const block = prompts.map((text) => `prompt: ${text}`).join("\n");
  const current: Snapshot = {
    ...snapshot,
    candidates: prompts.map((text, i) => ({
      ...snapshot.candidates[0],
      key: `key${i}`,
      block,
      text,
    })),
  };
  const { document, window } = parseHTML(
    `<html><head></head><body><div data-testid="assistant-message"><div data-paseo-markdown-tag="pre"><span data-paseo-markdown-tag="code">${block}</span></div></div></body></html>`,
  );
  const previous = Object.getOwnPropertyDescriptor(globalThis, "MutationObserver");
  Object.defineProperty(globalThis, "MutationObserver", {
    value: window.MutationObserver,
    configurable: true,
  });
  const sent: (string | string[])[] = [];
  const cleanup = install(
    {
      inspect: async () => current,
      send: async (_scope, key) => {
        sent.push(key);
        return { sent: true };
      },
    },
    document as unknown as Parameters<typeof install>[1],
    () => context,
  );
  try {
    await pause();
    const rows = document.querySelectorAll(".npa-row");
    assert.equal(rows.length, prompts.length);
    assert.deepEqual(
      Array.from(rows, (row: Node) =>
        Array.from(row.querySelectorAll("button"), (button: Node) => button.textContent),
      ),
      [
        ["Edit", "Commit"],
        ["Edit", "Commit & Push"],
        ["Edit", "Push"],
        ["Edit", "Send"],
        ["Edit", "Send"],
      ],
    );
    assert.equal(document.querySelectorAll(".npa-send.npa-commit").length, 2);
    assert.equal(document.querySelectorAll(".npa-send.npa-push").length, 1);
    assert.equal(document.querySelector(".npa-all"), null);
    rows[1].querySelector(".npa-send")!.dispatchEvent(new window.Event("click"));
    await pause();
    rows[2].querySelector(".npa-send")!.dispatchEvent(new window.Event("click"));
    await pause();
    assert.deepEqual(sent, ["key1", "key2"]);
  } finally {
    cleanup();
    if (previous) Object.defineProperty(globalThis, "MutationObserver", previous);
    else Reflect.deleteProperty(globalThis, "MutationObserver");
  }
});
test("Edit puts the prompt in the composer, replacing the draft, without sending", async () => {
  // Another conversation stays mounted with its own composer; the host marks the real field
  // with dataSet={{composerInput:""}}.
  const { document, window } = parseHTML(
    '<html><head></head><body><div id="other"><textarea data-composer-input="">other conversation</textarea></div><div id="mine"><div data-testid="assistant-message"><div data-paseo-markdown-tag="pre"><span data-paseo-markdown-tag="code">prompt: Test UI.</span></div></div><div data-testid="message-input-root"><textarea data-composer-input="">half-written draft</textarea></div></div></body></html>',
  );
  const previous = Object.getOwnPropertyDescriptor(globalThis, "MutationObserver");
  Object.defineProperty(globalThis, "MutationObserver", {
    value: window.MutationObserver,
    configurable: true,
  });
  let sends = 0;
  let inputEvents = 0;
  const other = document.querySelector("#other textarea")!;
  const field = document.querySelector("#mine textarea")!;
  field.addEventListener("input", () => inputEvents++);
  const cleanup = install(
    {
      inspect: async () => snapshot,
      send: async () => {
        sends++;
        return { sent: true };
      },
    },
    document as unknown as Parameters<typeof install>[1],
    () => context,
  );
  try {
    await pause();
    const edit = document.querySelector(".npa-edit")!;
    assert.equal(edit.textContent, "Edit");
    edit.dispatchEvent(new window.Event("click"));
    assert.equal(field.value, "Test UI.");
    assert.equal(other.value, "other conversation", "another conversation must stay untouched");
    assert.equal(inputEvents, 1, "React needs the input event to adopt the value");
    assert.equal(sends, 0);
    assert.equal(document.querySelector(".npa-note")!.textContent, "Ready in the composer.");
  } finally {
    cleanup();
    if (previous) Object.defineProperty(globalThis, "MutationObserver", previous);
    else Reflect.deleteProperty(globalThis, "MutationObserver");
  }
});
test("React identity fails closed on streaming or missing scope", () => {
  const props = { message: context.message, timestamp: 100, phase: "complete" };
  const node = {
    __reactFiber$test: {
      memoizedProps: {},
      return: { memoizedProps: props, return: { memoizedProps: context } },
    },
  };
  assert.deepEqual(binding(node as unknown as Node), context);
  props.phase = "streaming";
  assert.equal(binding(node as unknown as Node), null);
  assert.equal(binding({} as Node), null);
});

test("a new turn's unsent block renders no note from the previous send", async () => {
  const scope = { serverId: "h", workspaceId: "w", agentId: "a" };
  const next = "## Next Steps\n```\nprompt: Report the next results.\n```";
  const current: Current = {
    epoch: "one",
    busy: false,
    complete: true,
    rows: [
      { type: "user_message", text: "Report test results.", id: "0", timestamp: 1 },
      {
        type: "assistant_message",
        text: "## Next Steps\n```\nprompt: Report results.\n```",
        id: "1",
        timestamp: 2,
      },
    ],
  };
  let engine!: Engine;
  const driver = {
    async read() {
      return structuredClone(current);
    },
    // The daemon starts the turn before acknowledging the send.
    async send() {
      engine.started(scope.agentId);
    },
  };
  engine = new Engine(new Store(), driver, async () => true);
  const key = (await engine.inspect(scope)).candidates[0].key;
  await engine.send(scope, key);
  current.rows.push(
    { type: "user_message", text: "Report results.", id: "2", timestamp: 3 },
    { type: "assistant_message", text: next, id: "3", timestamp: 4 },
  );

  const { document, window } = parseHTML(
    '<html><head></head><body><div data-testid="assistant-message"><div data-paseo-markdown-tag="pre"><span data-paseo-markdown-tag="code">prompt: Report the next results.</span></div></div></body></html>',
  );
  const previous = Object.getOwnPropertyDescriptor(globalThis, "MutationObserver");
  Object.defineProperty(globalThis, "MutationObserver", {
    value: window.MutationObserver,
    configurable: true,
  });
  const cleanup = install(
    {
      inspect: (input) => engine.inspect(input),
      send: async (input, sendKey) => {
        return { sent: await engine.send(input, sendKey) };
      },
    },
    document as unknown as Parameters<typeof install>[1],
    () => ({ ...scope, message: next, timestamp: 4 }),
  );
  try {
    await pause();
    assert.equal(document.querySelectorAll(".npa-send").length, 1);
    assert.equal(document.querySelector(".npa-send")!.textContent, "Send");
    assert.equal(document.querySelector(".npa-note")!.textContent, "");
  } finally {
    cleanup();
    if (previous) Object.defineProperty(globalThis, "MutationObserver", previous);
    else Reflect.deleteProperty(globalThis, "MutationObserver");
  }
});

test("new completed prompt renders promptly after a previous scan", async () => {
  const { document, window } = parseHTML(
    '<html><head></head><body><div data-testid="assistant-message"></div></body></html>',
  );
  const previous = Object.getOwnPropertyDescriptor(globalThis, "MutationObserver");
  Object.defineProperty(globalThis, "MutationObserver", {
    value: window.MutationObserver,
    configurable: true,
  });
  let completed = false;
  const cleanup = install(
    { inspect: async () => snapshot, send: async () => ({ sent: true }) },
    document as unknown as Parameters<typeof install>[1],
    () => (completed ? context : null),
  );
  try {
    await pause();
    assert.equal(document.querySelectorAll(".npa-send").length, 0);
    completed = true;
    document.querySelector('[data-testid="assistant-message"]')!.innerHTML =
      '<div data-paseo-markdown-tag="pre"><span data-paseo-markdown-tag="code">prompt: Test UI.</span></div>';
    await new Promise((resolve) => setTimeout(resolve, 500));
    assert.equal(
      document.querySelectorAll(".npa-send").length,
      1,
      "new prompts must not wait for the two-second scan cooldown",
    );
  } finally {
    cleanup();
    if (previous) Object.defineProperty(globalThis, "MutationObserver", previous);
    else Reflect.deleteProperty(globalThis, "MutationObserver");
  }
});

test("Send and Edit update controls in place without extra timeline reads", async () => {
  const { document, window } = parseHTML(
    '<html><head></head><body><div data-testid="assistant-message"><div data-paseo-markdown-tag="pre"><span data-paseo-markdown-tag="code">prompt: Test UI.</span></div></div></body></html>',
  );
  const previous = Object.getOwnPropertyDescriptor(globalThis, "MutationObserver");
  Object.defineProperty(globalThis, "MutationObserver", {
    value: window.MutationObserver,
    configurable: true,
  });
  // No composer: linkedom, unlike browsers, mutates the DOM on a textarea value write, and the
  // Edit note alone must be the only change here.
  let reads = 0;
  let current = snapshot;
  const cleanup = install(
    {
      inspect: async () => {
        reads++;
        return current;
      },
      send: async () => {
        current = { ...snapshot, candidates: [{ ...snapshot.candidates[0], state: "sent" }] };
        return { sent: true };
      },
    },
    document as unknown as Parameters<typeof install>[1],
    () => context,
  );
  try {
    // linkedom also reports the block's attribute change, which browsers skip when attributes
    // are not observed; let that one extra scan settle first.
    await pause();
    await pause();
    const ui = document.querySelector("[data-next-prompt-actions]");
    const before = reads;
    document.querySelector(".npa-edit")!.dispatchEvent(new window.Event("click"));
    await pause();
    assert.equal(reads, before, "the plugin's own note must not trigger a timeline read");
    const send = document.querySelector(".npa-send")!;
    send.dispatchEvent(new window.Event("click"));
    assert.equal(send.textContent, "Sending...", "Send gives feedback immediately");
    await pause();
    assert.equal(document.querySelector("[data-next-prompt-actions]"), ui, "no rebuild flash");
    assert.equal(send.textContent, "Sent");
    assert.equal(send.disabled, true);
  } finally {
    cleanup();
    if (previous) Object.defineProperty(globalThis, "MutationObserver", previous);
    else Reflect.deleteProperty(globalThis, "MutationObserver");
  }
});

test("streaming DOM changes reuse the last timeline read until a prompt block changes", async () => {
  const { document, window } = parseHTML(
    '<html><head></head><body><div data-testid="assistant-message"><div data-paseo-markdown-tag="pre"><span data-paseo-markdown-tag="code">prompt: Test UI.</span></div></div><div id="stream"></div></body></html>',
  );
  const previous = Object.getOwnPropertyDescriptor(globalThis, "MutationObserver");
  Object.defineProperty(globalThis, "MutationObserver", {
    value: window.MutationObserver,
    configurable: true,
  });
  let reads = 0;
  const cleanup = install(
    {
      inspect: async () => {
        reads++;
        return snapshot;
      },
      send: async () => ({ sent: true }),
    },
    document as unknown as Parameters<typeof install>[1],
    () => context,
  );
  try {
    await pause();
    await pause();
    const before = reads;
    const stream = document.querySelector("#stream")!;
    for (let i = 0; i < 5; i++) {
      stream.appendChild(document.createTextNode(`token ${i} `));
      await new Promise((resolve) => setTimeout(resolve, 60));
    }
    await pause();
    assert.equal(reads, before, "streaming tokens must not refetch the timeline");
    assert.equal(document.querySelectorAll(".npa-send").length, 1);
    stream.innerHTML =
      '<div data-testid="assistant-message"><div data-paseo-markdown-tag="pre"><span data-paseo-markdown-tag="code">prompt: Next step.</span></div></div>';
    await pause();
    assert.equal(reads, before + 1, "a new prompt block reads the timeline once");
  } finally {
    cleanup();
    if (previous) Object.defineProperty(globalThis, "MutationObserver", previous);
    else Reflect.deleteProperty(globalThis, "MutationObserver");
  }
});

test("one structured suggestion uses direct Edit and Send without a selection step", async () => {
  const block = JSON.stringify({
    version: 1,
    prompts: [{ id: "verify", prompt: "Verify the layout." }],
    exclusiveGroups: [],
    allowedCombinations: [],
  });
  const message = `## What Next\n\`\`\`next-prompts\n${block}\n\`\`\``;
  const candidate = {
    ...snapshot.candidates[0],
    block,
    source: message,
    text: "Verify the layout.",
    selection: { blockKey: "single", id: "verify", exclusiveGroups: [], allowedCombinations: [] },
  };
  const { document, window } = parseHTML(
    '<html><head></head><body><textarea data-composer-input="">draft</textarea><div data-testid="assistant-message"><div data-paseo-markdown-tag="h2">What Next</div><div data-paseo-markdown-tag="pre"><span data-paseo-markdown-tag="code"></span></div></div></body></html>',
  );
  document.querySelector('[data-paseo-markdown-tag="code"]')!.textContent = block;
  const previous = Object.getOwnPropertyDescriptor(globalThis, "MutationObserver");
  Object.defineProperty(globalThis, "MutationObserver", {
    value: window.MutationObserver,
    configurable: true,
  });
  const sent: (string | string[])[] = [];
  const cleanup = install(
    {
      inspect: async () => ({ ...snapshot, candidates: [candidate] }),
      send: async (_, key) => {
        sent.push(key);
        return { sent: true };
      },
    },
    document as unknown as Parameters<typeof install>[1],
    () => ({ ...context, message }),
  );
  try {
    await pause();
    assert.equal(document.querySelectorAll("input").length, 0);
    assert.equal(document.querySelectorAll(".npa-edit").length, 1);
    assert.equal(document.querySelectorAll(".npa-send").length, 1);
    document.querySelector(".npa-edit")!.dispatchEvent(new window.Event("click"));
    assert.equal(document.querySelector("textarea")!.value, candidate.text);
    document.querySelector(".npa-send")!.dispatchEvent(new window.Event("click"));
    await pause();
    assert.deepEqual(sent, [candidate.key]);
    assert.ok(document.querySelector("[data-npa-next-heading]"));
  } finally {
    cleanup();
    assert.equal(document.querySelector("[data-npa-next-heading]"), null);
    if (previous) Object.defineProperty(globalThis, "MutationObserver", previous);
    else Reflect.deleteProperty(globalThis, "MutationObserver");
  }
});

test("Recap, What Next, and intro fold into one prompt panel and restore exactly", async () => {
  const block = JSON.stringify({
    version: 1,
    prompts: [{ id: "verify", prompt: "Verify the layout." }],
    exclusiveGroups: [],
    allowedCombinations: [],
  });
  const message = `## Recap\n- Branch: \`main\`\n- Did: Read the [report](/report).\n- Commit/push: none\n\n## What Next\nCheck the panel.\n\`\`\`next-prompts\n${block}\n\`\`\``;
  const item = (value: string) =>
    `<div data-paseo-markdown-tag="li"><span data-paseo-markdown-ignore="true" data-paseo-markdown-list-marker="true">•</span><div><span>${value}</span></div></div>`;
  const candidate = {
    ...snapshot.candidates[0],
    block,
    source: message,
    text: "Verify the layout.",
    selection: { blockKey: "panel", id: "verify", exclusiveGroups: [], allowedCombinations: [] },
  };
  const { document, window } = parseHTML(
    `<html><head></head><body><textarea data-composer-input="">draft</textarea><div data-testid="assistant-message"><div data-paseo-markdown-tag="h2"><span>Recap</span></div><div data-paseo-markdown-tag="ul">${item('Branch: <span data-paseo-markdown-tag="code">main</span>')}${item('Did: Read the <a href="/report">report</a>.')}${item("Commit/push: none")}</div><div data-paseo-markdown-tag="h2"><span>What Next</span></div><div data-paseo-markdown-tag="p"><span>Check the panel.</span></div><div data-paseo-markdown-tag="pre"><span data-paseo-markdown-tag="code"></span></div></div></body></html>`,
  );
  document.querySelector(
    '[data-paseo-markdown-tag="pre"] [data-paseo-markdown-tag="code"]',
  )!.textContent = block;
  const assistant = document.querySelector('[data-testid="assistant-message"]')!;
  const before = assistant.innerHTML;
  const previous = Object.getOwnPropertyDescriptor(globalThis, "MutationObserver");
  Object.defineProperty(globalThis, "MutationObserver", {
    value: window.MutationObserver,
    configurable: true,
  });
  const sent: (string | string[])[] = [];
  const cleanup = install(
    {
      inspect: async () => ({ ...snapshot, candidates: [candidate] }),
      send: async (_, key) => {
        sent.push(key);
        return { sent: true };
      },
    },
    document as unknown as Parameters<typeof install>[1],
    () => ({ ...context, message }),
  );
  try {
    await pause();
    const panel = document.querySelector("[data-next-prompt-actions]")!;
    const recap = panel.querySelector(".npa-recap")!;
    assert.equal(recap.querySelector(".npa-kicker")!.textContent, "Recap");
    assert.equal(recap.querySelector(".npa-branch")!.textContent!.trim(), "main");
    const commit = recap.querySelector(".npa-commit")!;
    assert.equal(commit.textContent!.trim(), "No commit");
    assert.equal(commit.getAttribute("aria-label"), "Commit/push: none");
    assert.equal(recap.querySelector(".npa-did")!.textContent!.trim(), "Read the report.");
    assert.equal(
      recap.nextElementSibling!.querySelector(".npa-next-title")!.textContent,
      "What Next",
    );
    assert.equal(panel.querySelector("a")!.getAttribute("href"), "/report");
    assert.equal(panel.querySelector(".npa-next-title")!.textContent, "What Next");
    assert.equal(panel.querySelector(".npa-next-intro")!.textContent!.trim(), "Check the panel.");
    assert.equal(panel.querySelectorAll("[data-paseo-markdown-tag]").length, 0);
    assert.equal(document.querySelectorAll("[data-npa-folded]").length, 4);
    document.querySelector(".npa-send")!.dispatchEvent(new window.Event("click"));
    await pause();
    assert.deepEqual(sent, [candidate.key]);
  } finally {
    cleanup();
    assert.equal(assistant.innerHTML, before);
    if (previous) Object.defineProperty(globalThis, "MutationObserver", previous);
    else Reflect.deleteProperty(globalThis, "MutationObserver");
  }
});

test("Paseo history rows of one reply fold into the prompt panel and restore exactly", async () => {
  const block = JSON.stringify({
    version: 1,
    prompts: [{ id: "verify", prompt: "Verify the layout." }],
    exclusiveGroups: [],
    allowedCombinations: [],
  });
  const message = `Intro.\n\n## Recap\n- Branch: \`main\`\n- Did: Read it.\n- Commit/push: pushed main\n\n## What Next\nCheck the panel.\n\`\`\`next-prompts\n${block}\n\`\`\``;
  const item = (value: string) =>
    `<div data-paseo-markdown-tag="li"><div data-paseo-markdown-ignore="true" data-paseo-markdown-list-marker="true">•</div><div>${value}</div></div>`;
  const row = (index: number, content: string) =>
    `<div data-history-row-id="m1:block:${index}" data-message-id="m1"><div data-message-text="true" data-testid="assistant-message">${content}</div></div>`;
  const candidate = {
    ...snapshot.candidates[0],
    block,
    source: message,
    text: "Verify the layout.",
    selection: { blockKey: "rows", id: "verify", exclusiveGroups: [], allowedCombinations: [] },
  };
  const { document, window } = parseHTML(
    `<html><head></head><body><div id="list">${row(0, '<div data-paseo-markdown-tag="p"><div>Intro.</div></div>')}${row(1, `<div data-paseo-markdown-tag="h2"><div>Recap</div></div><div data-paseo-markdown-tag="ul">${item('Branch: <span data-paseo-markdown-tag="code">main</span>')}${item("Did: Read it.")}${item("Commit/push: pushed main")}</div>`)}${row(2, '<div data-paseo-markdown-tag="h2"><div>What Next</div></div><div data-paseo-markdown-tag="p"><div>Check the panel.</div></div>')}${row(3, '<div data-paseo-markdown-tag="pre"><div data-paseo-markdown-tag="code"></div></div>')}</div></body></html>`,
  );
  document.querySelector(
    '[data-paseo-markdown-tag="pre"] [data-paseo-markdown-tag="code"]',
  )!.textContent = block;
  const list = document.querySelector("#list")!;
  const before = list.innerHTML;
  const previous = Object.getOwnPropertyDescriptor(globalThis, "MutationObserver");
  Object.defineProperty(globalThis, "MutationObserver", {
    value: window.MutationObserver,
    configurable: true,
  });
  const cleanup = install(
    {
      inspect: async () => ({ ...snapshot, candidates: [candidate] }),
      send: async () => ({ sent: true }),
    },
    document as unknown as Parameters<typeof install>[1],
    () => ({ ...context, message }),
  );
  try {
    await pause();
    const panel = document.querySelector("[data-next-prompt-actions]")!;
    assert.equal(panel.querySelector(".npa-branch")!.textContent!.trim(), "main");
    const commit = panel.querySelector(".npa-commit.npa-ok")!;
    assert.equal(commit.textContent!.trim(), "pushed main");
    assert.equal(commit.getAttribute("aria-label"), "Commit/push: pushed main");
    assert.equal(panel.querySelector(".npa-next-title")!.textContent, "What Next");
    assert.equal(panel.querySelector(".npa-next-intro")!.textContent!.trim(), "Check the panel.");
    const folded = (index: number) =>
      document
        .querySelector(`[data-history-row-id="m1:block:${index}"]`)!
        .getAttribute("data-npa-folded");
    assert.deepEqual([0, 1, 2, 3].map(folded), [null, "true", "true", null]);
  } finally {
    cleanup();
    assert.equal(list.innerHTML, before);
    if (previous) Object.defineProperty(globalThis, "MutationObserver", previous);
    else Reflect.deleteProperty(globalThis, "MutationObserver");
  }
});

test("earlier replies keep a read-only panel without actions after a newer turn", async () => {
  const code = "prompt: Verify the layout.\nwhy: Pick this to **confirm it**, because it changed.";
  const row = (id: string, index: number, content: string) =>
    `<div data-history-row-id="${id}:block:${index}" data-message-id="${id}"><div data-message-text="true" data-testid="assistant-message">${content}</div></div>`;
  const pre = `<div data-paseo-markdown-tag="pre"><div data-paseo-markdown-tag="code">${code}</div></div>`;
  const { document, window } = parseHTML(
    `<html><head></head><body><div id="list">${row("old", 0, '<div data-paseo-markdown-tag="h2"><div>What Next</div></div>')}${row("old", 1, pre)}${row("loose", 0, '<div data-paseo-markdown-tag="h2"><div>Notes</div></div>')}${row("loose", 1, pre)}</div></body></html>`,
  );
  const list = document.querySelector("#list")!;
  const before = list.innerHTML;
  const previous = Object.getOwnPropertyDescriptor(globalThis, "MutationObserver");
  Object.defineProperty(globalThis, "MutationObserver", {
    value: window.MutationObserver,
    configurable: true,
  });
  const cleanup = install(
    { inspect: async () => ({ ...snapshot, candidates: [] }), send: async () => ({ sent: true }) },
    document as unknown as Parameters<typeof install>[1],
    () => ({ ...context, message: "older reply" }),
  );
  try {
    await pause();
    const panels = document.querySelectorAll("[data-next-prompt-actions]");
    assert.equal(panels.length, 1);
    const panel = panels[0];
    assert.ok(panel.closest('[data-message-id="old"]'));
    assert.equal(panel.getAttribute("data-npa-readonly"), "true");
    assert.equal(panel.querySelector(".npa-next-title")!.textContent, "What Next");
    assert.equal(panel.querySelector(".npa-label")!.firstChild!.textContent, "Verify the layout.");
    assert.equal(panel.querySelector(".npa-why strong")!.textContent, "confirm it");
    assert.equal(panel.querySelectorAll("button, input").length, 0);
  } finally {
    cleanup();
    assert.equal(list.innerHTML, before);
    if (previous) Object.defineProperty(globalThis, "MutationObserver", previous);
    else Reflect.deleteProperty(globalThis, "MutationObserver");
  }
});

test("rows mounted by the virtualized history fold before the timeline read returns", async () => {
  const code = "prompt: Continue the review.";
  const row = (index: number, content: string) =>
    `<div data-history-row-id="v:block:${index}" data-message-id="v"><div data-message-text="true" data-testid="assistant-message">${content}</div></div>`;
  const item = (value: string) =>
    `<div data-paseo-markdown-tag="li"><div data-paseo-markdown-ignore="true" data-paseo-markdown-list-marker="true">•</div><div>${value}</div></div>`;
  const { document, window } = parseHTML(
    '<html><head></head><body><div id="list"></div></body></html>',
  );
  const previous = Object.getOwnPropertyDescriptor(globalThis, "MutationObserver");
  Object.defineProperty(globalThis, "MutationObserver", {
    value: window.MutationObserver,
    configurable: true,
  });
  const cleanup = install(
    // The read never settles: only the synchronous mount path can produce the panel.
    { inspect: () => new Promise(() => {}), send: async () => ({ sent: true }) },
    document as unknown as Parameters<typeof install>[1],
    () => ({ ...context, message: "older reply" }),
  );
  try {
    const list = document.querySelector("#list")!;
    list.insertAdjacentHTML(
      "beforeend",
      row(1, '<div data-paseo-markdown-tag="h2"><div>What Next</div></div>') +
        row(
          2,
          `<div data-paseo-markdown-tag="pre"><div data-paseo-markdown-tag="code">${code}</div></div>`,
        ),
    );
    await Promise.resolve();
    await Promise.resolve();
    const panel = list.querySelector("[data-next-prompt-actions]");
    assert.ok(panel, "panel renders in the mutation callback");
    assert.equal(panel!.querySelector(".npa-recap"), null);
    list.insertAdjacentHTML(
      "afterbegin",
      row(
        0,
        `<div data-paseo-markdown-tag="h2"><div>Recap</div></div><div data-paseo-markdown-tag="ul">${item("Branch: main")}${item("Did: Read it.")}${item("Commit/push: none")}</div>`,
      ),
    );
    await Promise.resolve();
    await Promise.resolve();
    assert.ok(list.querySelector("[data-next-prompt-actions] .npa-recap"), "late rows fold in");
    assert.equal(
      list.querySelector('[data-history-row-id="v:block:0"]')!.getAttribute("data-npa-folded"),
      "true",
    );
  } finally {
    cleanup();
    if (previous) Object.defineProperty(globalThis, "MutationObserver", previous);
    else Reflect.deleteProperty(globalThis, "MutationObserver");
  }
});

test("a timeline read that started before rows mounted does not clear their panels", async () => {
  const row = (id: string, index: number, content: string) =>
    `<div data-history-row-id="${id}:block:${index}" data-message-id="${id}"><div data-message-text="true" data-testid="assistant-message">${content}</div></div>`;
  const reply = (id: string) =>
    row(id, 0, '<div data-paseo-markdown-tag="h2"><div>What Next</div></div>') +
    row(
      id,
      1,
      `<div data-paseo-markdown-tag="pre"><div data-paseo-markdown-tag="code">prompt: Continue ${id}.</div></div>`,
    );
  const { document, window } = parseHTML(
    `<html><head></head><body><div id="list">${reply("a")}</div></body></html>`,
  );
  const previous = Object.getOwnPropertyDescriptor(globalThis, "MutationObserver");
  Object.defineProperty(globalThis, "MutationObserver", {
    value: window.MutationObserver,
    configurable: true,
  });
  let settle: (value: Snapshot) => void = () => {};
  const cleanup = install(
    {
      inspect: () => new Promise<Snapshot>((resolve) => (settle = resolve)),
      send: async () => ({ sent: true }),
    },
    document as unknown as Parameters<typeof install>[1],
    () => ({ ...context, message: "older reply" }),
  );
  try {
    await pause(); // the scan now waits on the read
    document.querySelector("#list")!.insertAdjacentHTML("beforeend", reply("b"));
    await Promise.resolve();
    const panelB = () => document.querySelector('[data-message-id="b"] [data-next-prompt-actions]');
    assert.ok(panelB(), "mount path renders b");
    settle({ ...snapshot, candidates: [] });
    await pause();
    assert.ok(panelB(), "the earlier read keeps b's panel");
    assert.equal(document.querySelectorAll("[data-next-prompt-actions]").length, 2);
  } finally {
    cleanup();
    if (previous) Object.defineProperty(globalThis, "MutationObserver", previous);
    else Reflect.deleteProperty(globalThis, "MutationObserver");
  }
});

test("Recap Did with nested bullets keeps them as a list in the panel", async () => {
  const block = JSON.stringify({
    version: 1,
    prompts: [{ id: "verify", prompt: "Verify the layout." }],
    exclusiveGroups: [],
    allowedCombinations: [],
  });
  const message = `## Recap\n- Branch: main\n- Did:\n  - Added config.\n  - Fixed bridge.\n- Commit/push: none\n\n## What Next\n\`\`\`next-prompts\n${block}\n\`\`\``;
  const item = (value: string) =>
    `<div data-paseo-markdown-tag="li"><span data-paseo-markdown-ignore="true" data-paseo-markdown-list-marker="true">•</span><div><span>${value}</span></div></div>`;
  const nested = `<div data-paseo-markdown-tag="ul">${item("Added config.")}${item("Fixed bridge.")}</div>`;
  const candidate = {
    ...snapshot.candidates[0],
    block,
    source: message,
    text: "Verify the layout.",
    selection: { blockKey: "nested", id: "verify", exclusiveGroups: [], allowedCombinations: [] },
  };
  const { document, window } = parseHTML(
    `<html><head></head><body><textarea data-composer-input=""></textarea><div data-testid="assistant-message"><div data-paseo-markdown-tag="h2"><span>Recap</span></div><div data-paseo-markdown-tag="ul">${item("Branch: main")}${item(`Did:${nested}`)}${item("Commit/push: none")}</div><div data-paseo-markdown-tag="h2"><span>What Next</span></div><div data-paseo-markdown-tag="pre"><span data-paseo-markdown-tag="code"></span></div></div></body></html>`,
  );
  document.querySelector(
    '[data-paseo-markdown-tag="pre"] [data-paseo-markdown-tag="code"]',
  )!.textContent = block;
  const assistant = document.querySelector('[data-testid="assistant-message"]')!;
  const before = assistant.innerHTML;
  const previous = Object.getOwnPropertyDescriptor(globalThis, "MutationObserver");
  Object.defineProperty(globalThis, "MutationObserver", {
    value: window.MutationObserver,
    configurable: true,
  });
  const cleanup = install(
    {
      inspect: async () => ({ ...snapshot, candidates: [candidate] }),
      send: async () => ({ sent: true }),
    },
    document as unknown as Parameters<typeof install>[1],
    () => ({ ...context, message }),
  );
  try {
    await pause();
    const did = document.querySelector("[data-next-prompt-actions] .npa-did")!;
    assert.equal(did.tagName, "DIV");
    const bullets = Array.from(did.querySelectorAll('[data-npa-list="li"]'));
    assert.deepEqual(
      bullets.map((bullet) => (bullet as Node).textContent!.replace("•", "").trim()),
      ["Added config.", "Fixed bridge."],
    );
    assert.equal(did.querySelectorAll("[data-paseo-markdown-list-marker]").length, 2);
    assert.equal(document.querySelectorAll("[data-npa-folded]").length, 3);
  } finally {
    cleanup();
    assert.equal(assistant.innerHTML, before);
    if (previous) Object.defineProperty(globalThis, "MutationObserver", previous);
    else Reflect.deleteProperty(globalThis, "MutationObserver");
  }
});
