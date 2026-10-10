import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { useNavigation, useNavigationState, useRoute } from "@react-navigation/native";
import { StyleSheet, View } from "react-native";
import { useGlobalSearchParams, useLocalSearchParams, useRootNavigationState } from "expo-router";
import { useIsCompactFormFactor } from "@/constants/layout";
import { isPluginOverlayRouteAbove } from "@/plugins/routes";
import { HostRouteBootstrapBoundary } from "@/components/host-route-bootstrap-boundary";
import { RetainedPanel } from "@/components/retained-panel";
import {
  type ActiveWorkspaceSelection,
  useActiveWorkspaceSelection,
} from "@/stores/navigation-active-workspace-store";
import { useHasHydratedWorkspaces, useWorkspaceExists } from "@/stores/session-store-hooks";
import type { WorkspaceTabTarget } from "@/workspace-tabs/model";
import { WorkspaceScreen } from "@/screens/workspace/workspace-screen";
import { useWorkspaceLayoutStoreHydrated } from "@/stores/workspace-layout-store";
import {
  areRetainedWorkspaceSelectionListsEqual,
  areWorkspaceSelectionsEqual,
  getNextWorkspaceDeckExpirationDelay,
  getWorkspaceSelectionKey,
  orderWorkspaceSelectionsForStableRender,
  reconcileRetainedWorkspaceSelections,
  resolveWorkspaceDeckRetentionLimit,
  type RetainedWorkspaceSelection,
  resolveWorkspaceDeckEntries,
  resolveWorkspaceDeckVisibleSelection,
  shouldKeepWorkspaceDeckEntryMounted,
} from "@/screens/workspace/workspace-deck-retention";
import {
  decodeWorkspaceIdFromPathSegment,
  parseWorkspaceOpenIntent,
  type WorkspaceOpenIntent,
} from "@/utils/host-routes";
import {
  replaceBrowserRouteWithCanonicalHostWorkspaceRoute,
  stripHostWorkspaceRouteEchoSearchFromBrowserUrlAfterCommit,
} from "@/utils/host-route-browser";
import { prepareWorkspaceTab } from "@/utils/workspace-navigation";
import { isNative, isWeb } from "@/constants/platform";
import { RenderProfile } from "@/utils/render-profiler";

function getParamValue(value: string | string[] | undefined): string {
  if (typeof value === "string") {
    return value.trim();
  }
  if (Array.isArray(value)) {
    const firstValue = value[0];
    return typeof firstValue === "string" ? firstValue.trim() : "";
  }
  return "";
}

function getOpenIntentTarget(openIntent: WorkspaceOpenIntent): WorkspaceTabTarget {
  if (openIntent.kind === "agent") {
    return { kind: "agent", agentId: openIntent.agentId };
  }
  if (openIntent.kind === "terminal") {
    return { kind: "terminal", terminalId: openIntent.terminalId };
  }
  if (openIntent.kind === "file") {
    return { kind: "file", path: openIntent.path };
  }
  if (openIntent.kind === "setup") {
    return { kind: "setup", workspaceId: openIntent.workspaceId };
  }
  return { kind: "draft", draftId: openIntent.draftId };
}

function stripOpenSearchParamFromBrowserUrl() {
  if (!isWeb || typeof window === "undefined") {
    return;
  }
  const url = new URL(window.location.href);
  if (!url.searchParams.has("open")) {
    return;
  }
  url.searchParams.delete("open");
  replaceBrowserRouteWithCanonicalHostWorkspaceRoute(`${url.pathname}${url.search}${url.hash}`);
}

function clearConsumedOpenIntent(input: {
  navigation: { setParams: (params: { open?: string | undefined }) => void };
}) {
  input.navigation.setParams({ open: undefined });
  if (isWeb) {
    stripOpenSearchParamFromBrowserUrl();
  }
}

export default function HostWorkspaceIndexRoute() {
  return (
    <HostRouteBootstrapBoundary>
      <HostWorkspaceRouteContent />
    </HostRouteBootstrapBoundary>
  );
}

function HostWorkspaceRouteContent() {
  const navigation = useNavigation();
  const rootNavigationState = useRootNavigationState();
  const hasHydratedWorkspaceLayoutStore = useWorkspaceLayoutStoreHydrated();
  const consumedIntentRef = useRef<string | null>(null);
  const [intentConsumed, setIntentConsumed] = useState(false);
  const params = useLocalSearchParams<{
    serverId?: string | string[];
    workspaceId?: string | string[];
  }>();
  const globalParams = useGlobalSearchParams<{
    open?: string | string[];
  }>();
  const serverId = getParamValue(params.serverId);
  const workspaceValue = getParamValue(params.workspaceId);
  const workspaceId = workspaceValue
    ? (decodeWorkspaceIdFromPathSegment(workspaceValue) ?? "")
    : "";
  const openValue = getParamValue(globalParams.open);
  const hasHydratedWorkspaces = useHasHydratedWorkspaces(serverId);
  const workspaceExists = useWorkspaceExists(serverId, workspaceId);
  const openIntent = useMemo(() => parseWorkspaceOpenIntent(openValue), [openValue]);
  const isAgentOpenIntent = openIntent?.kind === "agent";
  const isOpenIntentWaitingForWorkspace = Boolean(
    isAgentOpenIntent && (!hasHydratedWorkspaces || !workspaceExists),
  );
  useEffect(() => {
    if (!serverId || !workspaceId) {
      return;
    }
    stripHostWorkspaceRouteEchoSearchFromBrowserUrlAfterCommit();
  }, [serverId, workspaceId]);

  useEffect(() => {
    if (!openValue) {
      return;
    }
    if (!rootNavigationState?.key) {
      return;
    }
    if (!hasHydratedWorkspaceLayoutStore) {
      return;
    }
    if (isOpenIntentWaitingForWorkspace) {
      return;
    }

    const consumptionKey = `${serverId}:${workspaceId}:${openValue}`;
    if (consumedIntentRef.current === consumptionKey) {
      clearConsumedOpenIntent({
        navigation: navigation as unknown as {
          setParams: (params: { open?: string | undefined }) => void;
        },
      });
      setIntentConsumed(true);
      return;
    }
    consumedIntentRef.current = consumptionKey;

    if (openIntent) {
      prepareWorkspaceTab({
        serverId,
        workspaceId,
        target: getOpenIntentTarget(openIntent),
        pin: openIntent.kind === "agent",
      });
    }

    // Expo Router's replace ignores query-param-only changes (findDivergentState
    // skips search params). Strip ?open from the browser URL directly so the
    // address bar reflects the clean workspace route.
    clearConsumedOpenIntent({
      navigation: navigation as unknown as {
        setParams: (params: { open?: string | undefined }) => void;
      },
    });

    setIntentConsumed(true);
  }, [
    hasHydratedWorkspaceLayoutStore,
    isOpenIntentWaitingForWorkspace,
    navigation,
    openIntent,
    openValue,
    rootNavigationState?.key,
    serverId,
    workspaceId,
  ]);

  if (
    openValue &&
    !isOpenIntentWaitingForWorkspace &&
    (!intentConsumed || !hasHydratedWorkspaceLayoutStore)
  ) {
    return null;
  }

  return (
    <WorkspaceDeck
      ownServerId={serverId}
      ownWorkspaceId={workspaceId}
      recoveryRequested={isAgentOpenIntent}
    />
  );
}

function WorkspaceDeck({
  ownServerId,
  ownWorkspaceId,
  recoveryRequested,
}: {
  ownServerId: string;
  ownWorkspaceId: string;
  recoveryRequested: boolean;
}) {
  const routeSelection = useActiveWorkspaceSelection();
  const routeKey = useRoute().key;
  // The overlay shows this screen only when it sits directly above it, and not on a compact
  // layout, where a plugin surface is a plain page.
  const isOverlayAbove = useNavigationState((state) => isPluginOverlayRouteAbove(state, routeKey));
  const compact = useIsCompactFormFactor();
  const ownSelection = useMemo<ActiveWorkspaceSelection | null>(
    () =>
      ownServerId && ownWorkspaceId ? { serverId: ownServerId, workspaceId: ownWorkspaceId } : null,
    [ownServerId, ownWorkspaceId],
  );
  const activeSelection = resolveWorkspaceDeckVisibleSelection({
    routeSelection,
    ownSelection,
    isOverlayOnTop: isOverlayAbove && !compact,
  });
  const [retainedSelections, setRetainedSelections] = useState<RetainedWorkspaceSelection[]>(() =>
    activeSelection ? [{ selection: activeSelection, inactiveSince: null }] : [],
  );
  const unmountWorkspaceSelection = useCallback((selection: ActiveWorkspaceSelection) => {
    setRetainedSelections((current) =>
      current.filter((entry) => !areWorkspaceSelectionsEqual(entry.selection, selection)),
    );
  }, []);

  const reconciliationNow = Date.now();
  const nextRetainedSelections = useMemo(
    () =>
      reconcileRetainedWorkspaceSelections({
        currentEntries: retainedSelections,
        activeSelection,
        now: reconciliationNow,
        maxMountedWorkspaces: resolveWorkspaceDeckRetentionLimit({ isNative }),
      }),
    [activeSelection, reconciliationNow, retainedSelections],
  );
  const renderedSelections = useMemo(
    () =>
      orderWorkspaceSelectionsForStableRender(
        nextRetainedSelections.map((entry) => entry.selection),
      ),
    [nextRetainedSelections],
  );
  const renderedEntries = useMemo(
    () =>
      resolveWorkspaceDeckEntries({
        selections: renderedSelections,
        activeSelection,
        focusedSelection: routeSelection,
      }),
    [activeSelection, renderedSelections, routeSelection],
  );

  useLayoutEffect(() => {
    if (!areRetainedWorkspaceSelectionListsEqual(retainedSelections, nextRetainedSelections)) {
      setRetainedSelections(nextRetainedSelections);
    }
  }, [nextRetainedSelections, retainedSelections]);

  useEffect(() => {
    const expirationDelay = getNextWorkspaceDeckExpirationDelay({
      entries: nextRetainedSelections,
      activeSelection,
      now: reconciliationNow,
    });
    if (expirationDelay === null) {
      return;
    }
    const timeout = setTimeout(() => {
      setRetainedSelections((current) =>
        reconcileRetainedWorkspaceSelections({
          currentEntries: current,
          activeSelection,
          now: Date.now(),
          maxMountedWorkspaces: resolveWorkspaceDeckRetentionLimit({ isNative }),
        }),
      );
    }, expirationDelay + 1);
    return () => clearTimeout(timeout);
  }, [activeSelection, nextRetainedSelections, reconciliationNow]);

  return (
    <RenderProfile id="WorkspaceDeck">
      <View style={styles.deck}>
        {renderedEntries.map(({ selection, active, focused }) => {
          return (
            <WorkspaceDeckEntry
              key={getWorkspaceSelectionKey(selection)}
              selection={selection}
              active={active}
              focused={focused}
              recoveryRequested={recoveryRequested}
              onUnmountInactive={unmountWorkspaceSelection}
            />
          );
        })}
      </View>
    </RenderProfile>
  );
}

function WorkspaceDeckEntry({
  selection,
  active,
  focused,
  recoveryRequested,
  onUnmountInactive,
}: {
  selection: ActiveWorkspaceSelection;
  /** On display. It stays true under a plugin overlay, where `focused` is false. */
  active: boolean;
  focused: boolean;
  recoveryRequested: boolean;
  onUnmountInactive: (selection: ActiveWorkspaceSelection) => void;
}) {
  const hasHydratedWorkspaces = useHasHydratedWorkspaces(selection.serverId);
  const workspaceExists = useWorkspaceExists(selection.serverId, selection.workspaceId);
  const shouldKeepMounted = shouldKeepWorkspaceDeckEntryMounted({
    isActive: active,
    hasHydratedWorkspaces,
    workspaceExists,
  });

  useEffect(() => {
    if (!shouldKeepMounted) {
      onUnmountInactive(selection);
    }
  }, [onUnmountInactive, selection, shouldKeepMounted]);

  if (!shouldKeepMounted) {
    return null;
  }

  return (
    <RetainedPanel
      active={active}
      testID={`workspace-deck-entry-${selection.serverId}:${selection.workspaceId}`}
    >
      <WorkspaceScreen
        serverId={selection.serverId}
        workspaceId={selection.workspaceId}
        isRouteFocused={focused}
        recoveryRequested={focused && recoveryRequested}
      />
    </RetainedPanel>
  );
}

const styles = StyleSheet.create({
  deck: {
    flex: 1,
  },
});
