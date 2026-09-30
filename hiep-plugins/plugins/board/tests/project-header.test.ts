import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { runInNewContext } from "node:vm";
import test from "node:test";
import ts from "typescript";
import { projectColors } from "../shared/project-colors";
import type * as Header from "../client/project-header";

type Module = typeof Header;

function load(platform: string, document?: unknown) {
  const source = readFileSync(new URL("../client/project-header.ts", import.meta.url), "utf8");
  const code = ts.transpileModule(source, {
    compilerOptions: { module: ts.ModuleKind.CommonJS },
  }).outputText;
  const exports = {} as Module;
  const timers: (() => void)[] = [];
  runInNewContext(code, {
    exports,
    document,
    setInterval: (callback: () => void) => timers.push(callback),
    clearInterval: () => {},
    require: (id: string) => {
      if (id === "react-native") return { Platform: { OS: platform } };
      if (id === "@getpaseo/plugin")
        return { settingsRpc: (settingsId: string) => ({ read: settingsId }) };
      if (id === "../shared/project-colors") return { projectColors };
      throw new Error(`Unexpected module ${id}`);
    },
  });
  return { module: exports, timers };
}

function element(text: string) {
  const attributes = new Map<string, string>();
  const properties = new Map<string, string>();
  return {
    textContent: text,
    attributes,
    properties,
    getAttribute: (name: string) => attributes.get(name) ?? null,
    setAttribute: (name: string, value: string) => void attributes.set(name, value),
    removeAttribute: (name: string) => void attributes.delete(name),
    style: {
      setProperty: (name: string, value: string) => void properties.set(name, value),
      removeProperty: (name: string) => void properties.delete(name),
    },
  };
}

const { module } = load("web");

test("hues map by display name and skip names shared by several projects", () => {
  const hues = module.headerHues(
    [
      { projectId: "a", projectDisplayName: "hiep-paseo-plugin" },
      { projectId: "b", projectDisplayName: "app" },
      { projectId: "c", projectDisplayName: "app" },
      { projectId: "d", projectDisplayName: "no-color" },
    ],
    { a: 252, b: 28, c: 90 },
  );
  // The module runs in its own VM realm; compare plain values, not prototypes.
  assert.equal(JSON.stringify([...hues]), JSON.stringify([["hiep-paseo-plugin", 252]]));
});

test("subtitle gets its initial and hue, and a reused element drops the old hue", () => {
  const subtitle = element(" hiep-paseo-plugin ");
  const hues = new Map([["hiep-paseo-plugin", 252]]);
  module.syncHeaders([subtitle], hues);
  assert.equal(subtitle.attributes.get("data-board-project"), "hiep-paseo-plugin");
  assert.equal(subtitle.attributes.get("data-board-initial"), "H");
  assert.equal(subtitle.attributes.get("data-board-hue"), "252");
  assert.equal(subtitle.properties.get("--board-hue"), "252");

  subtitle.textContent = "élan";
  module.syncHeaders([subtitle], hues);
  assert.equal(subtitle.attributes.get("data-board-initial"), "É");
  assert.equal(subtitle.attributes.has("data-board-hue"), false);
  assert.equal(subtitle.properties.has("--board-hue"), false);

  subtitle.textContent = "";
  module.syncHeaders([subtitle], hues);
  assert.equal(subtitle.attributes.get("data-board-project"), "élan");
});

test("the first host that knows a project keeps its color", () => {
  const subtitle = element("app");
  module.syncHeaders([subtitle], new Map([["app", 10]]));
  module.syncHeaders([subtitle], new Map());
  module.syncHeaders([subtitle], new Map([["app", 200]]));
  assert.equal(subtitle.attributes.get("data-board-hue"), "10");
});

test("web install styles headers from host colors and cleans up", async () => {
  const subtitle = element("hiep-paseo-plugin");
  const appended: { textContent: string; removed: boolean }[] = [];
  const document = {
    head: { appendChild: (node: { textContent: string; removed: boolean }) => appended.push(node) },
    createElement: () => {
      const node = { textContent: "", removed: false, remove: () => (node.removed = true) };
      return node;
    },
    querySelectorAll: (selector: string) => {
      assert.equal(selector, '[data-testid="workspace-header-subtitle"]');
      return [subtitle];
    },
  };
  const { module: web, timers } = load("web", document);
  const reads: unknown[] = [];
  const stop = web.installProjectHeader({
    paseo: {
      projects: {
        list: async () => ({
          projects: [{ projectId: "p1", projectDisplayName: "hiep-paseo-plugin" }],
        }),
      },
    },
    rpc: async (contract: unknown) => {
      reads.push(contract);
      return { status: "ready", revision: "1", values: { hues: { p1: 140 } } };
    },
  } as never);
  assert.equal(appended[0].textContent, web.HEADER_CSS);
  assert.equal(subtitle.attributes.get("data-board-initial"), "H");
  await new Promise((resolve) => setImmediate(resolve));
  assert.deepEqual(reads, [projectColors.id]);
  assert.equal(subtitle.attributes.get("data-board-hue"), "140");
  assert.equal(timers.length, 2);
  stop();
  assert.equal(appended[0].removed, true);
  assert.equal(subtitle.attributes.size, 0);
  assert.equal(subtitle.properties.size, 0);
});

test("native clients do nothing", () => {
  const { module: native } = load("ios", {
    createElement: () => assert.fail("native must not touch the DOM"),
  });
  native.installProjectHeader({} as never)();
});
