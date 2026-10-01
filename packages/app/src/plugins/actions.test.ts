import { QueryClient } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";

const push = vi.fn();
vi.mock("expo-router", () => ({ router: { push: (href: string) => push(href) } }));

const { createPluginCapabilities } = await import("./actions");
const { createPluginNavigation } = await import("./navigation");
import type { InstalledPlugin } from "./types";

function plugin(): InstalledPlugin {
  return {
    lifetime: new AbortController(),
    id: "next",
    serverId: "host-a",
    clientBundle: "bundle",
    queryClient: new QueryClient(),
    cleanup: () => undefined,
    settingsScreens: [],
    surfaces: [{ id: "own", Component: () => null }],
    sidebarItems: [],
    workspacePanels: [],
    commandCenterItems: [],
    clientSlashCommands: [],
    attachmentSources: [],
    themes: [],
    timelineTransformers: [],
    timelineRenderers: [],
  };
}

function capabilities() {
  const installation = plugin();
  return createPluginCapabilities(
    installation,
    { paseo: {} as never, invoke: vi.fn() } as never,
    createPluginNavigation({ serverId: installation.serverId, workspaceId: null }),
  );
}

describe("plugin surface navigation", () => {
  beforeEach(() => push.mockReset());

  it("opens its own surface and rejects a missing one", () => {
    const actions = capabilities();
    actions.openSurface("own");
    expect(push).toHaveBeenCalledTimes(1);
    expect(() => actions.openSurface("missing")).toThrow("Plugin surface is unavailable");
  });

  it("opens the surface on the active host when one is picked", async () => {
    const { useActiveHostStore } = await import("@/stores/active-host-store");
    useActiveHostStore.setState({ activeServerId: "host-c" });
    capabilities().openSurface("own");
    useActiveHostStore.setState({ activeServerId: null });
    expect(String(push.mock.calls[0][0])).toContain("host-c");
  });

  it("opens another plugin's surface on its own host or a given host", () => {
    const actions = capabilities();
    actions.openSurface("board", { pluginId: "board" });
    actions.openSurface("board", { pluginId: "board", serverId: "host-b" });
    expect(push.mock.calls.map(([href]) => String(href))).toEqual([
      expect.stringContaining("host-a"),
      expect.stringContaining("host-b"),
    ]);
    expect(String(push.mock.calls[0][0])).toContain("board");
  });
});
