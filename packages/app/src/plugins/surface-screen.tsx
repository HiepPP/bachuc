import { useIsFocused, useNavigationState, useRoute } from "@react-navigation/native";
import { router, useLocalSearchParams } from "expo-router";
import type { PluginSurfaceProps } from "@getpaseo/plugin/client";
import type { PluginTheme } from "@getpaseo/plugin";
import { ChevronDown, X } from "lucide-react-native";
import { useCallback, useMemo, useRef, useState, type ComponentType, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { Platform, Pressable, Text, View } from "react-native";
import { StyleSheet, withUnistyles } from "react-native-unistyles";
import { HeaderIconBadge } from "@/components/headers/header-icon-badge";
import { HeaderToggleButton } from "@/components/headers/header-toggle-button";
import { ScreenHeader } from "@/components/headers/screen-header";
import { ScreenTitle } from "@/components/headers/screen-title";
import { HostPicker } from "@/components/hosts/host-picker";
import { useIsCompactFormFactor } from "@/constants/layout";
import { isWeb } from "@/constants/platform";
import {
  getOverlayRoot,
  OverlayLayerProvider,
  useGlobalWebOverlayLayer,
  useWebOverlayRegistration,
} from "@/lib/overlay-root";
import { useHostRuntimeClient, useHosts } from "@/runtime/host-runtime";
import { useActiveServerId } from "@/hosts/use-visible-hosts";
import type { Theme } from "@/styles/theme";
import type { ShortcutKey } from "@/utils/format-shortcut";
import { WindowChromeRegion } from "@/utils/desktop-window";
import { usePluginHostNavigation } from "./host-navigation";
import { resolvePluginIcon } from "./icons";
import { toPluginTheme } from "./theme";
import { useInstalledPlugin, usePluginInstallations } from "./registry";
import {
  buildPluginSurfaceRoute,
  isPluginOverlayRoute,
  type PluginSurfacePresentation,
} from "./routes";
import { rememberPluginContributionHost } from "./contribution-host";
import { SurfaceErrorBoundary } from "./surface-error-boundary";
import type { DaemonClient } from "@getpaseo/client/internal/daemon-client";
import { PluginRuntimeBoundary } from "./runtime-boundary";
import {
  getPluginSurfaceContributionServerIds,
  pluginSurfaceHeading,
  resolvePluginSurfaceContribution,
  type PluginSurfaceContributionIdentity,
} from "./surface-contribution";

const EMPTY_SHORTCUT_KEYS: ShortcutKey[] = [];
const mutedColorMapping = (theme: Theme) => ({ color: theme.colors.foregroundMuted });
const pluginThemeMapping = (theme: Theme) => ({
  theme: toPluginTheme(theme),
});
const ThemedX = withUnistyles(X);
const ThemedChevronDown = withUnistyles(ChevronDown);

function routeParam(value: string | string[] | undefined): string {
  return typeof value === "string" ? value : "";
}

function PluginHeaderIcon({
  Icon,
  color = "",
}: {
  Icon: ReturnType<typeof resolvePluginIcon>;
  color?: string;
}) {
  return <Icon size={16} color={color} />;
}

const ThemedPluginHeaderIcon = withUnistyles(PluginHeaderIcon);

function SurfaceRenderer({
  Surface,
  client,
  plugin,
  layout,
  host,
  theme,
}: {
  Surface: ComponentType<PluginSurfaceProps>;
  client: DaemonClient;
  plugin: NonNullable<ReturnType<typeof useInstalledPlugin>>;
  layout: PluginSurfaceProps["layout"];
  host: PluginSurfaceProps["host"];
  theme: PluginTheme;
}) {
  const navigation = usePluginHostNavigation(host.id);
  return (
    <PluginRuntimeBoundary plugin={plugin} client={client}>
      <Surface theme={theme} host={host} layout={layout} navigation={navigation} />
    </PluginRuntimeBoundary>
  );
}

const ThemedSurfaceRenderer = withUnistyles(SurfaceRenderer);

// A portal escapes the hidden screen, so a surface covered by a later screen hides itself.
const HIDDEN_STYLE = { display: "none" } as const;

/**
 * The popup form of a plugin surface: a dimmed backdrop and a centered card.
 *
 * On web it renders into the shared overlay root. Electron paints browser panes in a body-level
 * layer that sits above the app root, so only that root covers them. A real React portal keeps
 * the plugin runtime context. The card is a web overlay scope: it owns Escape, traps Tab, and
 * restores focus on close. A modal opened inside the card registers above it, so one Escape
 * closes only that modal.
 */
function PluginSurfaceOverlay({
  close,
  focused,
  children,
}: {
  close: () => void;
  focused: boolean;
  children: ReactNode;
}) {
  const webOverlayActive = isWeb && focused;
  const modalLayer = useGlobalWebOverlayLayer("modal", webOverlayActive);
  const handleKeyDown = useCallback(
    (event: KeyboardEvent) => {
      if (event.key !== "Escape") return false;
      event.preventDefault();
      event.stopPropagation();
      close();
      return true;
    },
    [close],
  );
  const setScope = useWebOverlayRegistration({
    active: webOverlayActive,
    layer: modalLayer,
    onKeyDown: handleKeyDown,
  });
  const view = (
    <View
      style={[styles.overlayRoot, isWeb && { zIndex: modalLayer }, !focused && HIDDEN_STYLE]}
      testID="plugin-surface-overlay"
    >
      <Pressable
        accessibilityRole="button"
        accessibilityLabel="Dismiss overlay"
        focusable={false}
        onPress={close}
        style={styles.backdrop}
        testID="plugin-surface-backdrop"
      />
      <View ref={setScope} style={styles.overlayCard} role="dialog" aria-modal tabIndex={-1}>
        <OverlayLayerProvider layer={modalLayer}>
          {/* The card sits away from the window corners, so it owns no window controls. */}
          <WindowChromeRegion corners="none">{children}</WindowChromeRegion>
        </OverlayLayerProvider>
      </View>
    </View>
  );
  if (isWeb) return createPortal(view, getOverlayRoot());
  return view;
}

function resolvePlatform(): PluginSurfaceProps["layout"]["platform"] {
  if (Platform.OS === "ios") return "ios";
  if (Platform.OS === "android") return "android";
  return "web";
}

function PluginHostSwitcher({
  serverId,
  pluginId,
  identity,
  presentation,
  serverIds,
}: {
  serverId: string;
  pluginId: string;
  identity: PluginSurfaceContributionIdentity;
  presentation: PluginSurfacePresentation;
  serverIds: string[];
}) {
  const allHosts = useHosts();
  const activeServerId = useActiveServerId();
  const hosts = useMemo(
    () => allHosts.filter((host) => serverIds.includes(host.serverId)),
    [allHosts, serverIds],
  );
  const [open, setOpen] = useState(false);
  const anchorRef = useRef<View | null>(null);
  const selectedLabel = hosts.find((host) => host.serverId === serverId)?.label ?? serverId;
  const selectHost = useCallback(
    (nextServerId: string) => {
      rememberPluginContributionHost(`${pluginId}/${identity.kind}/${identity.id}`, nextServerId);
      router.replace(buildPluginSurfaceRoute(nextServerId, pluginId, identity, presentation));
    },
    [identity, pluginId, presentation],
  );
  const openPicker = useCallback(() => setOpen(true), []);
  // An active host already decides which host this page shows.
  const show = activeServerId === null && serverIds.length > 1 && hosts.length > 1;
  if (!show) return null;

  return (
    <HostPicker
      hosts={hosts}
      value={serverId}
      onSelect={selectHost}
      open={open}
      onOpenChange={setOpen}
      anchorRef={anchorRef}
      title="Choose plugin host"
      desktopPlacement="bottom-start"
    >
      <View ref={anchorRef} collapsable={false}>
        <Pressable
          accessibilityRole="button"
          accessibilityLabel={`Plugin host: ${selectedLabel}`}
          testID="plugin-host-switcher"
          onPress={openPicker}
          style={styles.hostSwitcher}
        >
          <Text numberOfLines={1} style={styles.hostSwitcherText}>
            {selectedLabel}
          </Text>
          <ThemedChevronDown size={14} uniProps={mutedColorMapping} />
        </Pressable>
      </View>
    </HostPicker>
  );
}

export function PluginSurfaceScreen() {
  const params = useLocalSearchParams<{
    serverId?: string | string[];
    pluginId?: string | string[];
    contributionKind?: string | string[];
    contributionId?: string | string[];
    presentation?: string | string[];
  }>();
  const serverId = routeParam(params.serverId);
  const pluginId = routeParam(params.pluginId);
  const contributionKind = routeParam(params.contributionKind);
  const contributionId = routeParam(params.contributionId);
  const identity = useMemo<PluginSurfaceContributionIdentity | null>(() => {
    if (contributionKind !== "sidebar" && contributionKind !== "surface") return null;
    return { kind: contributionKind, id: contributionId };
  }, [contributionId, contributionKind]);
  const plugin = useInstalledPlugin(serverId, pluginId);
  const installations = usePluginInstallations(pluginId);
  const hosts = useHosts();
  const client = useHostRuntimeClient(serverId);
  const compact = useIsCompactFormFactor();
  const presentation: PluginSurfacePresentation =
    routeParam(params.presentation) === "overlay" ? "overlay" : "page";
  // The route asks for an overlay and a screen sits below it. `_layout.tsx` applies the same rule
  // to the stack options, so the transparent screen and this card appear together.
  const routeKey = useRoute().key;
  const isOverlayRoute = useNavigationState((state) => isPluginOverlayRoute(state, routeKey));
  const overlay = isOverlayRoute && !compact;
  const { sidebarItem, surface } = useMemo(
    () => resolvePluginSurfaceContribution(plugin, identity),
    [identity, plugin],
  );
  const hostLabel = hosts.find((host) => host.serverId === serverId)?.label ?? serverId;
  const contributionServerIds = useMemo(
    () =>
      identity ? getPluginSurfaceContributionServerIds(installations, pluginId, identity) : [],
    [identity, installations, pluginId],
  );
  const heading = pluginSurfaceHeading(sidebarItem, surface, pluginId);
  const title = heading.title;
  const Icon = heading.icon ? resolvePluginIcon(heading.icon) : null;
  const close = useCallback(() => {
    if (router.canGoBack()) router.back();
    else router.replace(`/h/${encodeURIComponent(serverId)}`);
  }, [serverId]);
  const focused = useIsFocused();
  const layout = useMemo(() => ({ compact, platform: resolvePlatform() }), [compact]);
  const host = useMemo(() => ({ id: serverId, label: hostLabel }), [hostLabel, serverId]);
  const headerLeft = useMemo(
    () => (
      <>
        {Icon ? (
          <HeaderIconBadge>
            <ThemedPluginHeaderIcon Icon={Icon} uniProps={mutedColorMapping} />
          </HeaderIconBadge>
        ) : null}
        <ScreenTitle>{title}</ScreenTitle>
      </>
    ),
    [Icon, title],
  );
  const headerRight = useMemo(
    () => (
      <>
        {identity ? (
          <PluginHostSwitcher
            serverId={serverId}
            pluginId={pluginId}
            identity={identity}
            presentation={presentation}
            serverIds={contributionServerIds}
          />
        ) : null}
        <HeaderToggleButton
          accessibilityLabel="Close plugin"
          onPress={close}
          testID="plugin-surface-close"
          tooltipKeys={EMPTY_SHORTCUT_KEYS}
          tooltipLabel="Close"
          tooltipSide="bottom"
        >
          <ThemedX size={18} uniProps={mutedColorMapping} />
        </HeaderToggleButton>
      </>
    ),
    [close, contributionServerIds, identity, pluginId, presentation, serverId],
  );

  const content = (
    <>
      <ScreenHeader left={headerLeft} right={headerRight} />
      <View style={styles.body}>
        {plugin && surface && client ? (
          <SurfaceErrorBoundary
            key={`${serverId}/${pluginId}/${identity?.kind}/${contributionId}`}
            installation={plugin}
            Surface={surface.Component}
          >
            <ThemedSurfaceRenderer
              Surface={surface.Component}
              client={client}
              plugin={plugin}
              host={host}
              layout={layout}
              uniProps={pluginThemeMapping}
            />
          </SurfaceErrorBoundary>
        ) : (
          <Text style={styles.errorText}>
            {plugin && surface ? "Plugin host is offline." : "This plugin surface is unavailable."}
          </Text>
        )}
      </View>
    </>
  );

  if (!overlay) return <View style={styles.screen}>{content}</View>;

  return (
    <PluginSurfaceOverlay close={close} focused={focused}>
      {content}
    </PluginSurfaceOverlay>
  );
}

const styles = StyleSheet.create((theme) => ({
  screen: {
    flex: 1,
    backgroundColor: theme.colors.surface0,
  },
  body: {
    flex: 1,
  },
  overlayRoot: {
    ...StyleSheet.absoluteFillObject,
    alignItems: "center",
    justifyContent: "center",
    // The shared overlay root ignores pointer events so its children opt in.
    pointerEvents: "auto",
  },
  backdrop: {
    ...StyleSheet.absoluteFillObject,
    backgroundColor: "rgba(0, 0, 0, 0.35)",
  },
  overlayCard: {
    width: "92%",
    maxWidth: 1180,
    height: "88%",
    backgroundColor: theme.colors.surface0,
    borderWidth: theme.borderWidth[1],
    borderColor: theme.colors.border,
    borderRadius: theme.borderRadius.lg,
    overflow: "hidden",
    ...theme.shadow.lg,
  },
  errorText: {
    color: theme.colors.statusDanger,
    padding: theme.spacing[4],
  },
  hostSwitcher: {
    maxWidth: 180,
    minHeight: 32,
    paddingHorizontal: theme.spacing[2],
    flexDirection: "row",
    alignItems: "center",
    gap: theme.spacing[1],
    borderRadius: theme.borderRadius.md,
    backgroundColor: theme.colors.surface1,
  },
  hostSwitcherText: {
    flexShrink: 1,
    color: theme.colors.foregroundMuted,
    fontSize: theme.fontSize.base,
  },
}));
