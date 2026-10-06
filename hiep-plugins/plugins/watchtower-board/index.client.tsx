import type { PluginClientContext } from "@getpaseo/plugin/client";
import { Dimensions, Platform } from "react-native";
import { WatchtowerPanel, WatchtowerProjectPanel } from "./client/board";
import { isCompactWidth } from "./client/dashboard";
import { preselectWorkspace, WatchtowerPage } from "./client/page";
import { taskAttachments } from "./shared/board";

export default function contribute(client: PluginClientContext) {
  const removeSurface = client.addSurface("watchtower", WatchtowerPage);
  // COMPAT(newWorkspacePanel): added 2026-10-06; apps built earlier lack the method. Remove after
  // 2026-11-06, once release is rebuilt.
  const removeNewWorkspacePanel =
    client.addNewWorkspacePanel?.({
      id: "board",
      title: "Watchtower",
      icon: "ListTodo",
      Component: WatchtowerProjectPanel,
    }) ?? (() => {});
  // Sidebar items follow plugin ID order, so "watchtower-board" lands below "board".
  // On web the row opens the current thread's Explorer board, like the Command Center item,
  // or the project's board in the new workspace side panel. A compact web window opens the
  // surface on the current workspace, because its Explorer has no plugin tabs. Native keeps
  // the surface, which picks a workspace itself.
  const removeSidebar = client.addSidebarItem({
    id: "watchtower",
    title: "Watchtower",
    icon: "ListTodo",
    surface: "watchtower",
    action:
      Platform.OS === "web"
        ? {
            requiresWorkspace: true,
            newWorkspacePanel: "board",
            onPress: ({ workspaceId }) => {
              if (!workspaceId) return;
              if (isCompactWidth(Dimensions.get("window").width)) {
                preselectWorkspace(workspaceId);
                client.openSurface("watchtower");
                return;
              }
              client.openPanel("board", { workspaceId, location: "explorer" });
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
    removeNewWorkspacePanel();
    removeSurface();
  };
}
