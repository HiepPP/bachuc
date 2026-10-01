import { pluginSettingsKey } from "./settings/use-settings";
import { useEffect } from "react";
import type { DaemonClient } from "@getpaseo/client/internal/daemon-client";
import { useHostFeature } from "@/runtime/host-features";
import { useHostRuntimeIsConnected } from "@/runtime/host-runtime";
import { useActiveServerId } from "@/hosts/use-visible-hosts";
import { pluginRegistry } from "./registry";

export function PluginCatalogSync({
  serverId,
  client,
}: {
  serverId: string;
  client: DaemonClient;
}) {
  const connected = useHostRuntimeIsConnected(serverId);
  const supported = useHostFeature(serverId, "plugins");
  // With one host picked, only that host's plugin client code runs. Code from another host,
  // such as an older plugin build that edits the DOM, cannot reach the window.
  const activeServerId = useActiveServerId();
  const hidden = activeServerId !== null && activeServerId !== serverId;

  useEffect(() => {
    let cancelled = false;
    let refreshQueue = Promise.resolve();
    if (!supported) {
      pluginRegistry.removeHost(serverId);
      return;
    }
    if (!connected || hidden) {
      pluginRegistry.removeHost(serverId);
      return;
    }
    const refresh = (replacePluginId?: string) => {
      refreshQueue = refreshQueue.then(() =>
        client
          .getPluginCatalog()
          .then((catalog) => {
            if (!cancelled) {
              pluginRegistry.installCatalog(serverId, catalog, {
                replacePluginId,
                client,
              });
            }
            return undefined;
          })
          .catch((error) => {
            if (!cancelled) {
              console.warn(`[Plugins] Failed to load catalog for ${serverId}`, error);
            }
            return undefined;
          }),
      );
      return refreshQueue;
    };
    const observation = client.observeEvents([
      "status.plugin_catalog_changed",
      "status.plugin_settings_changed",
    ]);
    observation.subscribe({
      snapshot: () => {
        void refresh();
      },
      update: (message) => {
        if (message.type !== "status") return;
        if (message.payload.status === "plugin_settings_changed") {
          const { pluginId, settingsId } = message.payload;
          if (typeof settingsId === "string") {
            const plugin = pluginRegistry
              .getSnapshot()
              .find((item) => item.serverId === serverId && item.id === pluginId);
            void plugin?.queryClient.invalidateQueries({ queryKey: pluginSettingsKey(settingsId) });
          }
        }
        if (message.payload.status === "plugin_catalog_changed") {
          const pluginId = message.payload.pluginId;
          if (typeof pluginId === "string") void refresh(pluginId);
        }
      },
    });
    return () => {
      cancelled = true;
      void observation
        .release()
        .catch((error) => console.warn("[Plugins] Failed to release catalog", error));
    };
  }, [client, connected, hidden, serverId, supported]);

  useEffect(() => () => pluginRegistry.removeHost(serverId), [serverId]);
  return null;
}
