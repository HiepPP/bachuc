import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { runInNewContext } from "node:vm";
import ts from "typescript";
import type { BoardRun } from "../shared/board";
import { buildRunTrees } from "../shared/tree";

type Element = { key?: string; type: string | ((props: any) => Element); props: any };
const require = createRequire(import.meta.url);
// Exercise real JSX and handlers with host primitives, without native bindings in Node.
const source = readFileSync(new URL("../client/page.tsx", import.meta.url), "utf8");
const compiled = ts.transpileModule(`${source}\nexport { RunCluster };`, {
  compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX },
}).outputText;
let measuredPanelWidth = 0;
const exports: { RunCluster?: (props: any) => Element } = {};
runInNewContext(compiled, {
  exports,
  require: (id: string) => {
    if (id === "react/jsx-runtime") return require(id);
    if (id === "../shared/effort") return require("../shared/effort");
    if (id === "./clock") return { useClock: (read: (now: number) => unknown) => read(Date.now()) };
    if (id === "react")
      return {
        ...require("react"),
        useEffect: () => {},
        useState: (initial: unknown) =>
          typeof initial === "number"
            ? [
                measuredPanelWidth,
                (width: number) => {
                  measuredPanelWidth = width;
                },
              ]
            : [typeof initial === "function" ? initial() : initial, () => {}],
      };
    if (id === "react-native")
      return {
        View: "View",
        Text: "Text",
        Pressable: "Pressable",
        ActivityIndicator: "ActivityIndicator",
        Animated: {
          View: "Animated.View",
          Value: class {
            interpolate() {
              return 0;
            }
          },
        },
        Easing: {},
        Platform: { OS: "web" },
      };
    return {};
  },
});
function render(element: any): any {
  if (Array.isArray(element)) return element.map(render);
  if (!element || typeof element !== "object") return element;
  if (typeof element.type === "function") return render(element.type(element.props));
  return { ...element, props: { ...element.props, children: render(element.props.children) } };
}
function elements(value: any): Element[] {
  if (Array.isArray(value)) return value.flatMap(elements);
  if (!value || typeof value !== "object") return [];
  return [value, ...elements(value.props.children)];
}

const base = {
  project: "App",
  projectKey: "app",
  provider: "codex",
  starred: false,
  startedAt: null,
  endedAt: null,
  status: "running" as const,
};
const theme = {
  colors: {
    foreground: "black",
    foregroundMuted: "gray",
    surface0: "white",
    surface1: "white",
    surface2: "white",
    border: "gray",
    accent: "blue",
    accentForeground: "white",
    statusSuccess: "green",
    statusWarning: "orange",
    statusDanger: "red",
  },
};

test("collapse hides descendants, keeps attention, and expands with separate card actions", async () => {
  const [tree] = buildRunTrees([
    { ...base, id: "parent", agentId: "parent", title: "Parent" },
    {
      ...base,
      id: "child",
      agentId: "child",
      title: "Child",
      parentAgentId: "parent",
      needsInput: true,
    },
  ]);
  const collapsed = new Set<string>();
  const opened: string[] = [];
  const starred: string[] = [];
  const props = {
    tree,
    compact: false,
    collapsed,
    scale: 1,
    now: Date.now(),
    theme,
    onOpen: (id: string) => opened.push(id),
    onStar: async (id: string) => {
      starred.push(id);
    },
    onRemove: async () => {},
    onToggle: (id: string) => (collapsed.has(id) ? collapsed.delete(id) : collapsed.add(id)),
  };
  const view = () => elements(render(exports.RunCluster!(props)));
  let nodes = view();
  const button = (label: string) => nodes.find((node) => node.props.accessibilityLabel === label)!;
  const text = (value: string) => nodes.some((node) => node.props.children === value);
  assert.equal(button("Collapse subagents of Parent").props.accessibilityState.expanded, true);
  // Expanded members show their own status; the shared project row is not repeated.
  assert.ok(!text("1 needs input"));
  assert.equal(nodes.filter((node) => node.props.children === "App").length, 1);
  // Running clusters never offer Remove.
  assert.ok(!nodes.some((node) => /^Remove .* from Board$/.test(node.props.accessibilityLabel)));
  button("Open conversation Child").props.onPress();
  await button("Star Child").props.onPress();
  assert.deepEqual(opened, ["child"]);
  assert.deepEqual(starred, ["child"]);
  button("Collapse subagents of Parent").props.onPress();
  nodes = view();
  assert.equal(button("Expand subagents of Parent").props.accessibilityState.expanded, false);
  assert.equal(button("Open conversation Child"), undefined);
  assert.ok(text("1 needs input"));
  assert.ok(button("Open conversation Parent"));
  props.compact = true;
  button("Expand subagents of Parent").props.onPress();
  nodes = view();
  assert.ok(button("Open conversation Child"));
  assert.deepEqual(opened, ["child"]);
});

test("Remove on a finished cluster announces the cascade and waits for running subagents", async () => {
  const finished = { ...base, status: "completed" as const, endedAt: "2026-09-22T00:00:00Z" };
  const rows: BoardRun[] = [
    { ...finished, id: "parent", agentId: "parent", title: "Parent" },
    { ...finished, id: "child", agentId: "child", title: "Child", parentAgentId: "parent" },
    { ...finished, id: "leaf", agentId: "leaf", title: "Leaf", parentAgentId: "child" },
  ];
  const removed: string[] = [];
  const mount = (runs: BoardRun[]) =>
    elements(
      render(
        exports.RunCluster!({
          tree: buildRunTrees(runs)[0],
          compact: false,
          collapsed: new Set<string>(),
          scale: 1,
          now: Date.now(),
          theme,
          onStar: async () => {},
          onRemove: async (id: string) => {
            removed.push(id);
          },
          onToggle: () => {},
        }),
      ),
    );
  let nodes = mount(rows);
  const button = (label: string) => nodes.find((node) => node.props.accessibilityLabel === label)!;
  const text = (value: string) => nodes.some((node) => node.props.children === value);
  assert.ok(text("Remove all · 3"));
  // Compact children use icons; accessible names still announce the cascade scope.
  assert.ok(button("Remove Child and 1 subagent from Board"));
  assert.ok(button("Remove Leaf from Board"));
  await button("Remove Parent and 2 subagents from Board").props.onPress();
  await button("Remove Child and 1 subagent from Board").props.onPress();
  await button("Remove Leaf from Board").props.onPress();
  assert.deepEqual(removed, ["parent", "child", "leaf"]);
  // A running leaf keeps every ancestor's Remove hidden until the cluster settles.
  rows[2] = { ...rows[2], status: "running", endedAt: null };
  nodes = mount(rows);
  assert.equal(button("Remove Parent and 2 subagents from Board"), undefined);
  assert.equal(button("Remove Child and 1 subagent from Board"), undefined);
  assert.ok(!text("Remove"));
  assert.ok(button("Open conversation Leaf"));
});

test("child grid responds to its own panel width and Board scale", () => {
  measuredPanelWidth = 0;
  const [tree] = buildRunTrees([
    { ...base, id: "parent", agentId: "parent", title: "Parent" },
    { ...base, id: "child", agentId: "child", parentAgentId: "parent", title: "Child" },
    { ...base, id: "sibling", agentId: "sibling", parentAgentId: "parent", title: "Sibling" },
  ]);
  const props = {
    tree,
    compact: false,
    collapsed: new Set<string>(),
    scale: 1,
    now: Date.now(),
    theme,
    onStar: async () => {},
    onToggle: () => {},
  };
  const view = () => elements(render(exports.RunCluster!(props)));
  let nodes = view();
  const measure = (width: number) => {
    nodes
      .find((node) => node.props.onLayout)!
      .props.onLayout({ nativeEvent: { layout: { width } } });
    nodes = view();
  };
  const width = (id: string) => nodes.find((node) => node.key === id)!.props.style.width;
  measure(440);
  assert.equal(typeof width("child"), "number");
  assert.equal(width("child"), width("sibling"));
  assert.ok(width("child") * 2 < 440);
  measure(320);
  assert.equal(width("child"), "100%");
  measure(440);
  props.scale = 1.5;
  nodes = view();
  assert.equal(width("child"), "100%");
  measuredPanelWidth = 0;
});

test("grouped cards omit the project and show model with effort", () => {
  const [tree] = buildRunTrees([
    {
      ...base,
      id: "parent",
      agentId: "parent",
      title: "Parent",
      model: "Opus 5.5",
      effort: "High",
    },
    { ...base, id: "child", agentId: "child", title: "Child", parentAgentId: "parent" },
  ]);
  const nodes = elements(
    render(
      exports.RunCluster!({
        tree,
        compact: false,
        collapsed: new Set(),
        scale: 1,
        theme,
        inheritedProject: true,
        onStar: async () => {},
        onToggle: () => {},
      }),
    ),
  );
  const text = (value: string) => nodes.filter((node) => node.props.children === value).length;
  assert.equal(text("App"), 0);
  assert.equal(text("Opus 5.5"), 1);
  assert.equal(text("High"), 1);
  // A subagent whose model is not read yet falls back to its provider.
  assert.equal(text("codex"), 1);
});
