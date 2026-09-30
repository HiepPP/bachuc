import type { PluginClientContext } from "@getpaseo/plugin/client";
import { WatchtowerPanel } from "./client/board";
import { WatchtowerPage } from "./client/page";
import { installSidebarShortcut } from "./client/web";
import { taskAttachments } from "./shared/board";

export default function contribute(client: PluginClientContext) {
  const removeSurface = client.addSurface("watchtower", WatchtowerPage);
  // Sidebar items follow plugin ID order, so "watchtower-board" lands below "board".
  const removeSidebar = client.addSidebarItem({
    id: "watchtower",
    title: "Watchtower",
    icon: "ListTodo",
    surface: "watchtower",
  });
  const removeShortcut = installSidebarShortcut((workspaceId) =>
    client.openPanel("board", { workspaceId, location: "explorer" }),
  );
  const removePanel = client.addWorkspacePanel({
    id: "board",
    title: "Watchtower",
    icon: "ListTodo",
    context: "workspace",
    locations: ["explorer"],
    Component: WatchtowerPanel,
  });
  const removeCommand = client.addCommandCenterItem({
    id: "open-board",
    title: "Open Watchtower board",
    icon: "ListTodo",
    context: "workspace",
    onSelect: ({ openPanel }) => openPanel("board", { location: "explorer" }),
  });
  const removeAttachments = client.addAttachmentSource(taskAttachments);
  return () => {
    removeAttachments();
    removeCommand();
    removePanel();
    removeShortcut();
    removeSidebar();
    removeSurface();
  };
}
