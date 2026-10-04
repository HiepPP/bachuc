import type { PluginClientContext } from "@getpaseo/plugin/client";

export default function contribute(client: PluginClientContext) {
  // App builds before the fork's openPluginsPage API get no row instead of a dead one.
  if (typeof client.openPluginsPage !== "function") return () => {};
  // The sidebar pins the row to the active host, so the press opens that host's Plugins page.
  const removeSurface = client.addSurface("plugins", () => null);
  const removeSidebar = client.addSidebarItem({
    id: "plugins",
    title: "Plugins",
    icon: "Blocks",
    surface: "plugins",
    action: { onPress: () => client.openPluginsPage() },
  });
  return () => {
    removeSidebar();
    removeSurface();
  };
}
