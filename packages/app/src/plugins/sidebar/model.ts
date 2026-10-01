import type {
  PluginSidebarContext,
  PluginSidebarProject,
  PluginSidebarProjectMenuItem,
} from "@getpaseo/plugin/client";
import type { SidebarProjectEntry } from "@/hooks/use-sidebar-workspaces-list";
import type { InstalledPlugin } from "../types";

/** One installation per plugin: the active host's when it has one, else the first. */
export function selectSidebarPlugins(
  plugins: readonly InstalledPlugin[],
  activeServerId: string | null,
): InstalledPlugin[] {
  const byId = new Map<string, InstalledPlugin>();
  for (const plugin of plugins) {
    const current = byId.get(plugin.id);
    if (!current || (plugin.serverId === activeServerId && current.serverId !== activeServerId)) {
      byId.set(plugin.id, plugin);
    }
  }
  return [...byId.values()];
}

export function toPluginSidebarProject(project: SidebarProjectEntry): PluginSidebarProject {
  return {
    viewKey: project.viewKey,
    name: project.projectName,
    serverIds: project.hosts.map((host) => host.serverId),
    projectIds: project.hosts.map((host) => host.projectId),
  };
}

/** Returns the view keys that plugin filters hide. A throwing filter hides nothing. */
export function hiddenProjectViewKeys(
  plugins: readonly InstalledPlugin[],
  projects: readonly SidebarProjectEntry[],
  context: PluginSidebarContext,
): ReadonlySet<string> {
  const hidden = new Set<string>();
  const filters = plugins.flatMap((plugin) =>
    (plugin.sidebarProjectFilters ?? []).map((filter) => ({ plugin, filter })),
  );
  if (filters.length === 0) return hidden;
  for (const project of projects) {
    const value = toPluginSidebarProject(project);
    for (const { plugin, filter } of filters) {
      try {
        if (!filter.isVisible(value, context)) {
          hidden.add(project.viewKey);
          break;
        }
      } catch (error) {
        console.warn(`[Plugins] Sidebar project filter failed: ${plugin.id}/${filter.id}`, error);
      }
    }
  }
  return hidden;
}

export function pluginProjectMenuItems(
  plugins: readonly InstalledPlugin[],
  project: PluginSidebarProject,
  context: PluginSidebarContext,
): Array<{ key: string; item: PluginSidebarProjectMenuItem }> {
  return plugins.flatMap((plugin) =>
    (plugin.sidebarProjectMenus ?? []).flatMap((menu) => {
      try {
        return menu
          .getItems(project, context)
          .map((item) => ({ key: `${plugin.id}/${menu.id}/${item.id}`, item }));
      } catch (error) {
        console.warn(`[Plugins] Sidebar project menu failed: ${plugin.id}/${menu.id}`, error);
        return [];
      }
    }),
  );
}

/** The first non-empty filter title. A throwing filter is skipped. */
export function pluginSidebarTitle(
  plugins: readonly InstalledPlugin[],
  context: PluginSidebarContext,
): string | null {
  for (const plugin of plugins) {
    for (const filter of plugin.sidebarProjectFilters ?? []) {
      try {
        const title = filter.getTitle?.(context)?.trim();
        if (title) return title;
      } catch (error) {
        console.warn(`[Plugins] Sidebar title failed: ${plugin.id}/${filter.id}`, error);
      }
    }
  }
  return null;
}
