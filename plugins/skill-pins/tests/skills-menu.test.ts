import { test } from "node:test";
import assert from "node:assert/strict";
import { parseHTML } from "linkedom";
import type { PluginClientContext } from "@getpaseo/plugin/client";
import { installSkillsMenu, pillLabel, pillTitle } from "../client/skills-menu";
import { createSkillState } from "../client/state";
import { composerHost, type Doc, type El } from "../client/dom";
import type { SkillId } from "../shared/catalog";

const AGENT = "00000000-0000-4000-8000-000000000011";
const settle = () => new Promise((resolve) => setTimeout(resolve, 0));

function page(props: Record<string, unknown>, caveman = false) {
  const { document } = parseHTML(
    `<html><head></head><body><div data-testid="message-input-root"><textarea></textarea><div>${caveman ? '<div data-prompt-translate-mode=""></div>' : ""}<div><button data-testid="combined-model-selector">Model</button></div></div></div></body></html>`,
  );
  Object.assign(document.querySelector("textarea")!, {
    __reactFiber$test: { memoizedProps: props },
  });
  return document;
}

function fakeClient(initial: SkillId[] = []) {
  let saved = initial;
  const writes: SkillId[][] = [];
  const client = {
    rpc(contract: { name: string }, input: { skills?: SkillId[] }) {
      if (contract.name === "skill-pins.write") {
        saved = input.skills!;
        writes.push(saved);
      }
      return Promise.resolve({ skills: saved });
    },
  } as unknown as PluginClientContext;
  return { client, writes };
}

test("pill label and title name the pinned skills in catalog order", () => {
  assert.equal(pillLabel([]), "Skills");
  assert.equal(pillLabel(undefined), "Skills");
  assert.equal(pillLabel(["watchtower"]), "Watch");
  assert.equal(pillLabel(["sequential-thinking", "watchtower"]), "Watch · Seq");
  assert.equal(
    pillLabel(["watchtower", "chase-goal-claude", "sequential-thinking"]),
    "Watch · Goal · Seq",
  );
  assert.equal(pillTitle([]), "Pin skills for this chat");
  assert.equal(
    pillTitle(["sequential-thinking", "watchtower"]),
    "Pinned: Watchtower, Sequential thinking",
  );
});

test("toggling saves the full set and keeps the menu open", async () => {
  const document = page({ voiceAgentId: AGENT, voiceServerId: "host-a" });
  const { client, writes } = fakeClient(["watchtower"]);
  const state = createSkillState(client);
  const menu = installSkillsMenu(document as unknown as Doc, undefined, state);
  try {
    await settle();
    menu.update();
    const trigger = document.querySelector("[data-sp-trigger]")!;
    assert.equal(trigger.textContent, "Watch");
    assert.equal(trigger.getAttribute("title"), "Pinned: Watchtower");
    assert.equal(trigger.getAttribute("aria-label"), "Pinned: Watchtower");
    assert.equal(
      document.querySelector("[data-skill-pins]")!.getAttribute("data-sp-state"),
      "active",
    );
    assert.equal(trigger.hasAttribute("disabled"), false);
    trigger.click();
    const option = (id: string) => document.querySelector(`[data-sp-option="${id}"]`)!;
    assert.equal(option("watchtower").getAttribute("aria-selected"), "true");
    option("sequential-thinking").click();
    await settle();
    assert.deepEqual(writes.at(-1), ["watchtower", "sequential-thinking"]);
    assert.ok(document.querySelector("[data-sp-menu]"), "menu stays open");
    assert.equal(option("sequential-thinking").getAttribute("aria-selected"), "true");
    assert.equal(trigger.textContent, "Watch · Seq");
    assert.equal(trigger.getAttribute("title"), "Pinned: Watchtower, Sequential thinking");
    option("watchtower").click();
    await settle();
    assert.deepEqual(writes.at(-1), ["sequential-thinking"]);
    menu.stop();
    assert.equal(document.querySelector("[data-skill-pins]"), null);
  } finally {
    menu.stop();
  }
});

test("new-thread composer pins through the host draft", async () => {
  const document = page({ voiceAgentId: "new-workspace" });
  const calls: string[] = [];
  let draft: SkillId[] = ["watchtower"];
  const client = {
    rpc(contract: { name: string }, input: { skills?: SkillId[] }) {
      calls.push(contract.name);
      if (contract.name === "skill-pins.draft-write") draft = input.skills!;
      return Promise.resolve({ skills: draft });
    },
  } as unknown as PluginClientContext;
  const menu = installSkillsMenu(document as unknown as Doc, undefined, createSkillState(client));
  try {
    await settle();
    menu.update();
    const trigger = document.querySelector("[data-sp-trigger]")!;
    assert.equal(trigger.hasAttribute("disabled"), false);
    assert.equal(trigger.textContent, "Watch");
    trigger.click();
    document.querySelector('[data-sp-option="sequential-thinking"]')!.click();
    await settle();
    assert.deepEqual(draft, ["watchtower", "sequential-thinking"]);
    assert.deepEqual(calls, ["skill-pins.draft-read", "skill-pins.draft-write"]);
    // The draft is reread when a new-thread composer appears again.
    const root = document.querySelector('[data-testid="message-input-root"]')!;
    root.remove();
    menu.scan();
    draft = [];
    document.body.append(root);
    menu.scan();
    await settle();
    menu.update();
    assert.equal(calls.filter((name) => name === "skill-pins.draft-read").length, 2);
    assert.equal(document.querySelector("[data-sp-trigger]")!.textContent, "Skills");
  } finally {
    menu.stop();
  }
});

test("a composer owned by another host gets no pill", async () => {
  const document = page({ voiceAgentId: AGENT, voiceServerId: "host-b" });
  const { client } = fakeClient();
  const owns = (root: El) => composerHost(root) === "host-a";
  const menu = installSkillsMenu(
    document as unknown as Doc,
    undefined,
    createSkillState(client),
    owns,
  );
  try {
    assert.equal(document.querySelector("[data-skill-pins]"), null);
  } finally {
    menu.stop();
  }
});

test("the pill sits before the Caveman pill and keeps that order on rescan", async () => {
  const document = page({ voiceAgentId: AGENT }, true);
  const { client } = fakeClient();
  const menu = installSkillsMenu(document as unknown as Doc, undefined, createSkillState(client));
  try {
    const pill = document.querySelector("[data-skill-pins]")!;
    const caveman = document.querySelector("[data-prompt-translate-mode]")!;
    assert.equal(pill.nextElementSibling, caveman);
    menu.scan();
    assert.equal(pill.nextElementSibling, caveman);
    assert.equal(
      caveman.nextElementSibling?.querySelector("[data-testid=combined-model-selector]") !== null,
      true,
    );
  } finally {
    menu.stop();
  }
});

test("Escape closes the menu", async () => {
  const { window } = parseHTML("<html></html>");
  const document = page({ voiceAgentId: AGENT });
  const { client } = fakeClient();
  const state = createSkillState(client);
  await state.load(AGENT);
  const menu = installSkillsMenu(document as unknown as Doc, undefined, state);
  try {
    menu.update();
    document.querySelector<HTMLElement>("[data-sp-trigger]")!.click();
    assert.ok(document.querySelector("[data-sp-menu]"));
    const wrapper = document.querySelector("[data-skill-pins]")!;
    const escape = new window.Event("keydown", { bubbles: true });
    Object.defineProperty(escape, "key", { value: "Escape" });
    wrapper.dispatchEvent(escape);
    assert.equal(document.querySelector("[data-sp-menu]"), null);
  } finally {
    menu.stop();
  }
});

test("the pill copies the model selector's text style and marks picks with a check", async () => {
  const document = page({ voiceAgentId: AGENT });
  const model = document.querySelector('[data-testid="combined-model-selector"]')!;
  model.innerHTML = "<span>Opus</span>";
  const view = document.defaultView as unknown as Record<string, unknown>;
  view.getComputedStyle = (node: El) =>
    node === model.querySelector("span")
      ? {
          color: "rgb(1, 2, 3)",
          backgroundColor: "transparent",
          fontFamily: "Inter",
          fontSize: "13px",
          fontWeight: "500",
          fontStyle: "normal",
          lineHeight: "18px",
          letterSpacing: "0px",
        }
      : { color: "rgb(0, 0, 0)", backgroundColor: "rgb(255, 255, 255)" };
  const { client } = fakeClient(["chase-goal-claude"]);
  const state = createSkillState(client);
  await state.load(AGENT);
  const menu = installSkillsMenu(document as unknown as Doc, undefined, state);
  try {
    menu.update();
    const trigger = document.querySelector("[data-sp-trigger]")!;
    assert.equal(
      trigger.getAttribute("data-sp-typography"),
      "color:rgb(1, 2, 3);font-family:Inter;font-size:13px;font-weight:500;font-style:normal;line-height:18px;letter-spacing:0px;",
    );
    trigger.click();
    const style = (document.querySelector("[data-sp-menu]") as El).getAttribute("style") ?? "";
    assert.match(style, /--sp-foreground:rgb\(1, 2, 3\)/);
    const checks = Array.from(document.querySelectorAll("[data-sp-check]")).map(
      (n) => (n as El).textContent,
    );
    assert.deepEqual(checks, ["", "✓", ""]);
  } finally {
    menu.stop();
  }
});

test("menu shows the header, descriptions, warning, and a clear row that empties the pins", async () => {
  const document = page({ voiceAgentId: AGENT });
  const { client, writes } = fakeClient(["watchtower", "sequential-thinking"]);
  const state = createSkillState(client);
  await state.load(AGENT);
  const menu = installSkillsMenu(document as unknown as Doc, undefined, state);
  try {
    menu.update();
    const wrapper = document.querySelector("[data-skill-pins]")!;
    const trigger = document.querySelector("[data-sp-trigger]")!;
    trigger.click();
    assert.equal(document.querySelector("[data-sp-head]")?.textContent, "Pinned for this chat");
    const descriptions = Array.from(document.querySelectorAll("[data-sp-desc]")).map(
      (node) => (node as El).textContent,
    );
    assert.deepEqual(descriptions, [
      "Plan and track watchtower tasks",
      "Plan and run a goal with subagents",
      "Step-by-step reasoning with revisions",
    ]);
    const warn = document.querySelector("[data-sp-warn]")!;
    assert.equal(warn.textContent, "Starts goal work on every turn");
    assert.equal(
      warn.closest("[data-sp-option]")?.getAttribute("data-sp-option"),
      "chase-goal-claude",
    );
    const clear = document.querySelector("[data-sp-clear]")!;
    assert.equal(clear.textContent, "Bỏ chọn tất cả");
    assert.equal(clear.hasAttribute("disabled"), false);
    clear.click();
    await settle();
    assert.deepEqual(writes.at(-1), []);
    assert.ok(document.querySelector("[data-sp-menu]"), "menu stays open");
    assert.equal(clear.hasAttribute("disabled"), true);
    assert.equal(clear.textContent, "Bỏ chọn tất cả", "the disabled row keeps its text");
    assert.equal(trigger.textContent, "Skills");
    assert.equal(trigger.getAttribute("title"), "Pin skills for this chat");
    assert.equal(wrapper.getAttribute("data-sp-state"), "empty");
    const checks = Array.from(document.querySelectorAll("[data-sp-check]")).map(
      (node) => (node as El).textContent,
    );
    assert.deepEqual(checks, ["", "", ""]);
  } finally {
    menu.stop();
  }
});

test("a pill without an agent has the off state", async () => {
  const document = page({ voiceAgentId: "new-workspace" });
  const { client } = fakeClient();
  const menu = installSkillsMenu(document as unknown as Doc, undefined, createSkillState(client));
  try {
    menu.update();
    assert.equal(document.querySelector("[data-skill-pins]")!.getAttribute("data-sp-state"), "off");
  } finally {
    menu.stop();
  }
});
