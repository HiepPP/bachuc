import { router, usePathname } from "expo-router";
import { useCallback } from "react";
import { SidebarHeaderRow } from "@/components/sidebar/sidebar-header-row";
import { useIsCompactFormFactor } from "@/constants/layout";
import { useActiveServerId } from "@/hosts/use-visible-hosts";
import { isNewWorkspaceScreenActive } from "@/screens/new-workspace/screen-presence";
import { useActiveWorkspaceSelection } from "@/stores/navigation-active-workspace-store";
import { resolvePluginIcon } from "./icons";
import { buildPluginSurfaceRoute, hostIdFromPathname } from "./routes";
import {
  getPreferredPluginContributionHost,
  rememberPluginContributionHost,
} from "./contribution-host";
import {
  newWorkspacePanelKey,
  useNewWorkspaceSidePanelStore,
} from "./new-workspace-side-panel/store";
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
  const selection = useActiveWorkspaceSelection();
  const workspaceId = selection?.serverId === target.plugin.serverId ? selection.workspaceId : null;
  const action = target.item.action;
  const compact = useIsCompactFormFactor();
  const newWorkspaceTarget = useNewWorkspaceSidePanelStore((state) => state.target);
  const panelId = action?.newWorkspacePanel;
  const panelKey =
    panelId !== undefined &&
    !compact &&
    isNewWorkspaceScreenActive({ isMounted: newWorkspaceTarget !== null, pathname }) &&
    newWorkspaceTarget?.serverId === target.plugin.serverId &&
    target.plugin.newWorkspacePanels?.some((panel) => panel.id === panelId)
      ? newWorkspacePanelKey(target.plugin.id, panelId)
      : null;
  const press = useCallback(() => {
    rememberPluginContributionHost(group.key, target.plugin.serverId);
    onBeforeNavigate?.();
    if (panelKey) {
      useNewWorkspaceSidePanelStore.getState().show(panelKey);
      return;
    }
    if (!action) {
      router.push(route);
      return;
    }
    try {
      action.onPress({ workspaceId });
    } catch (error) {
      console.warn(`[Plugins] Sidebar action failed for ${group.key}`, error);
    }
  }, [action, group.key, onBeforeNavigate, panelKey, route, target.plugin.serverId, workspaceId]);
  return (
    <SidebarHeaderRow
      icon={resolvePluginIcon(group.icon)}
      label={group.title}
      onPress={press}
      disabled={Boolean(action?.requiresWorkspace) && !workspaceId && !panelKey}
      isActive={isActive}
      testID={`plugin-sidebar-${group.pluginId}-${group.contributionId}`}
      variant="compact"
    />
  );
}
