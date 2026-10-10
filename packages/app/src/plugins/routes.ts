import type { PluginSurfaceContributionIdentity } from "./surface-contribution";

type PluginSurfaceRoute<Kind extends PluginSurfaceContributionIdentity["kind"]> =
  `/h/${string}/plugin/${string}/${Kind}/${string}`;

export type PluginSurfacePresentation = "page" | "overlay";

export function buildPluginSurfaceRoute<Identity extends PluginSurfaceContributionIdentity>(
  serverId: string,
  pluginId: string,
  identity: Identity,
  presentation?: PluginSurfacePresentation,
): PluginSurfaceRoute<Identity["kind"]> {
  const query = presentation === "overlay" ? "?presentation=overlay" : "";
  return `/h/${encodeURIComponent(serverId)}/plugin/${encodeURIComponent(pluginId)}/${identity.kind}/${encodeURIComponent(identity.id)}${query}` as PluginSurfaceRoute<
    Identity["kind"]
  >;
}

export function buildLegacyPluginSurfaceRedirectRoute(
  serverId: string,
  pluginId: string,
  sidebarContributionId: string,
): `/h/${string}/plugin/${string}/sidebar/${string}` {
  return buildPluginSurfaceRoute(serverId, pluginId, {
    kind: "sidebar",
    id: sidebarContributionId,
  });
}

export function hostIdFromPathname(pathname: string): string | null {
  const encoded = /^\/h\/([^/]+)/.exec(pathname)?.[1];
  if (!encoded) return null;
  try {
    return decodeURIComponent(encoded);
  } catch {
    return null;
  }
}

/** The name of the plugin surface screen in the host stack (`app/h/[serverId]`). */
export const PLUGIN_SURFACE_SCREEN_NAME = "plugin/[pluginId]/[contributionKind]/[contributionId]";

interface StackRoutes {
  routes: ReadonlyArray<{ key: string; name: string; params?: object }>;
}

/**
 * True when this route asks for `presentation=overlay` and a screen sits below it in the same
 * stack. A reload, a deep link, or a surface on another host starts a stack with the route
 * first, so there is nothing to show under the backdrop and the route stays a page.
 */
export function isPluginOverlayRoute(state: StackRoutes, routeKey: string): boolean {
  const index = state.routes.findIndex((route) => route.key === routeKey);
  const route = state.routes[index];
  if (index < 1 || route?.name !== PLUGIN_SURFACE_SCREEN_NAME) return false;
  return (route.params as { presentation?: unknown } | undefined)?.presentation === "overlay";
}

/** True when the route directly above `routeKey` in its stack is an overlay route. */
export function isPluginOverlayRouteAbove(state: StackRoutes, routeKey: string): boolean {
  const index = state.routes.findIndex((route) => route.key === routeKey);
  const above = index < 0 ? undefined : state.routes[index + 1];
  return above !== undefined && isPluginOverlayRoute(state, above.key);
}

export function parsePluginSurfaceRoute(pathname: string): {
  serverId: string;
  pluginId: string;
  identity: PluginSurfaceContributionIdentity;
} | null {
  const match = /^\/h\/([^/]+)\/plugin\/([^/]+)\/(sidebar|surface)\/([^/?#]+)\/?(?:[?#]|$)/.exec(
    pathname,
  );
  if (!match) return null;
  try {
    return {
      serverId: decodeURIComponent(match[1]),
      pluginId: decodeURIComponent(match[2]),
      identity: {
        kind: match[3] === "sidebar" ? "sidebar" : "surface",
        id: decodeURIComponent(match[4]),
      },
    };
  } catch {
    return null;
  }
}
