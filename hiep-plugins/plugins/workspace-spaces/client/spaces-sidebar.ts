import type {
  PluginSidebarContext,
  PluginSidebarFilterChange,
  PluginSidebarProject,
  PluginSidebarProjectMenuItem,
} from "@getpaseo/plugin/client";
import { adjacent, removalTarget } from "../shared/spaces";
import {
  matchProject,
  sidebarMembership,
  viewMembership,
  type SidebarController,
} from "./sidebar-state";

/**
 * Selected Space per host and the sidebar contributions that read it. `activeServerId` comes from
 * the sidebar context; each host keeps its own Spaces and selection.
 */
export function createSpacesSidebar(controller: SidebarController) {
  const selected = new Map<string, string>();
  const listeners = new Set<(change?: PluginSidebarFilterChange) => void>();
  const notify = (change?: PluginSidebarFilterChange) => {
    for (const listener of listeners) listener(change);
  };
  const hostKey = (host: string | null) => host ?? "";
  function snapshot() {
    return controller.get();
  }
  function selectedSpace(host: string | null): string | null {
    const current = snapshot();
    if (!current) return null;
    const saved = selected.get(hostKey(host));
    const spaces = current.state.spaces;
    return spaces.some((space) => space.id === saved) ? saved! : spaces[0].id;
  }
  // The host slides the project list toward the newly selected Space.
  function select(host: string | null, spaceId: string) {
    const ids = snapshot()?.state.spaces.map((space) => space.id) ?? [];
    const from = ids.indexOf(selectedSpace(host) ?? "");
    const to = ids.indexOf(spaceId);
    selected.set(hostKey(host), spaceId);
    notify(from < 0 || to < 0 || from === to ? undefined : { direction: to > from ? 1 : -1 });
  }
  return {
    subscribe(listener: (change?: PluginSidebarFilterChange) => void) {
      listeners.add(listener);
      const stop = controller.subscribe(listener);
      return () => {
        listeners.delete(listener);
        stop();
      };
    },
    selectedSpace,
    select,
    // The new Space is the last one; select it once the save lands.
    async create(host: string | null) {
      if (!(await controller.create())) return false;
      const current = snapshot();
      const created = current?.host === host ? current.state.spaces.at(-1) : undefined;
      if (created) select(host, created.id);
      return true;
    },
    // Select the removal target before the save, so the list never falls back to the first Space
    // in between. A failed save restores the selection.
    async remove(host: string | null, spaceId: string) {
      const current = snapshot();
      const removable =
        current?.host === host &&
        current.state.spaces.length > 1 &&
        current.state.spaces.some((space) => space.id === spaceId);
      const target = removable ? removalTarget(current.state, spaceId) : null;
      const moving = target !== null && selectedSpace(host) === spaceId;
      if (moving) select(host, target);
      if (await controller.remove(spaceId)) return true;
      if (moving) select(host, spaceId);
      return false;
    },
    // A trackpad swipe moves to the adjacent Space without wrapping, and waits out a save.
    onSwipe(direction: 1 | -1, context: PluginSidebarContext) {
      const current = snapshot();
      if (!current || current.busy || current.host !== context.activeServerId) return;
      const ids = current.state.spaces.map((space) => space.id);
      const from = selectedSpace(context.activeServerId) ?? ids[0];
      const next = adjacent(ids, from, direction);
      if (next !== from) select(context.activeServerId, next);
    },
    // The selected Space names the sidebar's Workspaces heading.
    getTitle(context: PluginSidebarContext): string | null {
      const current = snapshot();
      if (!current || current.error || current.host !== context.activeServerId) return null;
      const space = selectedSpace(context.activeServerId);
      return current.state.spaces.find((entry) => entry.id === space)?.name ?? null;
    },
    /** Every catalogued project with the Space it belongs to, for the Move projects page. */
    projects(host: string | null): { id: string; name: string; spaceId: string }[] {
      const current = snapshot();
      if (!current || current.host !== host) return [];
      return current.projects.map((project) => ({
        id: project.id,
        name: project.name,
        spaceId: project.viewKey
          ? viewMembership(current.state, project.viewKey, project)
          : sidebarMembership(current.state, project.id),
      }));
    },
    // A project with a view key moves by that key, so the sidebar row follows it.
    async moveProject(host: string | null, projectId: string, spaceId: string) {
      const current = snapshot();
      const project =
        current?.host === host ? current.projects.find((p) => p.id === projectId) : undefined;
      if (!project) return false;
      return project.viewKey
        ? controller.moveView(project.viewKey, spaceId)
        : controller.move(project.id, spaceId);
    },
    // Errors and loading show every project, so a failure never hides work.
    isVisible(project: PluginSidebarProject, context: PluginSidebarContext): boolean {
      const current = snapshot();
      if (!current || current.error || current.host !== context.activeServerId) return true;
      const space = selectedSpace(context.activeServerId);
      const match = matchProject(project.viewKey, current.projects);
      return viewMembership(current.state, project.viewKey, match) === space;
    },
    menuItems(
      project: PluginSidebarProject,
      context: PluginSidebarContext,
    ): PluginSidebarProjectMenuItem[] {
      const current = snapshot();
      if (!current || current.busy || current.host !== context.activeServerId) return [];
      const match = matchProject(project.viewKey, current.projects);
      const member = viewMembership(current.state, project.viewKey, match);
      // One "Move to" submenu listing every Space, with the current one checked.
      return [
        {
          id: "move",
          title: "Move to",
          items: current.state.spaces.map((space) => ({
            id: space.id,
            title: space.name,
            checked: space.id === member,
            onSelect: async () => {
              if (space.id === member) return;
              if (!(await controller.moveView(project.viewKey, space.id)))
                throw new Error(controller.get()?.error || "Move failed");
            },
          })),
        },
      ];
    },
  };
}
export type SpacesSidebar = ReturnType<typeof createSpacesSidebar>;
