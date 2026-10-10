import type { NativeStackNavigationOptions } from "@react-navigation/native-stack";
import { Redirect, Stack, useLocalSearchParams } from "expo-router";
import { useHostRuntimeBootstrapState } from "@/app/_layout";
import { useIsCompactFormFactor } from "@/constants/layout";
import { HostRouteProvider } from "@/navigation/host-route-context";
import { resolveStartupRoute } from "@/navigation/host-runtime-bootstrap";
import { ThemedStack } from "@/navigation/themed-stack";
import { isPluginOverlayRoute, PLUGIN_SURFACE_SCREEN_NAME } from "@/plugins/routes";
import { useHostRegistryStatus, useHosts } from "@/runtime/host-runtime";

const HOST_STACK_SCREEN_OPTIONS = {
  headerShown: false,
  animation: "none" as const,
};

const AGENT_SCREEN_OPTIONS = { gestureEnabled: false };

// A plugin surface opened with `?presentation=overlay` stacks over the previous screen. The
// transparent content lets `PluginSurfaceScreen` draw its own backdrop and card.
const PLUGIN_OVERLAY_SCREEN_OPTIONS: NativeStackNavigationOptions = {
  presentation: "transparentModal",
  contentStyle: { backgroundColor: "transparent" },
};

function pluginSurfaceScreenOptions({
  route,
  navigation,
}: {
  route: { key: string };
  navigation: { getState(): Parameters<typeof isPluginOverlayRoute>[0] };
}): NativeStackNavigationOptions {
  return isPluginOverlayRoute(navigation.getState(), route.key)
    ? PLUGIN_OVERLAY_SCREEN_OPTIONS
    : {};
}

export default function HostRouteLayout() {
  return <KnownHostRoute />;
}

function KnownHostRoute() {
  const params = useLocalSearchParams<{ serverId?: string | string[] }>();
  const hosts = useHosts();
  const hostRegistryStatus = useHostRegistryStatus();
  const bootstrapState = useHostRuntimeBootstrapState();
  // Compact layouts show every plugin surface as a page, so they get no overlay options.
  const compact = useIsCompactFormFactor();
  const routeServerId = typeof params.serverId === "string" ? params.serverId : null;
  const startupRoute = resolveStartupRoute({
    route: { kind: "host", serverId: routeServerId },
    startupBlocker: bootstrapState.startupBlocker,
    hostRegistryStatus,
    hosts,
  });

  if (startupRoute.kind === "redirect") {
    return <Redirect href={startupRoute.href} />;
  }

  const stack = (
    <ThemedStack screenOptions={HOST_STACK_SCREEN_OPTIONS}>
      <Stack.Screen name="index" />
      <Stack.Screen name="workspace/[workspaceId]/index" />
      <Stack.Screen name="agent/[agentId]" options={AGENT_SCREEN_OPTIONS} />
      <Stack.Screen name="sessions" />
      <Stack.Screen name="open-project" />
      <Stack.Screen name="settings" />
      <Stack.Screen name="plugin/[pluginId]/[surfaceId]" />
      <Stack.Screen
        name={PLUGIN_SURFACE_SCREEN_NAME}
        options={compact ? undefined : pluginSurfaceScreenOptions}
      />
    </ThemedStack>
  );

  if (!routeServerId) {
    return stack;
  }

  return <HostRouteProvider serverId={routeServerId}>{stack}</HostRouteProvider>;
}
