import type { PluginClientContext } from "@getpaseo/plugin/client";
import { WatchtowerProjectPanel } from "./client/board";
import { preselectWorkspace, WatchtowerPage } from "./client/page";
import { WatchtowerPillLabel } from "./client/pill";
import { taskAttachments } from "./shared/board";

// The composer pill, the Command Center, and Mod+E open the page over the current screen. There
// is no sidebar item and no Explorer panel, so the board has a single view.
export default function contribute(client: PluginClientContext) {
  const removeSurface = client.addSurface("watchtower", WatchtowerPage, {
    title: "Watchtower",
    icon: "ListTodo",
  });
  // Mod+E toggles the board: it opens on the active workspace, and closes when it is open.
  // COMPAT(addShortcut): added 2026-10-10; release builds before then lack the method and ignore
  // the surface title. Remove after 2026-11-10, once release is rebuilt.
  const removeShortcut =
    client.addShortcut?.({
      id: "toggle-board",
      combo: "Mod+E",
      surface: "watchtower",
      onPress: ({ workspaceId }) => {
        if (!workspaceId) return;
        preselectWorkspace(workspaceId);
        client.openSurface("watchtower", { presentation: "overlay" });
      },
    }) ?? (() => {});
  // COMPAT(newWorkspacePanel): added 2026-10-06; apps built earlier lack the method. Remove after
  // 2026-11-06, once release is rebuilt.
  const removeNewWorkspacePanel =
    client.addNewWorkspacePanel?.({
      id: "board",
      title: "Watchtower",
      icon: "ListTodo",
      Component: WatchtowerProjectPanel,
    }) ?? (() => {});
  const removeCommand = client.addCommandCenterItem({
    id: "open-board",
    title: "Open Watchtower board",
    icon: "ListTodo",
    context: "workspace",
    onSelect: ({ workspace }) => {
      preselectWorkspace(workspace.id);
      client.openSurface("watchtower", { presentation: "overlay" });
    },
  });
  const removeAttachments = client.addAttachmentSource(taskAttachments);
  // One unscoped pill serves every agent composer and New workspace (Q-002). Its label reads the
  // composer's own board; a press opens the overview surface over the current screen on that
  // workspace. New workspace has no workspace yet, so the host shows the chosen project's board in
  // the side panel instead.
  const pill = client.addComposerPill({
    id: "watchtower",
    showOnDraft: true,
    button: {
      title: "Open Watchtower",
      icon: "ListTodo",
      label: WatchtowerPillLabel,
      behavior: {
        kind: "action",
        newWorkspacePanel: "board",
        onPress: (context) => {
          if (context && context.context !== "draft") preselectWorkspace(context.workspaceId);
          client.openSurface("watchtower", { presentation: "overlay" });
        },
      },
    },
  });
  return () => {
    pill.remove();
    removeAttachments();
    removeCommand();
    removeNewWorkspacePanel();
    removeShortcut();
    removeSurface();
  };
}
