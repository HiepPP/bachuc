import { router, usePathname } from "expo-router";
import { ChevronsUpDown, Server } from "lucide-react-native";
import { useCallback, useRef, useState } from "react";
import { Pressable, Text, View, type PressableStateCallbackType } from "react-native";
import { StyleSheet, withUnistyles } from "react-native-unistyles";
import {
  ALL_HOSTS_OPTION_ID,
  HostPicker,
  getHostPickerLabel,
} from "@/components/hosts/host-picker";
import { useActiveServerId } from "@/hosts/use-visible-hosts";
import { useHosts } from "@/runtime/host-runtime";
import { getNextActiveServerId, useActiveHostStore } from "@/stores/active-host-store";
import { useLocalDaemonServerId } from "@/hooks/use-is-local-daemon";
import { orderHostsLocalFirst } from "@/types/host-connection";
import { useSessionStore } from "@/stores/session-store";
import { ICON_SIZE, type Theme } from "@/styles/theme";
import { resolveHostSwitchRoute } from "@/hosts/host-switch-route";
import { pluginRegistry } from "@/plugins/registry";
import { navigateToWorkspace } from "@/stores/navigation-active-workspace-store";
import { parseHostWorkspaceRouteFromPathname } from "@/utils/host-routes";

const ThemedServer = withUnistyles(Server);
const ThemedChevrons = withUnistyles(ChevronsUpDown);
const foregroundColorMapping = (theme: Theme) => ({ color: theme.colors.foreground });
const foregroundMutedColorMapping = (theme: Theme) => ({ color: theme.colors.foregroundMuted });

function activeHostOptionTestID(serverId: string): string {
  return `sidebar-active-host-option-${serverId}`;
}

/** Hosts other than the active one that have an agent waiting on the user. */
function useHiddenHostsNeedingAttention(activeServerId: string | null): number {
  return useSessionStore((state) => {
    if (activeServerId === null) return 0;
    let count = 0;
    for (const [serverId, session] of Object.entries(state.sessions)) {
      if (serverId === activeServerId) continue;
      for (const agent of session.agents.values()) {
        if (agent.archivedAt) continue;
        if (agent.requiresAttention || agent.pendingPermissions.length > 0) {
          count += 1;
          break;
        }
      }
    }
    return count;
  });
}

function openHost(serverId: string, pathname: string) {
  const route = resolveHostSwitchRoute({
    pathname,
    targetServerId: serverId,
    installations: pluginRegistry.getSnapshot(),
    lastRouteByServerId: useActiveHostStore.getState().lastRouteByServerId,
  });
  if (!route) return;
  // Both paths reuse the single root host route. `router.navigate` would append a new one per host.
  const workspace = typeof route === "string" ? parseHostWorkspaceRouteFromPathname(route) : null;
  if (workspace) {
    navigateToWorkspace(workspace);
    return;
  }
  router.dismissTo(route);
}

function selectActiveHost(id: string, pathname: string, onBeforeNavigate?: () => void): void {
  const { setActiveServerId } = useActiveHostStore.getState();
  if (id === ALL_HOSTS_OPTION_ID) {
    setActiveServerId(null);
    return;
  }
  setActiveServerId(id);
  onBeforeNavigate?.();
  openHost(id, pathname);
}

/** Cmd+` moves to the next host, local first, in the same order as the picker. */
export function useCycleActiveHost(): () => void {
  const hosts = useHosts();
  const localServerId = useLocalDaemonServerId();
  const activeServerId = useActiveServerId();
  const pathname = usePathname();
  return useCallback(() => {
    const ordered = orderHostsLocalFirst(hosts, localServerId).map((host) => host.serverId);
    if (ordered.length < 2) return;
    const next = getNextActiveServerId(ordered, activeServerId);
    if (next) selectActiveHost(next, pathname);
  }, [activeServerId, hosts, localServerId, pathname]);
}

/**
 * Pins the whole app to one host, or to all of them. Lists, search, schedules, history, and
 * new-workspace defaults read the choice through `useVisibleHosts()`.
 */
export function ActiveHostSwitch({ onBeforeNavigate }: { onBeforeNavigate?: () => void }) {
  const hosts = useHosts();
  const activeServerId = useActiveServerId();
  const pathname = usePathname();
  const hiddenAttentionCount = useHiddenHostsNeedingAttention(activeServerId);
  const anchorRef = useRef<View | null>(null);
  const [open, setOpen] = useState(false);
  const value = activeServerId ?? ALL_HOSTS_OPTION_ID;
  const label = getHostPickerLabel(hosts, value, { includeAllHost: true });

  const handleSelect = useCallback(
    (id: string) => selectActiveHost(id, pathname, onBeforeNavigate),
    [onBeforeNavigate, pathname],
  );
  const handleOpen = useCallback(() => setOpen(true), []);
  const buttonStyle = useCallback(
    ({ hovered }: PressableStateCallbackType & { hovered?: boolean }) => [
      styles.button,
      Boolean(hovered) && styles.buttonHovered,
    ],
    [],
  );

  if (hosts.length < 2) return null;

  const accessibilityLabel =
    hiddenAttentionCount > 0
      ? `Active host: ${label}. ${hiddenAttentionCount} other host needs attention`
      : `Active host: ${label}`;

  return (
    <HostPicker
      hosts={hosts}
      value={value}
      onSelect={handleSelect}
      open={open}
      onOpenChange={setOpen}
      anchorRef={anchorRef}
      includeAllHost
      searchable
      title="Show host"
      desktopPlacement="bottom-start"
      desktopMinWidth={240}
      hostOptionTestID={activeHostOptionTestID}
    >
      <View ref={anchorRef} collapsable={false} style={styles.container}>
        <Pressable
          onPress={handleOpen}
          testID="sidebar-active-host-switch"
          accessibilityRole="button"
          accessibilityLabel={accessibilityLabel}
          style={buttonStyle}
        >
          {({ hovered }) => {
            const colorMapping = hovered ? foregroundColorMapping : foregroundMutedColorMapping;
            return (
              <>
                <ThemedServer size={ICON_SIZE.sm} uniProps={colorMapping} />
                <Text numberOfLines={1} style={styles.label}>
                  {label}
                </Text>
                {hiddenAttentionCount > 0 ? (
                  <View style={styles.attentionDot} testID="sidebar-active-host-attention" />
                ) : null}
                <ThemedChevrons size={ICON_SIZE.sm} uniProps={foregroundMutedColorMapping} />
              </>
            );
          }}
        </Pressable>
      </View>
    </HostPicker>
  );
}

const styles = StyleSheet.create((theme) => ({
  container: {
    paddingHorizontal: theme.spacing[2],
    userSelect: "none",
  },
  button: {
    flexDirection: "row",
    alignItems: "center",
    gap: theme.spacing[2],
    minHeight: 28,
    paddingVertical: theme.spacing[1],
    paddingHorizontal: theme.spacing[2],
    borderRadius: theme.borderRadius.lg,
  },
  buttonHovered: {
    backgroundColor: theme.colors.surfaceSidebarHover,
  },
  label: {
    flex: 1,
    fontSize: theme.fontSize.base,
    fontWeight: theme.fontWeight.medium,
    color: theme.colors.foreground,
  },
  attentionDot: {
    width: 8,
    height: 8,
    borderRadius: theme.borderRadius.full,
    backgroundColor: theme.colors.statusDotWarning,
  },
}));
