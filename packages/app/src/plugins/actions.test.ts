import { QueryClient } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { defineRpc } from "@getpaseo/plugin";
import { z } from "zod";

const hosts = vi.hoisted(() => ({ getHosts: vi.fn(), getSnapshot: vi.fn() }));
vi.mock("@/runtime/host-runtime", () => ({ getHostRuntimeStore: () => hosts }));

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

describe("plugin RPC host targeting", () => {
  const contract = defineRpc({
    name: "board.test",
    input: z.object({ id: z.string() }),
    output: z.object({ host: z.string() }),
  });

  function setup() {
    const installation = plugin();
    const local = vi.fn(async () => ({ host: "host-a" }));
    const remote = vi.fn(async () => ({ host: "host-b" }));
    hosts.getHosts.mockReturnValue([{ serverId: "host-b" }]);
    hosts.getSnapshot.mockReturnValue({
      connectionStatus: "online",
      client: { invokePluginRpc: remote },
    });
    const actions = createPluginCapabilities(
      installation,
      { paseo: {} as never, invoke: local },
      createPluginNavigation({ serverId: installation.serverId, workspaceId: null }),
    );
    return { installation, local, remote, actions };
  }

  it("routes equal IDs to the requested installation and keeps the default local", async () => {
    const h = setup();
    await expect(h.actions.rpc(contract, { id: "same" })).resolves.toEqual({ host: "host-a" });
    await expect(h.actions.rpc(contract, { id: "same" }, { serverId: "host-b" })).resolves.toEqual({
      host: "host-b",
    });
    expect(h.local).toHaveBeenCalledExactlyOnceWith("board.test", { id: "same" });
    expect(h.remote).toHaveBeenCalledExactlyOnceWith("next", "board.test", { id: "same" });
  });

  it("never falls back for offline, removed, or missing remote installations", async () => {
    const h = setup();
    hosts.getSnapshot.mockReturnValue({ connectionStatus: "offline", client: null });
    await expect(h.actions.rpc(contract, { id: "same" }, { serverId: "host-b" })).rejects.toThrow(
      "disconnected",
    );
    hosts.getHosts.mockReturnValue([]);
    await expect(h.actions.rpc(contract, { id: "same" }, { serverId: "host-b" })).rejects.toThrow(
      "Unknown Paseo host",
    );
    hosts.getHosts.mockReturnValue([{ serverId: "host-b" }]);
    hosts.getSnapshot.mockReturnValue({
      connectionStatus: "online",
      client: { invokePluginRpc: h.remote },
    });
    h.remote.mockRejectedValueOnce(new Error("Plugin is unavailable"));
    await expect(h.actions.rpc(contract, { id: "same" }, { serverId: "host-b" })).rejects.toThrow(
      "Plugin is unavailable",
    );
    expect(h.local).not.toHaveBeenCalled();
  });

  it("validates remote responses and rejects retained callbacks after plugin unload", async () => {
    const h = setup();
    h.remote.mockResolvedValueOnce({ host: 12 } as never);
    await expect(
      h.actions.rpc(contract, { id: "same" }, { serverId: "host-b" }),
    ).rejects.toBeInstanceOf(z.ZodError);
    h.installation.lifetime.abort();
    await expect(h.actions.rpc(contract, { id: "same" }, { serverId: "host-b" })).rejects.toThrow(
      "Plugin has stopped",
    );
    expect(h.remote).toHaveBeenCalledOnce();
    expect(h.local).not.toHaveBeenCalled();
  });
});
