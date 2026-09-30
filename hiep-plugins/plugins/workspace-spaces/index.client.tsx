import type { PluginClientContext } from "@getpaseo/plugin/client";
import { createSidebarController } from "./client/sidebar-state";
import { installSidebar, supportsSidebarInjection } from "./client/web";
import { SpacesPage } from "./client/page";
export default function contribute(client: PluginClientContext) {
  const cleanupSidebar = installSidebar(createSidebarController(client));
  const surface = client.addSurface("spaces", SpacesPage);
  const sidebar = supportsSidebarInjection()
    ? () => {}
    : client.addSidebarItem({
        id: "spaces",
        title: "Spaces",
        icon: "Layers",
        surface: "spaces",
      });
  return () => {
    cleanupSidebar();
    sidebar();
    surface();
  };
}
