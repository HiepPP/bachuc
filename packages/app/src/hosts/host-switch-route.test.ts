import { QueryClient } from "@tanstack/react-query";
import { describe, expect, it } from "vitest";
import type { InstalledPlugin } from "@/plugins/types";
import { resolveHostSwitchRoute } from "./host-switch-route";

function board(serverId: string): InstalledPlugin {
  return {
    id: "board",
    serverId,
    clientBundle: serverId,
    lifetime: new AbortController(),
    queryClient: new QueryClient(),
    cleanup: () => undefined,
    settingsScreens: [],
    surfaces: [{ id: "board", Component: () => null }],
    sidebarItems: [{ id: "board", surface: "board", title: "Board", icon: "Blocks" }],
    workspacePanels: [],
    commandCenterItems: [],
    clientSlashCommands: [],
    attachmentSources: [],
    themes: [],
    timelineTransformers: [],
    timelineRenderers: [],
  };
}

const BOARD_ON_A = "/h/a/plugin/board/sidebar/board";

function resolve(
  pathname: string,
  targetServerId: string,
  installations: InstalledPlugin[] = [],
  lastRouteByServerId: Record<string, string> = {},
) {
  return resolveHostSwitchRoute({ pathname, targetServerId, installations, lastRouteByServerId });
}

describe("resolveHostSwitchRoute", () => {
  it("returns to the target host's last screen before anything else", () => {
    const last = { b: "/h/b/workspace/w9" };
    expect(resolve("/h/a/workspace/w1", "b", [], last)).toBe("/h/b/workspace/w9");
    expect(resolve(BOARD_ON_A, "b", [board("a"), board("b")], last)).toBe("/h/b/workspace/w9");
    expect(resolve("/open-project", "b", [], last)).toBe("/h/b/workspace/w9");
  });

  it("keeps a plugin page when the target has no last screen and has the plugin", () => {
    expect(resolve(BOARD_ON_A, "b", [board("a"), board("b")])).toBe(
      "/h/b/plugin/board/sidebar/board",
    );
  });

  it("falls back to the target's project page when it lacks the plugin", () => {
    expect(resolve(BOARD_ON_A, "b", [board("a")])).toBe("/h/b/open-project");
  });

  it("opens a concrete page, not the bare host index, from another host page", () => {
    expect(resolve("/h/a/workspace/w1", "b")).toBe("/h/b/open-project");
  });

  it("stays on pages that already follow the active host", () => {
    for (const pathname of ["/sessions", "/schedules", "/new", "/settings/general"]) {
      expect(resolve(pathname, "b", [], { b: "/h/b/workspace/w9" })).toBeNull();
    }
  });

  it("opens the host index from the landing page when there is no last screen", () => {
    expect(resolve("/open-project", "b")).toBe("/h/b");
  });

  it("stays when the route is already on the target host", () => {
    expect(resolve(BOARD_ON_A, "a", [board("a")])).toBeNull();
  });
});
