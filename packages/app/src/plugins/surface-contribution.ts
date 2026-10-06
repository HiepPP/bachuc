import type { InstalledPlugin } from "./types";

export type PluginSurfaceContributionIdentity =
  | { kind: "sidebar"; id: string }
  | { kind: "surface"; id: string };

export function resolvePluginSurfaceContribution(
  plugin: InstalledPlugin | null,
  identity: PluginSurfaceContributionIdentity | null,
): {
  sidebarItem: InstalledPlugin["sidebarItems"][number] | null;
  surface: InstalledPlugin["surfaces"][number] | null;
} {
  if (!identity) return { sidebarItem: null, surface: null };
  // A surface opened directly takes the title and icon of the sidebar item that opens it.
  // Match on the item's `surface`, never its `id`: the two ID spaces are separate.
  const sidebarItem =
    plugin?.sidebarItems.find((contribution) =>
      identity.kind === "sidebar"
        ? contribution.id === identity.id
        : contribution.surface === identity.id,
    ) ?? null;
  const surfaceId = identity.kind === "sidebar" ? sidebarItem?.surface : identity.id;
  const surface = surfaceId
    ? (plugin?.surfaces.find((contribution) => contribution.id === surfaceId) ?? null)
    : null;
  return { sidebarItem, surface };
}

export function getPluginSurfaceContributionServerIds(
  installations: readonly InstalledPlugin[],
  pluginId: string,
  identity: PluginSurfaceContributionIdentity,
): string[] {
  return installations
    .filter((installation) => {
      if (installation.id !== pluginId) return false;
      return identity.kind === "sidebar"
        ? installation.sidebarItems.some((contribution) => contribution.id === identity.id)
        : installation.surfaces.some((contribution) => contribution.id === identity.id);
    })
    .map((installation) => installation.serverId);
}
