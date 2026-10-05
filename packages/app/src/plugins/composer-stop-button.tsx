import { useCallback, useMemo, useSyncExternalStore, type ReactNode } from "react";
import { Platform } from "react-native";
import { withUnistyles } from "react-native-unistyles";
import type { PluginTheme } from "@getpaseo/plugin";
import type { PluginHostProps } from "@getpaseo/plugin/client";
import { useIsCompactFormFactor } from "@/constants/layout";
import { useHostRuntimeClient, useHosts } from "@/runtime/host-runtime";
import type { Theme } from "@/styles/theme";
import { pluginRegistry } from "./registry";
import { PluginRuntimeBoundary } from "./runtime-boundary";
import { SurfaceErrorBoundary } from "./surface-error-boundary";
import { toPluginTheme } from "./theme";

function resolvePlatform(): PluginHostProps["layout"]["platform"] {
  if (Platform.OS === "ios") return "ios";
  if (Platform.OS === "android") return "android";
  return "web";
}

interface StopButtonFaceProps {
  serverId: string;
  agentId: string;
  size: number;
  iconSize: number;
  cancelling: boolean;
  /** Paseo's own face, shown when no plugin on this host replaces it or it fails. */
  children: ReactNode;
  theme: PluginTheme;
}

function StopButtonFace({
  serverId,
  agentId,
  size,
  iconSize,
  cancelling,
  children,
  theme,
}: StopButtonFaceProps) {
  const installed = useSyncExternalStore(
    pluginRegistry.subscribe,
    pluginRegistry.getSnapshot,
    pluginRegistry.getSnapshot,
  );
  // The first contribution on the agent's host wins.
  const plugin = installed.find(
    (candidate) => candidate.serverId === serverId && candidate.composerStopButtons?.length,
  );
  const contribution = plugin?.composerStopButtons?.[0];
  const client = useHostRuntimeClient(serverId);
  const hosts = useHosts();
  const compact = useIsCompactFormFactor();
  const label = hosts.find((host) => host.serverId === serverId)?.label ?? serverId;
  const host = useMemo(() => ({ id: serverId, label }), [label, serverId]);
  const layout = useMemo(() => ({ compact, platform: resolvePlatform() }), [compact]);
  const renderFallback = useCallback(() => children, [children]);
  if (!plugin || !contribution || !client) return children;
  const Component = contribution.Component;
  return (
    <SurfaceErrorBoundary installation={plugin} Surface={Component} renderError={renderFallback}>
      <PluginRuntimeBoundary plugin={plugin} client={client}>
        <Component
          theme={theme}
          host={host}
          layout={layout}
          agentId={agentId}
          size={size}
          iconSize={iconSize}
          cancelling={cancelling}
        />
      </PluginRuntimeBoundary>
    </SurfaceErrorBoundary>
  );
}

const pluginThemeMapping = (theme: Theme) => ({ theme: toPluginTheme(theme) });
const ThemedStopButtonFace = withUnistyles(StopButtonFace);

/** The composer stop button's face, or the plugin component that replaces it. */
export function PluginComposerStopButtonFace(props: Omit<StopButtonFaceProps, "theme">) {
  return <ThemedStopButtonFace {...props} uniProps={pluginThemeMapping} />;
}
