import { describe, expect, it, vi } from "vitest";
import { runPluginClientBundle, type PluginClientRuntime } from "./evaluate";

// This suite checks module registration, not Markdown rendering. The native dependency ships JSX in .js.
vi.mock("@/components/markdown/renderer", () => ({ MarkdownRenderer: () => null }));
// The stub theme has no syntax palette.
vi.mock("@/styles/syntax-token-styles", () => ({ syntaxTokenStyleFor: () => undefined }));

const runtime = {
  paseo: {},
  async rpc() {},
  openSettings() {},
  openSurface() {},
  openPanel() {},
  addHeaderButton() {
    return { update() {}, remove() {} };
  },
  addComposerPill() {
    return { update() {}, remove() {} };
  },
} as unknown as PluginClientRuntime;

function evaluatePluginClientBundle(id: string, source: string) {
  return runPluginClientBundle(id, source, runtime);
}

function bundle(body: string): string {
  return `(function(require) {
    const module = { exports: {} };
    module.exports.default = function(plugin) { ${body}; return function() {}; };
    return module.exports;
  })`;
}

describe("evaluatePluginClientBundle", () => {
  it("registers assistant selection actions and drops them on unload", async () => {
    const evaluated = evaluatePluginClientBundle(
      "cite",
      bundle(`
      plugin.addAssistantSelectionAction({
        id: "cite", title: "Cite", onSelect(selection) { globalThis.__citeSelection = selection; }
      });
    `),
    );
    expect(evaluated.assistantSelectionActions?.map((action) => action.title)).toEqual(["Cite"]);
    await evaluated.assistantSelectionActions![0].onSelect({
      serverId: "host",
      workspaceId: null,
      agentId: "agent-1",
      text: "> quoted",
    });
    expect(Reflect.get(globalThis, "__citeSelection")).toMatchObject({ text: "> quoted" });
    Reflect.deleteProperty(globalThis, "__citeSelection");
    await evaluated.cleanup();
    expect(evaluated.assistantSelectionActions).toEqual([]);
  });

  it("rejects a duplicate assistant selection action", () => {
    expect(() =>
      evaluatePluginClientBundle(
        "cite-twice",
        bundle(`
        plugin.addAssistantSelectionAction({ id: "cite", title: "Cite", onSelect() {} });
        plugin.addAssistantSelectionAction({ id: "cite", title: "Cite", onSelect() {} });
      `),
      ),
    ).toThrow("Duplicate assistant selection action: cite");
  });

  it("forwards composer chips to the host runtime", async () => {
    const addComposerAttachment = vi.fn();
    const evaluated = runPluginClientBundle(
      "cite",
      bundle(`
      plugin.addComposerAttachment({
        agentId: "agent-1", sourceId: "quotes", sourceTitle: "Quote", icon: "Quote",
        item: { id: "q1", identifier: "q1", title: "Quote", url: "https://example.com/q1",
          text: "> quoted", resourceType: "quote" },
        commentable: true,
      });
    `),
      { ...runtime, addComposerAttachment } as PluginClientRuntime,
    );
    expect(addComposerAttachment).toHaveBeenCalledWith(
      expect.objectContaining({ agentId: "agent-1", sourceId: "quotes", commentable: true }),
    );
    await evaluated.cleanup();
  });

  it("rejects a composer chip the draft could not store", () => {
    const addComposerAttachment = vi.fn();
    const add = (fields: string) =>
      runPluginClientBundle(
        "cite-bad",
        bundle(`
        plugin.addComposerAttachment({
          agentId: "agent-1", sourceId: "quotes", sourceTitle: "Quote", icon: "Quote",
          item: { id: "q1", identifier: "q1", title: "Quote", url: "https://example.com/q1",
            text: "> quoted", resourceType: "quote" },
          ${fields}
        });
      `),
        { ...runtime, addComposerAttachment } as PluginClientRuntime,
      );

    expect(() => add(`sourceTitle: " "`)).toThrow("addComposerAttachment needs a sourceTitle");
    expect(() => add(`item: { id: "q1", url: "" }`)).toThrow("invalid item");
    expect(addComposerAttachment).not.toHaveBeenCalled();
  });

  it("retains the sidebar project addition hook", async () => {
    const evaluated = evaluatePluginClientBundle(
      "spaces",
      bundle(`
      plugin.addSidebarProjectFilter({
        id: "spaces", subscribe() { return function() {}; }, isVisible() { return true; },
        async onProjectAdded() { throw new Error("Assignment failed"); }
      });
    `),
    );
    await expect(
      evaluated.sidebarProjectFilters![0].onProjectAdded!(
        {
          viewKey: "new",
          name: "New",
          serverIds: ["host"],
          projectIds: ["p"],
        },
        { activeServerId: "host" },
      ),
    ).rejects.toThrow("Assignment failed");
    await evaluated.cleanup();
  });
  it("releases button registrations when client setup throws", () => {
    let active = 0;
    function addButton() {
      active++;
      return {
        update() {},
        remove() {
          active--;
        },
      };
    }
    expect(() =>
      runPluginClientBundle(
        "failed-buttons",
        bundle(`
      plugin.addHeaderButton({ id: "header", workspaceId: "workspace", button: { title: "Header", icon: "Scan", behavior: { kind: "action", onPress() {} } } });
      plugin.addComposerPill({ id: "pill", workspaceId: "workspace", agentId: "agent", button: { title: "Pill", icon: "Scan", behavior: { kind: "action", onPress() {} } } });
      throw new Error("Setup failed");
    `),
        { ...runtime, addHeaderButton: addButton, addComposerPill: addButton },
      ),
    ).toThrow("Setup failed");
    expect(active).toBe(0);
  });

  it("accepts memoized settings screens", () => {
    const plugin = evaluatePluginClientBundle(
      "settings",
      bundle(`
        const Component = require("react").memo(function Settings() { return null; });
        plugin.addSettingsScreen({ id: "display", title: "Display", icon: "Settings", Component });
      `),
    );
    expect(plugin.settingsScreens.map((screen) => screen.id)).toEqual(["display"]);
  });

  it("returns idempotent removers for every client registration", () => {
    let pillCount = 0;
    const plugin = runPluginClientBundle(
      "removals",
      bundle(`
        function Component() { return null; }
        const schema = { safeParse(value) { return { success: true, data: value }; } };
        globalThis.__pluginRemovals = [
          plugin.addSurface("main", Component),
          plugin.addSettingsScreen({ id: "display", title: "Display", icon: "Settings", Component }),
          plugin.addSidebarItem({ id: "main", title: "Main", icon: "Blocks", surface: "main" }),
          plugin.addWorkspacePanel({ id: "panel", title: "Panel", icon: "Blocks", context: "workspace", Component }),
          plugin.addCommandCenterItem({ id: "command", title: "Command", icon: "Blocks", context: "global", onSelect() {} }),
          plugin.addSlashCommand({ name: "review", description: "Review", argumentHint: "", context: "workspace", onSubmit() {} }),
          plugin.addComposerPill({ id: "pill", workspaceId: "workspace", agentId: "agent", button: { title: "Pill", icon: "Scan", behavior: { kind: "action", onPress() {} } } }).remove,
          plugin.addAttachmentSource({ id: "issues", title: "Issues", icon: "Blocks", pickerTitle: "Attach issue", searchPlaceholder: "Search", search: { name: "issues.search", input: {}, output: {} } }),
          plugin.addTheme({ id: "night", name: "Night", appearance: "dark", colors: { background: "#000", foreground: "#fff", raised: "#111", control: "#222", border: "#333", mutedForeground: "#aaa", ring: "#555" } }),
          plugin.addTimelineTransformer({ id: "transformer", query: { itemType: "tool_call" }, transform() { return { items: [] }; } }),
          plugin.addTimelineRenderer({ kind: "card", version: 1, schema, Component }),
        ];
      `),
      {
        ...runtime,
        addComposerPill() {
          pillCount += 1;
          return {
            update() {},
            remove() {
              pillCount -= 1;
            },
          };
        },
      },
    );
    const removals = Reflect.get(globalThis, "__pluginRemovals") as Array<() => void>;

    expect(
      [
        plugin.surfaces,
        plugin.settingsScreens,
        plugin.sidebarItems,
        plugin.workspacePanels,
        plugin.commandCenterItems,
        plugin.clientSlashCommands,
        plugin.attachmentSources,
        plugin.themes,
        plugin.timelineTransformers,
        plugin.timelineRenderers,
      ].every((items) => items.length === 1),
    ).toBe(true);
    expect(pillCount).toBe(1);
    for (const remove of removals) {
      remove();
      remove();
    }
    expect(
      [
        plugin.surfaces,
        plugin.settingsScreens,
        plugin.sidebarItems,
        plugin.workspacePanels,
        plugin.commandCenterItems,
        plugin.clientSlashCommands,
        plugin.attachmentSources,
        plugin.themes,
        plugin.timelineTransformers,
        plugin.timelineRenderers,
      ].every((items) => items.length === 0),
    ).toBe(true);
    expect(pillCount).toBe(0);
    Reflect.deleteProperty(globalThis, "__pluginRemovals");
  });

  it("collects timeline transformers and renderers", () => {
    const plugin = evaluatePluginClientBundle(
      "reports",
      bundle(`
        function Card() { return null; }
        const schema = { safeParse(value) { return { success: true, data: value }; } };
        plugin.addTimelineTransformer({
          id: "test-report",
          query: { itemType: "tool_call" },
          transform() { return { items: [] }; },
        });
        plugin.addTimelineRenderer({
          kind: "test-report",
          version: 1,
          schema,
          Component: Card,
        });
      `),
    );

    expect(plugin.timelineTransformers.map(({ id, query }) => ({ id, query }))).toEqual([
      { id: "test-report", query: { itemType: "tool_call" } },
    ]);
    expect(plugin.timelineRenderers.map(({ kind, version }) => ({ kind, version }))).toEqual([
      { kind: "test-report", version: 1 },
    ]);
  });

  it("rejects unknown timeline item types", () => {
    expect(() =>
      evaluatePluginClientBundle(
        "reports",
        bundle(`
          plugin.addTimelineTransformer({
            id: "bad-query",
            query: { itemType: "settled" },
            transform() { return { items: [] }; },
          });
        `),
      ),
    ).toThrow("Timeline transformer bad-query has invalid item type: settled");
  });

  it("collects a surface and its sidebar placement", () => {
    const plugin = evaluatePluginClientBundle(
      "example",
      bundle(`
        function Surface() { return null; }
        plugin.addSurface("main", Surface);
        plugin.addSidebarItem({ id: "main", title: "Example", icon: "Blocks", surface: "main" });
      `),
    );

    expect(plugin.id).toBe("example");
    expect(plugin.surfaces.map((surface) => surface.id)).toEqual(["main"]);
    expect(plugin.sidebarItems).toEqual([
      { id: "main", title: "Example", icon: "Blocks", surface: "main" },
    ]);
  });

  it("collects a declarative attachment source", () => {
    const plugin = evaluatePluginClientBundle(
      "linear",
      bundle(`
        plugin.addAttachmentSource({
          id: "issues",
          title: "Linear issue",
          icon: "CircleDot",
          pickerTitle: "Attach Linear issue",
          searchPlaceholder: "Search by identifier or title",
          search: { name: "issues.search", input: {}, output: {} },
        });
      `),
    );

    expect(plugin.attachmentSources).toEqual([
      {
        id: "issues",
        title: "Linear issue",
        icon: "CircleDot",
        pickerTitle: "Attach Linear issue",
        searchPlaceholder: "Search by identifier or title",
        search: { name: "issues.search", input: {}, output: {} },
      },
    ]);
  });

  it("collects contextual workspace panels and Command Center items", () => {
    const plugin = evaluatePluginClientBundle(
      "review",
      bundle(`
        function ReviewPanel() { return null; }
        plugin.addWorkspacePanel({
          id: "review",
          title: "Review",
          icon: "Scan",
          context: "agent",
          Component: ReviewPanel,
        });
        plugin.addCommandCenterItem({
          id: "open-review",
          title: "Open review",
          icon: "Scan",
          context: "agent",
          onSelect() {},
        });
      `),
    );

    expect(
      plugin.workspacePanels.map(({ id, title, icon, context, locations }) => ({
        id,
        title,
        icon,
        context,
        locations,
      })),
    ).toEqual([
      {
        id: "review",
        title: "Review",
        icon: "Scan",
        context: "agent",
        locations: ["workspace"],
      },
    ]);
    expect(
      plugin.commandCenterItems.map(({ id, title, icon, context }) => ({
        id,
        title,
        icon,
        context,
      })),
    ).toEqual([{ id: "open-review", title: "Open review", icon: "Scan", context: "agent" }]);
  });

  it("normalizes and validates workspace panel locations", () => {
    const plugin = evaluatePluginClientBundle(
      "review",
      bundle(`
        function ReviewPanel() { return null; }
        plugin.addWorkspacePanel({
          id: "review",
          title: "Review",
          icon: "Scan",
          context: "agent",
          locations: ["workspace", "explorer"],
          Component: ReviewPanel,
        });
      `),
    );
    expect(plugin.workspacePanels[0]?.locations).toEqual(["workspace", "explorer"]);

    for (const [locations, message] of [
      ["[]", "must support at least one location"],
      ['["sidebar"]', "has invalid location: sidebar"],
      ['["explorer", "explorer"]', "has duplicate locations"],
    ] as const) {
      expect(() =>
        evaluatePluginClientBundle(
          "review",
          bundle(`
            function ReviewPanel() { return null; }
            plugin.addWorkspacePanel({
              id: "review",
              title: "Review",
              icon: "Scan",
              context: "agent",
              locations: ${locations},
              Component: ReviewPanel,
            });
          `),
        ),
      ).toThrow(message);
    }
  });

  it("rejects duplicate workspace panel and Command Center ids", () => {
    expect(() =>
      evaluatePluginClientBundle(
        "review",
        bundle(`
          function Panel() { return null; }
          const panel = { id: "review", title: "Review", icon: "Scan", context: "workspace", Component: Panel };
          plugin.addWorkspacePanel(panel);
          plugin.addWorkspacePanel(panel);
        `),
      ),
    ).toThrow("Duplicate workspace panel: review");

    expect(() =>
      evaluatePluginClientBundle(
        "review",
        bundle(`
          const item = { id: "review", title: "Review", icon: "Scan", context: "global", onSelect() {} };
          plugin.addCommandCenterItem(item);
          plugin.addCommandCenterItem(item);
        `),
      ),
    ).toThrow("Duplicate Command Center item: review");
  });

  it("runs the client entry with the full runtime context", () => {
    const plugin = evaluatePluginClientBundle(
      "review",
      bundle(`
        if (!plugin.paseo || !plugin.rpc || !plugin.openSurface || !plugin.openPanel || !plugin.addComposerPill) {
          throw new Error("missing client runtime");
        }
      `),
    );
    expect(plugin.id).toBe("review");
  });

  it("rejects duplicate attachment source ids", () => {
    expect(() =>
      evaluatePluginClientBundle(
        "linear",
        bundle(`
          const source = {
            id: "issues",
            title: "Linear issue",
            icon: "CircleDot",
            pickerTitle: "Attach Linear issue",
            searchPlaceholder: "Search",
            search: { name: "issues.search", input: {}, output: {} },
          };
          plugin.addAttachmentSource(source);
          plugin.addAttachmentSource(source);
        `),
      ),
    ).toThrow("Duplicate attachment source: issues");
  });

  it("collects a contributed theme", () => {
    const plugin = evaluatePluginClientBundle(
      "catppuccin",
      bundle(`
        plugin.addTheme({
          id: "mocha",
          name: "Catppuccin Mocha",
          appearance: "dark",
          colors: {
            background: "#1e1e2e",
            foreground: "#cdd6f4",
            raised: "#313244",
            control: "#45475a",
            border: "#45475a",
            accent: "#cba6f7",
            mutedForeground: "#a6adc8",
            ring: "#6c7086",
          },
        });
      `),
    );

    expect(plugin.themes.map((theme) => [theme.id, theme.name])).toEqual([
      ["mocha", "Catppuccin Mocha"],
    ]);
    expect(plugin.themes[0]?.colors.accent).toBe("#cba6f7");
  });

  it("rejects a theme with a color that is not a hex value", () => {
    expect(() =>
      evaluatePluginClientBundle(
        "catppuccin",
        bundle(`
          plugin.addTheme({
            id: "mocha",
            name: "Catppuccin Mocha",
            appearance: "dark",
            colors: {
              background: "rebeccapurple",
              foreground: "#cdd6f4",
              raised: "#313244",
              control: "#45475a",
              border: "#45475a",
              mutedForeground: "#a6adc8",
              ring: "#6c7086",
            },
          });
        `),
      ),
    ).toThrow("Must be a hex color");
  });

  it("rejects duplicate theme ids", () => {
    expect(() =>
      evaluatePluginClientBundle(
        "catppuccin",
        bundle(`
          const theme = {
            id: "mocha",
            name: "Catppuccin Mocha",
            appearance: "dark",
            colors: {
              background: "#1e1e2e",
              foreground: "#cdd6f4",
              raised: "#313244",
              control: "#45475a",
              border: "#45475a",
              mutedForeground: "#a6adc8",
              ring: "#6c7086",
            },
          };
          plugin.addTheme(theme);
          plugin.addTheme(theme);
        `),
      ),
    ).toThrow("Duplicate theme: mocha");
  });

  it("rejects a sidebar placement whose surface does not exist", () => {
    expect(() =>
      evaluatePluginClientBundle(
        "example",
        bundle(`
          plugin.addSidebarItem({ id: "main", title: "Example", icon: "Blocks", surface: "missing" });
        `),
      ),
    ).toThrow("references missing surface missing");
  });

  it("keeps a sidebar action and rejects one without onPress", () => {
    const plugin = evaluatePluginClientBundle(
      "example",
      bundle(`
        function Surface() { return null; }
        plugin.addSurface("main", Surface);
        plugin.addSidebarItem({ id: "main", title: "Example", icon: "Blocks", surface: "main", action: { onPress() {}, requiresWorkspace: true } });
      `),
    );
    expect(plugin.sidebarItems[0]?.action?.requiresWorkspace).toBe(true);
    expect(() =>
      evaluatePluginClientBundle(
        "example",
        bundle(`
          function Surface() { return null; }
          plugin.addSurface("main", Surface);
          plugin.addSidebarItem({ id: "main", title: "Example", icon: "Blocks", surface: "main", action: {} });
        `),
      ),
    ).toThrow("action has no onPress");
  });

  it("collects a new workspace panel that a sidebar action opens and rejects a missing one", () => {
    const plugin = evaluatePluginClientBundle(
      "example",
      bundle(`
        function Surface() { return null; }
        plugin.addSurface("main", Surface);
        plugin.addNewWorkspacePanel({ id: "board", title: "Board", icon: "Blocks", Component: Surface });
        plugin.addSidebarItem({ id: "main", title: "Example", icon: "Blocks", surface: "main", action: { onPress() {}, newWorkspacePanel: "board" } });
      `),
    );
    expect(plugin.newWorkspacePanels?.map((panel) => panel.id)).toEqual(["board"]);
    expect(plugin.sidebarItems[0]?.action?.newWorkspacePanel).toBe("board");
    expect(() =>
      evaluatePluginClientBundle(
        "example",
        bundle(`
          function Surface() { return null; }
          plugin.addSurface("main", Surface);
          plugin.addSidebarItem({ id: "main", title: "Example", icon: "Blocks", surface: "main", action: { onPress() {}, newWorkspacePanel: "board" } });
        `),
      ),
    ).toThrow("references missing new workspace panel board");
  });

  it("keeps a surface title and icon from addSurface options", () => {
    const plugin = evaluatePluginClientBundle(
      "example",
      bundle(`
        function Surface() { return null; }
        plugin.addSurface("main", Surface, { title: " Board ", icon: "Blocks" });
        plugin.addSurface("plain", Surface);
      `),
    );
    expect(plugin.surfaces.map(({ id, title, icon }) => ({ id, title, icon }))).toEqual([
      { id: "main", title: "Board", icon: "Blocks" },
      { id: "plain", title: undefined, icon: undefined },
    ]);
  });

  it("collects a shortcut and rejects a chord, a bad combo, or a missing surface", () => {
    const plugin = evaluatePluginClientBundle(
      "example",
      bundle(`
        function Surface() { return null; }
        plugin.addSurface("main", Surface);
        plugin.addShortcut({ id: "toggle", combo: "Mod+E", surface: "main", onPress() {} });
      `),
    );
    expect(plugin.shortcuts?.map(({ id, combo, surface }) => ({ id, combo, surface }))).toEqual([
      { id: "toggle", combo: "Mod+E", surface: "main" },
    ]);
    const failing = (body: string) => () =>
      evaluatePluginClientBundle(
        "example",
        bundle(`
          function Surface() { return null; }
          plugin.addSurface("main", Surface);
          ${body}
        `),
      );
    expect(
      failing(`plugin.addShortcut({ id: "chord", combo: "Mod+K Mod+E", onPress() {} });`),
    ).toThrow("must be one key combo");
    expect(failing(`plugin.addShortcut({ id: "bad", combo: "Mod+E+F", onPress() {} });`)).toThrow();
    expect(
      failing(
        `plugin.addShortcut({ id: "lost", combo: "Mod+E", surface: "missing", onPress() {} });`,
      ),
    ).toThrow("references missing surface missing");
  });

  it("collects a workspace header subtitle and rejects a duplicate id", () => {
    const plugin = evaluatePluginClientBundle(
      "example",
      bundle(`
        function Subtitle() { return null; }
        plugin.addWorkspaceHeaderSubtitle({ id: "project", Component: Subtitle });
      `),
    );
    expect(plugin.workspaceHeaderSubtitles?.map((subtitle) => subtitle.id)).toEqual(["project"]);
    expect(() =>
      evaluatePluginClientBundle(
        "example",
        bundle(`
          function Subtitle() { return null; }
          plugin.addWorkspaceHeaderSubtitle({ id: "project", Component: Subtitle });
          plugin.addWorkspaceHeaderSubtitle({ id: "project", Component: Subtitle });
        `),
      ),
    ).toThrow("Duplicate workspace header subtitle: project");
  });

  it("collects a composer stop button and rejects a duplicate id", () => {
    const plugin = evaluatePluginClientBundle(
      "example",
      bundle(`
        function Face() { return null; }
        plugin.addComposerStopButton({ id: "orb", Component: Face });
      `),
    );
    expect(plugin.composerStopButtons?.map((button) => button.id)).toEqual(["orb"]);
    expect(() =>
      evaluatePluginClientBundle(
        "example",
        bundle(`
          function Face() { return null; }
          plugin.addComposerStopButton({ id: "orb", Component: Face });
          plugin.addComposerStopButton({ id: "orb", Component: Face });
        `),
      ),
    ).toThrow("Duplicate composer stop button: orb");
  });

  it("rejects a bundle without a default contribution function", () => {
    expect(() => evaluatePluginClientBundle("example", `(function() { return {}; })`)).toThrow(
      "must default export a function",
    );
  });

  it("requires a cleanup function", () => {
    expect(() =>
      evaluatePluginClientBundle("example", `(function() { return { default: function() {} }; })`),
    ).toThrow("must return a cleanup function");
  });

  it("provides the host Icon component through @getpaseo/plugin/client/react-native", () => {
    const plugin = evaluatePluginClientBundle(
      "example",
      `(function(require) {
        const { Icon } = require("@getpaseo/plugin/client/react-native");
        const module = { exports: {} };
        module.exports.default = function(plugin) {
          plugin.addSurface("main", function Surface() {
            return Icon({ name: "Settings", size: 18, color: "#123456" });
          });
          return function() {};
        };
        return module.exports;
      })`,
    );

    const Component = plugin.surfaces[0]?.Component;
    expect(Component).toBeTypeOf("function");
    const element = (Component as (props: never) => { props: unknown })({} as never);
    expect(element).toMatchObject({ props: { size: 18, color: "#123456" } });
  });

  it("provides Paseo UI through @getpaseo/plugin/client/react-native", () => {
    const plugin = evaluatePluginClientBundle(
      "example",
      `(function(require) {
        const { Icon, Modal, useToast } = require("@getpaseo/plugin/client/react-native");
        const module = { exports: {} };
        module.exports.default = function(plugin) {
          if (typeof Icon !== "function" || typeof Modal !== "function" || typeof Modal.Content !== "function" || typeof useToast !== "function") {
            throw new Error("React Native plugin UI is incomplete");
          }
          plugin.addSurface("main", function Surface() { return null; });
          return function() {};
        };
        return module.exports;
      })`,
    );

    expect(plugin.surfaces.map((surface) => surface.id)).toEqual(["main"]);
  });

  it("keeps shared and client runtime exports separate", () => {
    expect(() =>
      evaluatePluginClientBundle(
        "example",
        `(function(require) {
      const shared = require("@getpaseo/plugin");
      const client = require("@getpaseo/plugin/client");
      const ui = require("@getpaseo/plugin/client/ui");
      for (const name of ["ExternalLink", "Button", "Markdown", "usePrimaryModifier", "tokenizeCode", "SyntaxToken"]) {
        if (typeof ui[name] !== "function") throw new Error(name);
      }
      for (const name of ["usePaseo", "useRpc", "useSettings", "useAgent", "useWorkspace", "openExternalUrl"]) {
        if (name in shared || typeof client[name] !== "function") throw new Error(name);
      }
      if ("Icon" in shared || typeof shared.PluginAttachmentItemSchema.parse !== "function") throw new Error("shared exports");
      return { default() { return () => {}; } };
    })`,
      ),
    ).not.toThrow();
  });

  it.each([
    "@getpaseo/plugin/server",
    "@getpaseo/plugin/server/provider",
    "@getpaseo/plugin/server/acp",
    "@getpaseo/plugin/client/host",
    "@getpaseo/plugin/react-native",
    "@getpaseo/plugin/ui",
    "@getpaseo/plugin/host",
    "@paseo/plugin",
  ])("rejects %s in the client loader", (specifier) => {
    expect(() =>
      evaluatePluginClientBundle(
        "example",
        `(function(require) { require("${specifier}"); return {}; })`,
      ),
    ).toThrow("not available in plugin client code");
  });

  it("resolves shared RPC helpers from @getpaseo/plugin", () => {
    const plugin = evaluatePluginClientBundle(
      "example",
      `(function(require) {
        const { defineRpc, defineAttachmentSource } = require("@getpaseo/plugin");
        const search = defineRpc({ name: "issues.search", input: {}, output: {} });
        const module = { exports: {} };
        module.exports.default = function(plugin) {
          plugin.addAttachmentSource(defineAttachmentSource({
            id: "issues",
            title: "Issue",
            icon: "CircleDot",
            pickerTitle: "Attach issue",
            searchPlaceholder: "Search",
            search,
          }));
          return function() {};
        };
        return module.exports;
      })`,
    );

    expect(plugin.attachmentSources.map((source) => source.search?.name)).toEqual([
      "issues.search",
    ]);
  });

  it("accepts a chip-only attachment source with onOpen and no search", () => {
    const evaluated = evaluatePluginClientBundle(
      "cite-source",
      bundle(`
      plugin.addAttachmentSource({ id: "quote", title: "Quote", icon: "Blocks", onOpen() {} });
    `),
    );
    expect(evaluated.attachmentSources).toEqual([
      { id: "quote", title: "Quote", icon: "Blocks", onOpen: expect.any(Function) },
    ]);
    expect(() =>
      evaluatePluginClientBundle(
        "empty-source",
        bundle(`plugin.addAttachmentSource({ id: "quote", title: "Quote", icon: "Blocks" });`),
      ),
    ).toThrow("Attachment source quote needs a search RPC or onOpen");
  });

  it("forwards a timeline reveal and rejects one with no message", () => {
    const revealTimelinePassage = vi.fn();
    const reveal = (fields: string) =>
      runPluginClientBundle(
        "cite-reveal",
        bundle(`plugin.revealTimelinePassage({ agentId: "agent-1", ${fields} });`),
        { ...runtime, revealTimelinePassage } as PluginClientRuntime,
      );

    reveal(`messageId: "m1", text: "  quoted  "`);
    expect(revealTimelinePassage).toHaveBeenCalledWith({
      agentId: "agent-1",
      messageId: "m1",
      text: "quoted",
    });
    expect(() => reveal(`messageId: " "`)).toThrow("revealTimelinePassage needs a messageId");
  });

  it("rejects modules that are not part of the client runtime", () => {
    expect(() =>
      evaluatePluginClientBundle(
        "example",
        `(function(require) {
          require("fs");
          const module = { exports: {} };
          module.exports.default = function() { return function() {}; };
          return module.exports;
        })`,
      ),
    ).toThrow('Module "fs" is not available in plugin client code');
  });

  it("does not publish partial contributions when setup fails", () => {
    expect(() =>
      evaluatePluginClientBundle(
        "example",
        `(function() { return { default: function(plugin) {
          plugin.addSurface("main", function() { return null; });
          throw new Error("setup exploded");
        } }; })`,
      ),
    ).toThrow("setup exploded");
  });
});

it("binds imported getters to each originating installation across delayed callbacks", async () => {
  const calls: string[] = [];
  const hostRuntime = (installation: string): PluginClientRuntime => ({
    ...runtime,
    hosts: {
      getSnapshot: () => [],
      subscribe: () => () => {},
      getPaseoClient(serverId) {
        calls.push(`${installation}/${serverId}`);
        return runtime.paseo;
      },
    },
  });
  const source = bundle(`
    const { getPaseoClient } = require("@getpaseo/plugin/client");
    getPaseoClient("entry-host");
    plugin.addCommandCenterItem({
      id: "read", title: "Read", icon: "Server", context: "global",
      onSelect: async () => { await Promise.resolve(); getPaseoClient("target-host"); },
    });
  `);
  const first = runPluginClientBundle("same-id", source, hostRuntime("first"));
  const second = runPluginClientBundle("same-id", source, hostRuntime("second"));
  // Both callbacks run after the second bundle has evaluated.
  await first.commandCenterItems[0].onSelect({} as never);
  await second.commandCenterItems[0].onSelect({} as never);
  expect(calls).toEqual([
    "first/entry-host",
    "second/entry-host",
    "first/target-host",
    "second/target-host",
  ]);
  await first.cleanup();
  await second.cleanup();
});
