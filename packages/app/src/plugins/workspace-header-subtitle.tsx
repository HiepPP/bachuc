import { useCallback, useMemo, useSyncExternalStore, type ReactNode } from "react";
import { Platform } from "react-native";
import { withUnistyles } from "react-native-unistyles";
import type { PluginTheme } from "@getpaseo/plugin";
import type { PluginHostProps } from "@getpaseo/plugin/client";
import { useIsCompactFormFactor } from "@/constants/layout";
import { useHostRuntimeClient, useHosts } from "@/runtime/host-runtime";
import { useWorkspace } from "@/stores/session-store-hooks";
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

interface SubtitleProps {
  serverId: string;
  workspaceId: string;
  projectDisplayName: string;
  /** Paseo's own project name, shown when no plugin on this host replaces it or it fails. */
  children: ReactNode;
  theme: PluginTheme;
}

function Subtitle({ serverId, workspaceId, projectDisplayName, children, theme }: SubtitleProps) {
  const installed = useSyncExternalStore(
    pluginRegistry.subscribe,
    pluginRegistry.getSnapshot,
    pluginRegistry.getSnapshot,
  );
  // The first contribution on the workspace's host wins.
  const plugin = installed.find(
    (candidate) => candidate.serverId === serverId && candidate.workspaceHeaderSubtitles?.length,
  );
  const contribution = plugin?.workspaceHeaderSubtitles?.[0];
  const client = useHostRuntimeClient(serverId);
  const workspace = useWorkspace(serverId, workspaceId);
  const projectId = workspace?.projectId;
  const hosts = useHosts();
  const compact = useIsCompactFormFactor();
  const label = hosts.find((host) => host.serverId === serverId)?.label ?? serverId;
  const host = useMemo(() => ({ id: serverId, label }), [label, serverId]);
  const layout = useMemo(() => ({ compact, platform: resolvePlatform() }), [compact]);
  const renderFallback = useCallback(() => children, [children]);
  if (!plugin || !contribution || !client || !projectId) return children;
  const Component = contribution.Component;
  return (
    <SurfaceErrorBoundary installation={plugin} Surface={Component} renderError={renderFallback}>
      <PluginRuntimeBoundary plugin={plugin} client={client}>
        <Component
          theme={theme}
          host={host}
          layout={layout}
          workspaceId={workspaceId}
          projectId={projectId}
          projectDisplayName={projectDisplayName}
          projectRootPath={workspace?.projectRootPath}
        />
      </PluginRuntimeBoundary>
    </SurfaceErrorBoundary>
  );
}

const pluginThemeMapping = (theme: Theme) => ({ theme: toPluginTheme(theme) });
const ThemedSubtitle = withUnistyles(Subtitle);

/** The workspace header's project name, or the plugin component that replaces it. */
export function PluginWorkspaceHeaderSubtitle(props: Omit<SubtitleProps, "theme">) {
  return <ThemedSubtitle {...props} uniProps={pluginThemeMapping} />;
}
