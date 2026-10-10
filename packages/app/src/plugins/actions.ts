import { callPluginRpc } from "@getpaseo/plugin/client/host";
import type {
  PluginAgentCommandContext,
  PluginCommandCapabilities,
  PluginPanelLocation,
  PluginWorkspaceCommandContext,
} from "@getpaseo/plugin/client";
import type { PluginClientStateSource } from "@getpaseo/plugin/client/host";
import { resolvePluginPanelOpenLocation } from "./workspace-panels/locations";
import type { PluginSurfaceRuntime } from "./surface-runtime";
import type { InstalledPlugin } from "./types";
import { createPluginNavigation } from "./navigation";
import type { PluginSurfacePresentation } from "./routes";
import { useActiveHostStore } from "@/stores/active-host-store";
import { getHostRuntimeStore } from "@/runtime/host-runtime";

export interface PluginNavigation {
  openSettings(pluginId: string, screenId: string): void;
  openSurface(pluginId: string, surfaceId: string, presentation?: PluginSurfacePresentation): void;
  openWorkspacePanel(pluginId: string, panelId: string, location: PluginPanelLocation): void;
  openAgentPanel(
    pluginId: string,
    panelId: string,
    agentId: string,
    location: PluginPanelLocation,
  ): void;
}

export function createPluginCapabilities(
  plugin: InstalledPlugin,
  runtime: PluginSurfaceRuntime,
  navigation: PluginNavigation,
): PluginCommandCapabilities {
  return {
    paseo: runtime.paseo,
    rpc: (contract, input, options) =>
      callPluginRpc(
        contract,
        async (method, value) => {
          if (plugin.lifetime.signal.aborted) throw new Error("Plugin has stopped");
          const serverId = options?.serverId ?? plugin.serverId;
          if (serverId === plugin.serverId) return runtime.invoke(method, value);
          const hosts = getHostRuntimeStore();
          if (!hosts.getHosts().some((host) => host.serverId === serverId)) {
            throw new Error(`Unknown Paseo host: ${serverId}`);
          }
          const target = hosts.getSnapshot(serverId);
          if (target?.connectionStatus !== "online" || !target.client) {
            throw new Error(`Paseo host is disconnected: ${serverId}`);
          }
          return target.client.invokePluginRpc(plugin.id, method, value);
        },
        input,
      ),
    openSettings(screenId) {
      if (!plugin.settingsScreens.some((screen) => screen.id === screenId))
        throw new Error(`Plugin settings screen is unavailable: ${screenId}`);
      navigation.openSettings(plugin.id, screenId);
    },
    openSurface(surfaceId, options) {
      const pluginId = options?.pluginId?.trim() || plugin.id;
      // Plugin pages follow the active host, like plugin sidebar rows.
      const serverId =
        options?.serverId?.trim() ||
        useActiveHostStore.getState().activeServerId ||
        plugin.serverId;
      if (pluginId === plugin.id && serverId === plugin.serverId) {
        if (!plugin.surfaces.some((surface) => surface.id === surfaceId)) {
          throw new Error(`Plugin surface is unavailable: ${surfaceId}`);
        }
        navigation.openSurface(plugin.id, surfaceId, options?.presentation);
        return;
      }
      // Another plugin or host: the surface route reports a missing surface itself.
      createPluginNavigation({ serverId, workspaceId: null }).openSurface(
        pluginId,
        surfaceId,
        options?.presentation,
      );
    },
  };
}

export function createPluginAgentActionContext(input: {
  plugin: InstalledPlugin;
  runtime: PluginSurfaceRuntime;
  navigation: PluginNavigation;
  state: PluginClientStateSource;
  workspaceId: string;
  agentId: string;
}): PluginAgentCommandContext | null {
  const { plugin, runtime, navigation, state, workspaceId, agentId } = input;
  const workspace = state.getWorkspace(workspaceId);
  const agent = state.getAgent(agentId);
  if (!workspace || !agent || agent.workspaceId !== workspace.id) return null;
  return {
    context: "agent",
    ...createPluginCapabilities(plugin, runtime, navigation),
    workspace,
    agent,
    openPanel(panelId, options) {
      const panel = plugin.workspacePanels.find((candidate) => candidate.id === panelId);
      if (!panel) throw new Error(`Workspace panel is unavailable: ${panelId}`);
      const location = resolvePluginPanelOpenLocation(panel, options?.location);
      if (panel.context === "workspace") {
        navigation.openWorkspacePanel(plugin.id, panelId, location);
        return;
      }
      navigation.openAgentPanel(plugin.id, panelId, agent.id, location);
    },
  };
}

export function createPluginWorkspaceActionContext(input: {
  plugin: InstalledPlugin;
  runtime: PluginSurfaceRuntime;
  navigation: PluginNavigation;
  state: PluginClientStateSource;
  workspaceId: string;
}): PluginWorkspaceCommandContext | null {
  const { plugin, runtime, navigation, state, workspaceId } = input;
  const workspace = state.getWorkspace(workspaceId);
  if (!workspace) return null;
  return {
    context: "workspace",
    ...createPluginCapabilities(plugin, runtime, navigation),
    workspace,
    openPanel(panelId, options) {
      const panel = plugin.workspacePanels.find(
        (candidate) => candidate.id === panelId && candidate.context === "workspace",
      );
      if (!panel) throw new Error(`Workspace panel is unavailable: ${panelId}`);
      const location = resolvePluginPanelOpenLocation(panel, options?.location);
      navigation.openWorkspacePanel(plugin.id, panelId, location);
    },
  };
}
