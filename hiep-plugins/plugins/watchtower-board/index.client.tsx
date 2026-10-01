import type { PluginClientContext } from "@getpaseo/plugin/client";
import { Platform } from "react-native";
import { WatchtowerPanel } from "./client/board";
import { WatchtowerPage } from "./client/page";
import { taskAttachments } from "./shared/board";

export default function contribute(client: PluginClientContext) {
  const removeSurface = client.addSurface("watchtower", WatchtowerPage);
  // Sidebar items follow plugin ID order, so "watchtower-board" lands below "board".
  // On web the row opens the current thread's Explorer board, like the Command Center item.
  // Native keeps the surface, which picks a workspace itself.
  const removeSidebar = client.addSidebarItem({
    id: "watchtower",
    title: "Watchtower",
    icon: "ListTodo",
    surface: "watchtower",
    action:
      Platform.OS === "web"
        ? {
            requiresWorkspace: true,
            onPress: ({ workspaceId }) => {
              if (workspaceId) client.openPanel("board", { workspaceId, location: "explorer" });
            },
          }
        : undefined,
  });
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
    removeSidebar();
    removeSurface();
  };
}
