import type { PluginClientContext } from "@getpaseo/plugin/client";
import { createSidebarController } from "./client/sidebar-state";
import { createSpacesSidebar } from "./client/spaces-sidebar";
import { createSpacesStrip } from "./client/spaces-strip";
import { SpacesPage } from "./client/page";

export default function contribute(client: PluginClientContext) {
  // The sidebar reports the active host; Spaces are saved per host.
  let activeHost: string | null = null;
  const controller = createSidebarController(client, () => activeHost);
  const sidebar = createSpacesSidebar(controller);
  const filter = client.addSidebarProjectFilter({
    id: "spaces",
    subscribe: sidebar.subscribe,
    isVisible: sidebar.isVisible,
    onSwipe: sidebar.onSwipe,
    getTitle: sidebar.getTitle,
    onProjectAdded: sidebar.onProjectAdded,
  });
  const menu = client.addSidebarProjectMenuItems({ id: "move", getItems: sidebar.menuItems });
  const section = client.addSidebarSection({
    id: "spaces",
    Component: createSpacesStrip(controller, sidebar, (host) => {
      if (host === activeHost) return;
      activeHost = host;
      controller.notify();
    }),
  });
  const surface = client.addSurface("spaces", SpacesPage);
  void controller.refresh();
  // Pick up Space and project changes made elsewhere.
  const timer = setInterval(() => void controller.refresh(), 15000);
  return () => {
    clearInterval(timer);
    controller.stop();
    section();
    menu();
    filter();
    surface();
  };
}
