import { router, usePathname } from "expo-router";
import { useCallback } from "react";
import { SidebarHeaderRow } from "@/components/sidebar/sidebar-header-row";
import { useActiveServerId } from "@/hosts/use-visible-hosts";
import { resolvePluginIcon } from "./icons";
import { buildPluginSurfaceRoute, hostIdFromPathname } from "./routes";
import {
  getPreferredPluginContributionHost,
  rememberPluginContributionHost,
} from "./contribution-host";
import { type PluginSidebarGroup, type PluginSidebarTarget } from "./sidebar-groups";

function selectTarget(
  group: PluginSidebarGroup,
  currentHostId: string | null,
  activeServerId: string | null,
): PluginSidebarTarget | null {
  // An active host pins the plugin to that host; a plugin it does not run is not offered.
  if (activeServerId) {
    return group.targets.find((target) => target.plugin.serverId === activeServerId) ?? null;
  }
  const current = group.targets.find((target) => target.plugin.serverId === currentHostId);
  if (current) return current;
  const rememberedHostId = getPreferredPluginContributionHost(group.key);
  const remembered = group.targets.find((target) => target.plugin.serverId === rememberedHostId);
  return remembered ?? group.targets[0];
}

export function PluginSidebarItemRow({
  group,
  onBeforeNavigate,
}: {
  group: PluginSidebarGroup;
  onBeforeNavigate?: () => void;
}) {
  const pathname = usePathname();
  const activeServerId = useActiveServerId();
  const target = selectTarget(group, hostIdFromPathname(pathname), activeServerId);
  if (!target) return null;
  return (
    <PluginSidebarTargetRow
      group={group}
      target={target}
      pathname={pathname}
      onBeforeNavigate={onBeforeNavigate}
    />
  );
}

function PluginSidebarTargetRow({
  group,
  target,
  pathname,
  onBeforeNavigate,
}: {
  group: PluginSidebarGroup;
  target: PluginSidebarTarget;
  pathname: string;
  onBeforeNavigate?: () => void;
}) {
  const route = buildPluginSurfaceRoute(target.plugin.serverId, group.pluginId, {
    kind: "sidebar",
    id: group.contributionId,
  });
  const isActive = group.targets.some(
    (candidate) =>
      pathname ===
      buildPluginSurfaceRoute(candidate.plugin.serverId, group.pluginId, {
        kind: "sidebar",
        id: group.contributionId,
      }),
  );
  const navigate = useCallback(() => {
    rememberPluginContributionHost(group.key, target.plugin.serverId);
    onBeforeNavigate?.();
    router.push(route);
  }, [group.key, onBeforeNavigate, route, target.plugin.serverId]);
  return (
    <SidebarHeaderRow
      icon={resolvePluginIcon(group.icon)}
      label={group.title}
      onPress={navigate}
      isActive={isActive}
      testID={`plugin-sidebar-${group.pluginId}-${group.contributionId}`}
      variant="compact"
    />
  );
}
