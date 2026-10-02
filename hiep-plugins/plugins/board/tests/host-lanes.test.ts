import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { runInNewContext } from "node:vm";
import test from "node:test";
import ts from "typescript";
import type { BoardRun } from "../shared/board";

const require = createRequire(import.meta.url);
const source = readFileSync(new URL("../client/page.tsx", import.meta.url), "utf8");
const code = ts.transpileModule(`${source}\nexport { HostBoard };`, {
  compilerOptions: {
    module: ts.ModuleKind.CommonJS,
    target: ts.ScriptTarget.ES2022,
    jsx: ts.JsxEmit.ReactJSX,
  },
}).outputText;
const base: BoardRun = {
  id: "same-run",
  agentId: "same-agent",
  projectId: "same-project",
  projectKey: "same-project",
  project: "App",
  cwd: "/work/app",
  title: "Done",
  starred: false,
  provider: "codex",
  status: "completed",
  startedAt: null,
  endedAt: "2026-10-01T00:00:00Z",
};
function elements(node: any): any[] {
  if (!node || typeof node !== "object") return [];
  return [node, ...[node.props?.children].flat(Infinity).flatMap(elements)];
}
function harness() {
  const queries: any[] = [];
  const opened: any[] = [];
  const calls: any[] = [];
  const exports = {} as { HostBoard: (props: any) => any };
  const react = require("react");
  runInNewContext(code, {
    exports,
    require: (id: string) => {
      if (id === "react/jsx-runtime") return require(id);
      if (id === "react")
        return {
          ...react,
          memo: (value: any) => value,
          useMemo: (read: any) => read(),
          useCallback: (value: any) => value,
          useRef: (value: any) => ({ current: value }),
          useState: (value: any) => [value, () => {}],
          useEffect: () => {},
        };
      if (id === "react-native")
        return { View: "View", Text: "Text", Pressable: "Pressable", Platform: { OS: "web" } };
      if (id === "@tanstack/react-query")
        return {
          useQuery: (options: any) => {
            queries.push(options);
            return {
              data: { runs: [base], observingSince: options.queryKey[1] },
              dataUpdatedAt: Date.now(),
              refetch: async () => {},
              isError: false,
              isPaused: false,
            };
          },
        };
      if (id.startsWith("../shared/")) return require(id);
      if (id === "./connection" || id === "./remove")
        return require(id.replace("./", "../client/"));
      if (id === "./host-settings")
        return {
          useHostSettings: (definition: any) => ({
            status: "ready",
            values: definition.id === "project-colors" ? { hues: {} } : {},
            save: async () => true,
          }),
        };
      if (id === "./clock") return { useClock: (read: any) => read(Date.now()) };
      return {};
    },
  });
  const render = (serverId: string, status = "online") =>
    exports.HostBoard({
      host: { id: serverId, label: serverId },
      theme: { colors: {} },
      layout: { compact: true, platform: "web" },
      scale: 1,
      view: "runs",
      status,
      navigation: { openAgent: (input: any) => opened.push(input) },
      client: {
        rpc: async (contract: any, input: any) => {
          calls.push([serverId, contract.name, input]);
          return { updated: true, removed: true };
        },
        openNewWorkspace: (input: any) => opened.push(input),
      },
    });
  return { render, queries, opened, calls };
}

test("identical run/project IDs retain host-specific open, star, remove, and new-thread actions", async () => {
  const h = harness();
  for (const host of ["air", "mini"]) {
    const nodes = elements(h.render(host));
    const column = nodes.find((node) => node.props?.title === "Just finished");
    assert.ok(column);
    column.props.onOpen(base.agentId);
    column.props.onOpenProject(base);
    await column.props.onStar(base.id, true);
    await column.props.onRemove(base.id);
  }
  assert.deepEqual(
    h.queries.map((query) => Array.from(query.queryKey)),
    [
      ["board", "air"],
      ["board", "mini"],
    ],
  );
  assert.deepEqual(
    h.opened.map((input) => input.serverId),
    ["air", "air", "mini", "mini"],
  );
  assert.deepEqual(
    h.opened.filter((input) => input.agentId).map((input) => input.anchor),
    ["latest-prompt", "latest-prompt"],
  );
  assert.deepEqual(
    h.calls.map(([host, method, input]) => [host, method, input.observingSince ?? input.agentId]),
    [
      ["air", "board.set-starred", "air"],
      ["air", "board.remove-finished", "air"],
      // A removed card's processes are released on the card's own host.
      ["air", "board.close-runtime", base.agentId],
      ["mini", "board.set-starred", "mini"],
      ["mini", "board.remove-finished", "mini"],
      ["mini", "board.close-runtime", base.agentId],
    ],
  );
});

test("an offline lane keeps cached cards visibly offline and disables remove/polling", async () => {
  const h = harness();
  const nodes = elements(h.render("mini", "offline"));
  assert.equal(h.queries[0].enabled, false);
  assert.ok(nodes.some((node) => node.props?.children === "Offline"));
  assert.ok(
    nodes.some(
      (node) => node.props?.children === "Host is offline. Showing the last successful snapshot.",
    ),
  );
  const column = nodes.find((node) => node.props?.title === "Just finished");
  assert.equal(column.props.onRemove, undefined);
  await assert.rejects(column.props.onStar(base.id, true), /Host is offline/);
  assert.equal(h.calls.length, 0);
});

test("a host lane names its host and keeps Running and Just finished as the only columns", () => {
  const h = harness();
  const nodes = elements(h.render("mini"));
  assert.ok(
    nodes.some(
      (node) => node.props?.accessibilityRole === "header" && node.props?.children === "mini",
    ),
  );
  assert.ok(nodes.some((node) => node.props?.accessibilityLabel === "0 running, 1 finished"));
  assert.deepEqual(
    nodes.filter((node) => node.props?.title).map((node) => node.props.title),
    ["Running", "Just finished"],
  );
  assert.ok(!nodes.some((node) => ["tab", "tablist"].includes(node.props?.accessibilityRole)));
});

test("a lane shows a short host name and one muted line of card metadata", () => {
  const h = harness();
  const nodes = elements(h.render("Hieps-Mac-mini.local"));
  const headers = nodes.filter((node) => node.props?.accessibilityRole === "header");
  assert.ok(headers.some((node) => node.props.children === "Hieps-Mac-mini"));
  assert.ok(!headers.some((node) => node.props.children === "Hieps-Mac-mini.local"));
  // Counts are one text line in the rail, not separate oversized figures.
  assert.ok(nodes.some((node) => node.props?.children === "0 running · 1 finished"));
});

test("the rail picks a device glyph from the host name and falls back to a generic one", () => {
  const h = harness();
  const glyph = (label: string) =>
    elements(h.render(label))
      .map((node) => node.props?.name)
      .find((name) => name === "Laptop" || name === "HardDrive");
  assert.equal(glyph("hieps-MacBook-Air-2.local"), "Laptop");
  assert.equal(glyph("Hieps-Mac-mini.local"), "HardDrive");
  assert.equal(glyph("build-box"), "HardDrive");
});
