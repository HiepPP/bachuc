import type {
  NavigationAction,
  NavigationContainerRef,
  NavigationContainerRefWithCurrent,
} from "@react-navigation/native";
import { router, type Href } from "expo-router";
import {
  encodeWorkspaceIdForPathSegment,
  getHostWorkspaceOpenParamFromPathname,
  parseHostWorkspaceRouteFromPathname,
} from "@/utils/host-routes";

const ROOT_HOST_ROUTE_NAME = "h/[serverId]";
const HOST_WORKSPACE_ROUTE_NAME = "workspace/[workspaceId]/index";

interface NavigateToHostWorkspaceRouteDeps {
  dismissTo(route: string): void;
}

const defaultNavigateToHostWorkspaceRouteDeps: NavigateToHostWorkspaceRouteDeps = {
  dismissTo: (route) => router.dismissTo(route as Href),
};

let rootNavigationRef: NavigationContainerRefWithCurrent<ReactNavigation.RootParamList> | null =
  null;
let pendingIntent: { route: string; deps: NavigateToHostWorkspaceRouteDeps } | null = null;

export function registerWorkspaceRouteNavigationRef(
  ref: NavigationContainerRefWithCurrent<ReactNavigation.RootParamList>,
): () => void {
  rootNavigationRef = ref;
  const unsubscribe = ref.addListener("ready", flushPendingIntent);
  flushPendingIntent();
  return () => {
    unsubscribe();
    if (rootNavigationRef === ref) {
      rootNavigationRef = null;
    }
  };
}

interface MountedRouteStack {
  key: string;
  focusedRouteName: string | null;
  focusedServerId: string | null;
}

function readFocusedRoute(route: unknown): { name: string | null; serverId: string | null } {
  if (!route || typeof route !== "object") {
    return { name: null, serverId: null };
  }
  const { name, params } = route as { name?: unknown; params?: { serverId?: unknown } | null };
  const serverId = params?.serverId;
  return {
    name: typeof name === "string" ? name : null,
    serverId: typeof serverId === "string" ? serverId : null,
  };
}

function findStackWithMountedRouteName(
  state: unknown,
  routeName: string,
): MountedRouteStack | null {
  if (!state || typeof state !== "object") {
    return null;
  }

  const candidate = state as {
    index?: unknown;
    key?: unknown;
    routes?: unknown;
  };

  if (!Array.isArray(candidate.routes)) {
    return null;
  }

  if (
    typeof candidate.key === "string" &&
    candidate.routes.some(
      (route) =>
        !!route && typeof route === "object" && (route as { name?: unknown }).name === routeName,
    )
  ) {
    const focusedIndex =
      typeof candidate.index === "number" &&
      Number.isInteger(candidate.index) &&
      candidate.index >= 0 &&
      candidate.index < candidate.routes.length
        ? candidate.index
        : candidate.routes.length - 1;
    const focusedRoute = readFocusedRoute(candidate.routes[focusedIndex]);
    return {
      key: candidate.key,
      focusedRouteName: focusedRoute.name,
      focusedServerId: focusedRoute.serverId,
    };
  }

  for (const route of candidate.routes) {
    if (!route || typeof route !== "object") {
      continue;
    }
    const childStack = findStackWithMountedRouteName(
      (route as { state?: unknown }).state,
      routeName,
    );
    if (childStack) {
      return childStack;
    }
  }

  return null;
}

function dispatchHostWorkspacePopTo(
  route: string,
  navigation: NavigationContainerRef<ReactNavigation.RootParamList>,
): boolean {
  const selection = parseHostWorkspaceRouteFromPathname(route);
  if (!selection) {
    return false;
  }

  const rootState = navigation.getRootState();
  const hostStack = findStackWithMountedRouteName(rootState, ROOT_HOST_ROUTE_NAME);
  if (!hostStack) {
    return false;
  }
  // The focused host route is reused for a same-host hop (dismissTo handles it). Another host still
  // needs POP_TO: it updates the params in place instead of appending a second root host route.
  const isFocusedHostRoute = hostStack.focusedRouteName === ROOT_HOST_ROUTE_NAME;
  const isSwitchingHost =
    hostStack.focusedServerId !== null && hostStack.focusedServerId !== selection.serverId;
  if (isFocusedHostRoute && !isSwitchingHost) {
    return false;
  }
  const open = getHostWorkspaceOpenParamFromPathname(route);

  const action: NavigationAction = {
    type: "POP_TO",
    target: hostStack.key,
    payload: {
      name: ROOT_HOST_ROUTE_NAME,
      params: {
        serverId: selection.serverId,
        screen: HOST_WORKSPACE_ROUTE_NAME,
        params: {
          serverId: selection.serverId,
          workspaceId: encodeWorkspaceIdForPathSegment(selection.workspaceId),
          ...(open ? { open } : {}),
        },
        // React Navigation consumes this nested hint when resolving the host child screen.
        // The browser-route canonicalizer strips the resulting ?pop=true URL artifact.
        // Removing it lets repeated /new -> workspace hops append hidden deck entries.
        pop: true,
      },
    },
  };
  navigation.dispatch(action);
  return true;
}

export function navigateToHostWorkspaceRoute(
  route: string,
  deps: NavigateToHostWorkspaceRouteDeps = defaultNavigateToHostWorkspaceRouteDeps,
): void {
  pendingIntent = { route, deps };
  flushPendingIntent();
}

function flushPendingIntent(): void {
  const navigation = rootNavigationRef?.current;
  if (!pendingIntent || !navigation?.isReady()) return;

  const { route, deps } = pendingIntent;
  pendingIntent = null;
  if (!dispatchHostWorkspacePopTo(route, navigation)) deps.dismissTo(route);
}
