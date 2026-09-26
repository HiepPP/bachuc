import type { PluginClientContext } from "@getpaseo/plugin/client";
import { Platform } from "react-native";
import { installNewThreadNavigation } from "./client/new-thread";
import { installParentNavigation } from "./client/parent";
import { installRemoveButtons } from "./client/remove";
import { installRemovePlacement, openBoardFromSidebar } from "./client/web";
import { BoardPage } from "./client/page";
import { OrbSettingsScreen } from "./client/orb-settings";
import { installBoardEvents } from "./client/events";
import { installBoardShortcut } from "./client/shortcut";
import { installProjectHeader } from "./client/project-header";

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
  const openBoard = () => {
    if (!openBoardFromSidebar()) client.openSurface("board");
  };
  const shortcut = installBoardShortcut(openBoard);
  const events = installBoardEvents(openBoard);
  const removePlacement = installRemovePlacement();
  const projectHeader = installProjectHeader(client);
  const parent = installParentNavigation(client);
  const newThread = installNewThreadNavigation(client);
  const removeButtons = installRemoveButtons(
    client,
    parent.open,
    Platform.OS === "web" ? newThread.open : undefined,
  );
  return () => {
    removePlacement();
    projectHeader();
    removeButtons();
    parent.cleanup();
    newThread.cleanup();
    shortcut();
    events();
    settings();
    sidebar();
    surface();
  };
}
