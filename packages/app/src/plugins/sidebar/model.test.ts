import { QueryClient } from "@tanstack/react-query";
import { describe, expect, it, vi } from "vitest";
import type { SidebarProjectEntry } from "@/hooks/use-sidebar-workspaces-list";
import type { InstalledPlugin } from "../types";
import {
  hiddenProjectViewKeys,
  pluginProjectMenuItems,
  pluginSidebarTitle,
  selectSidebarPlugins,
} from "./model";

function plugin(serverId: string, extra: Partial<InstalledPlugin> = {}): InstalledPlugin {
  return {
    lifetime: new AbortController(),
    id: "spaces",
    serverId,
    clientBundle: "bundle",
    queryClient: new QueryClient(),
    cleanup: () => undefined,
    settingsScreens: [],
    surfaces: [],
    sidebarItems: [],
    workspacePanels: [],
    commandCenterItems: [],
    clientSlashCommands: [],
    attachmentSources: [],
    themes: [],
    timelineTransformers: [],
    timelineRenderers: [],
    ...extra,
  };
}

function project(viewKey: string): SidebarProjectEntry {
  return {
    viewKey,
    projectName: viewKey.toUpperCase(),
    projectKind: "git",
    iconWorkingDir: `/${viewKey}`,
    hosts: [
      {
        serverId: "host-a",
        projectId: `project-${viewKey}`,
        iconWorkingDir: `/${viewKey}`,
        worktreeSupport: "supported",
      },
    ],
    workspaces: [],
  } as SidebarProjectEntry;
}

describe("plugin sidebar", () => {
  it("prefers the active host's installation of each plugin", () => {
    const a = plugin("host-a");
    const b = plugin("host-b");
    expect(selectSidebarPlugins([a, b], "host-b")).toEqual([b]);
    expect(selectSidebarPlugins([a, b], null)).toEqual([a]);
  });

  it("hides projects a filter rejects and ignores a throwing filter", () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    const installed = plugin("host-a", {
      sidebarProjectFilters: [
        {
          id: "space",
          subscribe: () => () => {},
          isVisible: (entry, context) =>
            entry.viewKey !== "b" && context.activeServerId === "host-a",
        },
        {
          id: "broken",
          subscribe: () => () => {},
          isVisible: () => {
            throw new Error("broken");
          },
        },
      ],
    });
    const hidden = hiddenProjectViewKeys([installed], [project("a"), project("b")], {
      activeServerId: "host-a",
    });
    expect([...hidden]).toEqual(["b"]);
    warn.mockRestore();
  });

  it("collects menu items with the project and keys them by plugin", () => {
    const seen: unknown[] = [];
    const installed = plugin("host-a", {
      sidebarProjectMenus: [
        {
          id: "move",
          getItems: (entry) => {
            seen.push(entry);
            return [{ id: "space-2", title: "Move to Space 2", onSelect: () => {} }];
          },
        },
      ],
    });
    const items = pluginProjectMenuItems(
      [installed],
      { viewKey: "a", name: "A", serverIds: ["host-a"], projectIds: ["project-a"] },
      { activeServerId: null },
    );
    expect(items.map(({ key, item }) => [key, item.title])).toEqual([
      ["spaces/move/space-2", "Move to Space 2"],
    ]);
    expect(seen).toEqual([
      { viewKey: "a", name: "A", serverIds: ["host-a"], projectIds: ["project-a"] },
    ]);
  });
});

describe("pluginSidebarTitle", () => {
  it("returns the first non-empty filter title and skips throwing filters", () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => undefined);
    const filter = (id: string, getTitle?: () => string | null) => ({
      id,
      subscribe: () => () => undefined,
      isVisible: () => true,
      ...(getTitle ? { getTitle } : {}),
    });
    const plugins = [
      plugin("host-a", {
        sidebarProjectFilters: [
          filter("plain"),
          filter("broken", () => {
            throw new Error("boom");
          }),
          filter("blank", () => "  "),
          filter("spaces", () => "Personal"),
        ],
      }),
    ];
    expect(pluginSidebarTitle(plugins, { activeServerId: "host-a" })).toBe("Personal");
    expect(pluginSidebarTitle([plugin("host-a")], { activeServerId: "host-a" })).toBeNull();
    warn.mockRestore();
  });
});
