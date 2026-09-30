import type { Href } from "expo-router";
import { buildPluginSurfaceRoute, parsePluginSurfaceRoute } from "@/plugins/routes";
import { getPluginSurfaceContributionServerIds } from "@/plugins/surface-contribution";
import type { InstalledPlugin } from "@/plugins/types";
import {
  buildHostOpenProjectRoute,
  buildHostRootRoute,
  parseServerIdFromPathname,
} from "@/utils/host-routes";

const LANDING_PATHNAMES = new Set(["/", "/open-project"]);

/**
 * Where switching the active host should land, or null to stay put.
 *
 * Pages without a host in the route (History, Schedules, New workspace, Settings) already follow
 * the active host or show every host, so they stay. Otherwise the target host's last screen wins,
 * so switching back returns to the same thread or plugin page. Without one, a plugin page opens the
 * same contribution on the target when it has it.
 *
 * From another host's page the result is always a concrete route, never the bare `/h/<id>`:
 * navigating there from inside `/h/<other>/...` reuses the mounted host navigator and lands back on
 * the old host's index.
 */
export function resolveHostSwitchRoute(input: {
  pathname: string;
  targetServerId: string;
  installations: readonly InstalledPlugin[];
  lastRouteByServerId: Readonly<Record<string, string>>;
}): Href | null {
  const lastRoute = input.lastRouteByServerId[input.targetServerId] as Href | undefined;
  const routeServerId = parseServerIdFromPathname(input.pathname);
  if (routeServerId === null) {
    if (!LANDING_PATHNAMES.has(input.pathname)) return null;
    return lastRoute ?? buildHostRootRoute(input.targetServerId);
  }
  if (routeServerId === input.targetServerId) return null;
  if (lastRoute) return lastRoute;
  const pluginRoute = parsePluginSurfaceRoute(input.pathname);
  if (
    pluginRoute &&
    getPluginSurfaceContributionServerIds(
      input.installations,
      pluginRoute.pluginId,
      pluginRoute.identity,
    ).includes(input.targetServerId)
  ) {
    return buildPluginSurfaceRoute(
      input.targetServerId,
      pluginRoute.pluginId,
      pluginRoute.identity,
    );
  }
  return buildHostOpenProjectRoute(input.targetServerId);
}
