import type { PluginClientContext } from "@getpaseo/plugin/client";
import { settingsRpc } from "@getpaseo/plugin";
import { Platform } from "react-native";
import { SendSettingsScreen } from "./client/settings";
import { backToBoard } from "./client/back-to-board";
import { boardEvent, desktopSupported, install } from "./client/web";
import { hostRpc, inspectRpc, sendRpc, startRpc, toggleRpc } from "./shared/contracts";
import { sendSettings } from "./shared/settings";

export default function contribute(client: PluginClientContext) {
  if (Platform.OS !== "web" || !desktopSupported()) return () => {};
  const toggle = client.addCommandCenterItem({
    id: "toggle-auto-run",
    title: "Toggle Jev next-prompt auto-run",
    icon: "Send",
    context: "agent",
    async onSelect({ agent, workspace, rpc }) {
      const { serverId } = await rpc(hostRpc, {});
      const scope = { serverId, agentId: agent.id, workspaceId: workspace.id };
      const current = await rpc(inspectRpc, scope);
      await rpc(toggleRpc, { ...scope, enabled: !current.enabled });
    },
  });
  // Cached so a click can leave for the Board at once; the settings screen keeps it current.
  void client
    .rpc(settingsRpc(sendSettings.id).read, {})
    .then((saved) => {
      if (saved.status === "ready")
        backToBoard.enabled = sendSettings.schema.parse(saved.values).backToBoard;
    })
    .catch(() => undefined);
  const cleanup = install({
    inspect: (scope) => client.rpc(inspectRpc, scope),
    send: (scope, key) => client.rpc(sendRpc, { ...scope, key }),
    start: (scope, key) => client.rpc(startRpc, { ...scope, key }),
    sending(outcome, scope) {
      if (!backToBoard.enabled) return;
      // Leave before the acknowledgement; the Board refreshes on success and warns on failure.
      boardEvent("paseo-board:open", scope.serverId);
      void outcome.then((sent) =>
        boardEvent(sent ? "paseo-board:sent" : "paseo-board:send-failed", scope.serverId),
      );
    },
  });
  const settings = client.addSettingsScreen({
    id: "send",
    title: "Next prompt actions",
    icon: "Send",
    Component: SendSettingsScreen,
  });
  return () => {
    cleanup();
    settings();
    toggle();
  };
}
