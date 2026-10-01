import type { PluginClientContext } from "@getpaseo/plugin/client";
import { Platform } from "react-native";
import { installNewThreadNavigation } from "./client/new-thread";
import { installParentNavigation } from "./client/parent";
import { installRemoveButtons } from "./client/remove";
import { BoardPage } from "./client/page";
import { OrbSettingsScreen } from "./client/orb-settings";
import { workspaceActions } from "./client/workspace";
import { installBoardShortcut } from "./client/shortcut";
import { ProjectHeaderChip } from "./client/project-header-chip";

export default function contribute(client: PluginClientContext) {
  const surface = client.addSurface("board", BoardPage);
  const sidebar = client.addSidebarItem({
    id: "board",
    title: "Board",
    icon: "Columns3",
    surface: "board",
  });
  const settings = client.addSettingsScreen({
    id: "thinking-orb",
    title: "Thinking orb",
    icon: "Sparkles",
    Component: OrbSettingsScreen,
  });
  workspaceActions.open = (input) => client.openNewWorkspace(input);
  // Opens the Board on the active host; one installation owns the shortcut.
  const shortcut = installBoardShortcut(() => client.openSurface("board"));
  const projectHeader = client.addWorkspaceHeaderSubtitle({
    id: "project",
    Component: ProjectHeaderChip,
  });
  const parent = installParentNavigation(client);
  const newThread = installNewThreadNavigation(client);
  const removeButtons = installRemoveButtons(
    client,
    parent.open,
    Platform.OS === "web" ? newThread.open : undefined,
  );
  return () => {
    projectHeader();
    removeButtons();
    parent.cleanup();
    newThread.cleanup();
    shortcut();
    settings();
    sidebar();
    surface();
  };
}
