import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { runInNewContext } from "node:vm";
import ts from "typescript";

test("parent action opens the selected host's direct parent and supports repeated jumps", () => {
  const require = createRequire(import.meta.url);
  const source = readFileSync(new URL("../client/parent.tsx", import.meta.url), "utf8");
  const compiled = ts.transpileModule(source, {
    compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.ReactJSX },
  }).outputText;
  const exports: { installParentNavigation?: (client: any) => any } = {};
  runInNewContext(compiled, {
    exports,
    require: (id: string) => {
      if (id === "react/jsx-runtime") return require(id);
      if (id === "react")
        return {
          useEffect: (effect: () => void) => effect(),
          useSyncExternalStore: (_subscribe: unknown, snapshot: () => unknown) => snapshot(),
        };
      if (id === "react-native") return { Text: "Text" };
      return {};
    },
  });
  let Surface: any;
  let removed = false;
  let surfaces = 0;
  const opened: string[] = [];
  const props = {
    theme: { colors: { foregroundMuted: "gray" } },
    navigation: { openAgent: ({ agentId }: { agentId: string }) => opened.push(agentId) },
  };
  const parent = exports.installParentNavigation!({
    addSurface(id: string, component: unknown) {
      assert.equal(id, "parent");
      Surface = component;
      return () => {
        removed = true;
      };
    },
    openSurface(id: string) {
      assert.equal(id, "parent");
      surfaces++;
      Surface(props);
    },
  });
  Surface(props);
  assert.deepEqual(opened, []);
  parent.open("first-parent");
  parent.open("second-parent");
  parent.open("second-parent");
  assert.deepEqual(opened, ["first-parent", "second-parent", "second-parent"]);
  assert.equal(surfaces, 3);
  const fallback = Surface({ ...props, navigation: undefined });
  assert.match(fallback.props.children, /unavailable/);
  parent.cleanup();
  assert.equal(removed, true);
});
