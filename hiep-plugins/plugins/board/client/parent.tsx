import type { PluginClientContext, PluginSurfaceProps } from "@getpaseo/plugin/client";
import { useEffect, useSyncExternalStore } from "react";
import { Text } from "react-native";

// Navigation is supplied to surfaces, not composer action callbacks.
export function installParentNavigation(client: PluginClientContext) {
  let target: { agentId: string } | null = null;
  const listeners = new Set<() => void>();
  const subscribe = (listener: () => void) => {
    listeners.add(listener);
    return () => {
      listeners.delete(listener);
    };
  };
  const snapshot = () => target;
  function Parent({ navigation, theme }: PluginSurfaceProps) {
    const request = useSyncExternalStore(subscribe, snapshot);
    useEffect(() => {
      if (request && navigation) navigation.openAgent(request);
    }, [request, navigation]);
    return (
      <Text style={{ color: theme.colors.foregroundMuted }}>
        {navigation ? "Opening parent…" : "Parent navigation is unavailable in this client."}
      </Text>
    );
  }
  const cleanup = client.addSurface("parent", Parent);
  return {
    open(agentId: string) {
      target = { agentId };
      client.openSurface("parent");
      for (const listener of listeners) listener();
    },
    cleanup,
  };
}
