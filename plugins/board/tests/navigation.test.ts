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

function harness(platform = "web", hasSidebar = true) {
  let host = "macair";
  const opened: string[] = [];
  const shortcuts: (() => void)[] = [];
  const events: (() => void)[] = [];
  const native = { Platform: { OS: platform } };
  const web = {};
  runInNewContext(compile("../client/web.ts"), {
    exports: web,
    require: () => native,
    document: {
      querySelector(selector: string) {
        assert.equal(selector, '[data-testid="plugin-sidebar-board-board"]');
        return hasSidebar ? { click: () => opened.push(host) } : null;
      },
    },
  });
  const entry: { default?: (client: unknown) => () => void } = {};
  runInNewContext(compile("../index.client.tsx"), {
    exports: entry,
    require: (id: string) => {
      if (id === "react-native") return native;
      if (id === "./client/web") return { ...web, installRemovePlacement: () => () => {} };
      if (id === "./client/shortcut")
        return { installBoardShortcut: (open: () => void) => (shortcuts.push(open), () => {}) };
      if (id === "./client/events")
        return { installBoardEvents: (open: () => void) => (events.push(open), () => {}) };
      return new Proxy({}, { get: () => () => ({ cleanup() {} }) });
    },
  });
  for (const installedHost of ["macmini", "macair"]) {
    entry.default!({
      rpc: async () => ({ serverId: installedHost }),
      addSurface: () => () => {},
      addSidebarItem: () => () => {},
      addSettingsScreen: () => () => {},
      openSurface: () => opened.push(installedHost),
    });
  }
  return { opened, shortcuts, events, select: (value: string) => (host = value) };
}

test("shortcuts use the selected Board, while scoped events open their installation", () => {
  const h = harness();
  for (const host of ["macair", "macmini", "macair"]) {
    h.select(host);
    for (const open of h.shortcuts) open();
    assert.deepEqual(h.opened.splice(0), [host, host]);
    for (const open of h.events) open();
    assert.deepEqual(h.opened.splice(0), ["macmini", "macair"]);
  }
});

test("native clients and absent sidebar keep the supported surface fallback", () => {
  for (const h of [harness("ios"), harness("web", false)]) {
    h.shortcuts[0]();
    assert.deepEqual(h.opened, ["macmini"]);
  }
});
