import { usePathname } from "expo-router";
import { useEffect, useMemo, useSyncExternalStore } from "react";
import type { HostProfile } from "@/types/host-connection";
import { useHostRegistryLoaded, useHosts } from "@/runtime/host-runtime";
import { resolveActiveServerId, useActiveHostStore } from "@/stores/active-host-store";
import {
  parseHostWorkspaceRouteFromPathname,
  parseServerIdFromPathname,
} from "@/utils/host-routes";

function subscribeActiveHostHydration(listener: () => void): () => void {
  return useActiveHostStore.persist.onFinishHydration(listener);
}

function getActiveHostHydrated(): boolean {
  return useActiveHostStore.persist.hasHydrated();
}

/** Startup redirects wait for this, so a restored active host is not overruled by a default. */
export function useActiveHostHydrated(): boolean {
  return useSyncExternalStore(
    subscribeActiveHostHydration,
    getActiveHostHydrated,
    getActiveHostHydrated,
  );
}

/** The machine the whole UI is pinned to, or null for "All machines". */
export function useActiveServerId(): string | null {
  const hosts = useHosts();
  const activeServerId = useActiveHostStore((state) => state.activeServerId);
  return useMemo(
    () =>
      resolveActiveServerId(
        activeServerId,
        hosts.map((host) => host.serverId),
      ),
    [activeServerId, hosts],
  );
}

/**
 * Hosts that lists and aggregates should show. Settings, pairing, and lookups by `serverId` keep
 * `useHosts()`, because a route or a setting can still name a host that is hidden here.
 */
export function useVisibleHosts(): HostProfile[] {
  const hosts = useHosts();
  const activeServerId = useActiveServerId();
  return useMemo(
    () => (activeServerId ? hosts.filter((host) => host.serverId === activeServerId) : hosts),
    [activeServerId, hosts],
  );
}

export type HostScope = "all" | "visible";

/** Shared hooks take a scope so each caller decides whether it follows the active host. */
export function useScopedHosts(scope: HostScope = "all"): HostProfile[] {
  const hosts = useHosts();
  const visibleHosts = useVisibleHosts();
  return scope === "visible" ? visibleHosts : hosts;
}

/**
 * A screen worth returning to when switching back to its host. The bare host index only redirects,
 * and host settings belong to Settings, not to the host's working screens.
 */
function isRememberableHostRoute(
  pathname: string,
  routeServerId: string | null,
): routeServerId is string {
  if (!routeServerId) return false;
  const rest = pathname.replace(/^\/h\/[^/]+/, "");
  return rest !== "" && rest !== "/" && !rest.startsWith("/settings");
}

/**
 * Keeps the active machine in step with the route. Mounted once at the root.
 *
 * A deep link or notification that opens another host switches the active machine to it, so the
 * lists never show one machine while the page shows another. Only a route change triggers this:
 * picking a machine sets the store before it navigates, and must not be undone by the old route.
 */
export function useActiveHostRouteSync(): void {
  const pathname = usePathname();
  const hosts = useHosts();
  const hostRegistryLoaded = useHostRegistryLoaded();
  const hydrated = useActiveHostHydrated();
  const routeServerId = useMemo(() => parseServerIdFromPathname(pathname), [pathname]);
  const routeWorkspace = useMemo(() => parseHostWorkspaceRouteFromPathname(pathname), [pathname]);
  const routeWorkspaceServerId = routeWorkspace?.serverId ?? null;
  const routeWorkspaceId = routeWorkspace?.workspaceId ?? null;

  useEffect(() => {
    if (!routeServerId || !hydrated) return;
    const { activeServerId, setActiveServerId } = useActiveHostStore.getState();
    if (activeServerId !== null && activeServerId !== routeServerId) {
      setActiveServerId(routeServerId);
    }
  }, [hydrated, routeServerId]);

  useEffect(() => {
    // Hydration replaces the in-memory map, so a write before it would be lost.
    if (!hydrated || !routeWorkspaceServerId || !routeWorkspaceId) return;
    useActiveHostStore.getState().rememberWorkspace(routeWorkspaceServerId, routeWorkspaceId);
  }, [hydrated, routeWorkspaceId, routeWorkspaceServerId]);

  useEffect(() => {
    if (!hydrated || !isRememberableHostRoute(pathname, routeServerId)) return;
    useActiveHostStore.getState().rememberRoute(routeServerId, pathname);
  }, [hydrated, pathname, routeServerId]);

  useEffect(() => {
    // Before the registry loads the host list is empty, and reconciling would drop the choice.
    if (!hostRegistryLoaded || !hydrated) return;
    useActiveHostStore.getState().reconcile(hosts.map((host) => host.serverId));
  }, [hostRegistryLoaded, hosts, hydrated]);
}
