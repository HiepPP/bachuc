import type { PluginTheme } from "@getpaseo/plugin";
import type { PluginHostProps } from "@getpaseo/plugin/client";
import { PluginClientStateProvider } from "@getpaseo/plugin/client/host";
import { PanelRight } from "lucide-react-native";
import { useCallback, useEffect, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { Platform, View } from "react-native";
import { StyleSheet, withUnistyles } from "react-native-unistyles";
import { HeaderToggleButton } from "@/components/headers/header-toggle-button";
import {
  extraMutedIconColorMapping,
  iconButtonChromeGlyphSize,
} from "@/components/ui/icon-button-chrome";
import { useIsCompactFormFactor } from "@/constants/layout";
import { useAppSettings } from "@/hooks/use-settings";
import { getHostProjectId, type HostProjectListItem } from "@/projects/host-projects";
import { useHostRuntimeClient, useHosts } from "@/runtime/host-runtime";
import type { Theme } from "@/styles/theme";
import type { ShortcutKey } from "@/utils/format-shortcut";
import { createPluginClientStateSource } from "../client-state/source";
import { usePluginHostNavigation } from "../host-navigation";
import { useInstalledPlugins } from "../registry";
import { PluginRuntimeBoundary } from "../runtime-boundary";
import { SurfaceErrorBoundary } from "../surface-error-boundary";
import { toPluginTheme } from "../theme";
import type { InstalledPlugin } from "../types";
import {
  newWorkspacePanelKey,
  useNewWorkspaceSidePanelStore,
  type NewWorkspaceSidePanelTarget,
} from "./store";

const SIDE_PANEL_WIDTH = 360;
const NO_SHORTCUT: ShortcutKey[] = [];
const ThemedPanelRight = withUnistyles(PanelRight);
const pluginThemeMapping = (theme: Theme) => ({ theme: toPluginTheme(theme) });

interface PanelEntry {
  key: string;
  plugin: InstalledPlugin;
  panel: NonNullable<InstalledPlugin["newWorkspacePanels"]>[number];
}

function resolvePlatform(): PluginHostProps["layout"]["platform"] {
  if (Platform.OS === "ios") return "ios";
  if (Platform.OS === "android") return "android";
  return "web";
}

function useNewWorkspacePanels(serverId: string): PanelEntry[] {
  const installed = useInstalledPlugins();
  return useMemo(
    () =>
      installed
        .filter((plugin) => plugin.serverId === serverId)
        .flatMap((plugin) =>
          (plugin.newWorkspacePanels ?? []).map((panel) => ({
            key: newWorkspacePanelKey(plugin.id, panel.id),
            plugin,
            panel,
          })),
        ),
    [installed, serverId],
  );
}

/** The side panel shows on wide layouts once a project on a host with panels is chosen. */
function useSidePanelState(target: NewWorkspaceSidePanelTarget | null) {
  const compact = useIsCompactFormFactor();
  const panels = useNewWorkspacePanels(target?.serverId ?? "");
  const { settings, isLoading } = useAppSettings();
  const choice = useNewWorkspaceSidePanelStore((state) => state.open);
  const available = !compact && target !== null && panels.length > 0;
  const open = available && (choice ?? (!isLoading && settings.newWorkspaceSidePanel));
  return { panels, available, open };
}

/** Publishes the chosen project so plugin sidebar rows can open their panel on this screen. */
export function useNewWorkspaceSidePanelTarget(input: {
  serverId: string | null;
  project: HostProjectListItem | null;
  cwd: string | null;
}): NewWorkspaceSidePanelTarget | null {
  const { serverId, project, cwd } = input;
  const projectId = project && serverId ? getHostProjectId(project, serverId) : null;
  const target = useMemo(
    () => (serverId && projectId && cwd ? { serverId, projectId, cwd } : null),
    [serverId, projectId, cwd],
  );
  useEffect(() => useNewWorkspaceSidePanelStore.getState().setTarget(target), [target]);
  useEffect(() => () => useNewWorkspaceSidePanelStore.getState().reset(), []);
  return target;
}

export function NewWorkspaceSidePanelToggle({
  target,
}: {
  target: NewWorkspaceSidePanelTarget | null;
}) {
  const { t } = useTranslation();
  const { available, open } = useSidePanelState(target);
  const toggle = useCallback(() => {
    const store = useNewWorkspaceSidePanelStore.getState();
    if (open) store.hide();
    else store.show();
  }, [open]);
  const accessibilityState = useMemo(() => ({ expanded: open }), [open]);
  if (!available) return null;
  const label = open ? t("newWorkspace.sidePanel.hide") : t("newWorkspace.sidePanel.show");
  return (
    <HeaderToggleButton
      testID="new-workspace-side-panel-toggle"
      onPress={toggle}
      tooltipLabel={label}
      tooltipKeys={NO_SHORTCUT}
      tooltipSide="left"
      accessible
      accessibilityRole="button"
      accessibilityLabel={label}
      accessibilityState={accessibilityState}
    >
      <ThemedPanelRight
        size={iconButtonChromeGlyphSize("large")}
        strokeWidth={1.5}
        uniProps={extraMutedIconColorMapping}
      />
    </HeaderToggleButton>
  );
}

/** Shows one panel: the one a sidebar row asked for, or the host's first. */
export function NewWorkspaceSidePanel({ target }: { target: NewWorkspaceSidePanelTarget | null }) {
  const { panels, open } = useSidePanelState(target);
  const selected = useNewWorkspaceSidePanelStore((state) => state.selected);
  if (!open || !target) return null;
  const active = panels.find((entry) => entry.key === selected) ?? panels[0];
  return (
    <View style={styles.panel} testID="new-workspace-side-panel">
      <ThemedPanelBody
        key={active.key}
        entry={active}
        projectId={target.projectId}
        cwd={target.cwd}
        uniProps={pluginThemeMapping}
      />
    </View>
  );
}

function PanelBody({
  entry,
  projectId,
  cwd,
  theme,
}: {
  entry: PanelEntry;
  projectId: string;
  cwd: string;
  theme: PluginTheme;
}) {
  const { plugin, panel } = entry;
  const serverId = plugin.serverId;
  const client = useHostRuntimeClient(serverId);
  const hosts = useHosts();
  const hostLabel = hosts.find((host) => host.serverId === serverId)?.label ?? serverId;
  const host = useMemo(() => ({ id: serverId, label: hostLabel }), [hostLabel, serverId]);
  const layout = useMemo(() => ({ compact: false, platform: resolvePlatform() }), []);
  const stateSource = useMemo(() => createPluginClientStateSource(serverId), [serverId]);
  const navigation = usePluginHostNavigation(serverId);
  if (!client) return null;
  const Component = panel.Component;
  return (
    <SurfaceErrorBoundary installation={plugin} Surface={Component}>
      <PluginRuntimeBoundary plugin={plugin} client={client}>
        <PluginClientStateProvider source={stateSource}>
          <Component
            theme={theme}
            host={host}
            layout={layout}
            navigation={navigation}
            projectId={projectId}
            cwd={cwd}
          />
        </PluginClientStateProvider>
      </PluginRuntimeBoundary>
    </SurfaceErrorBoundary>
  );
}

const ThemedPanelBody = withUnistyles(PanelBody);

const styles = StyleSheet.create((theme) => ({
  panel: {
    width: SIDE_PANEL_WIDTH,
    flexShrink: 0,
    borderLeftWidth: theme.borderWidth[1],
    borderLeftColor: theme.colors.border,
    backgroundColor: theme.colors.surface0,
  },
}));
