import type { PluginClientContext } from "@getpaseo/plugin/client";
import { settingsRpc } from "@getpaseo/plugin";
import { SendSettingsScreen } from "./client/settings";
import { backToBoard } from "./client/back-to-board";
import { createNextPromptPanel, panelSchema } from "./client/panel";
import { hostRpc, inspectRpc, toggleRpc } from "./shared/contracts";
import { sectionIntro, splitNextSection, splitRecap } from "./shared/section";
import { sendSettings } from "./shared/settings";

export default function contribute(client: PluginClientContext) {
  // Offered only while Jev can judge; re-enabling jev-evaluator needs a reload of this plugin.
  let stopped = false;
  let toggle: (() => void) | undefined;
  void client
    .rpc(hostRpc, {})
    .then(({ evaluator }) => {
      if (stopped || !evaluator) return;
      toggle = client.addCommandCenterItem({
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
    })
    .catch(() => undefined);
  // Cached so a click can leave for the Board at once; the settings screen keeps it current.
  void client
    .rpc(settingsRpc(sendSettings.id).read, {})
    .then((saved) => {
      if (saved.status === "ready")
        backToBoard.enabled = sendSettings.schema.parse(saved.values).backToBoard;
    })
    .catch(() => undefined);
  // The reply keeps its native Markdown up to the What Next heading; the panel replaces the rest.
  const transformer = client.addTimelineTransformer({
    id: "next-prompt-panel",
    query: { itemType: "assistant_message" },
    transform({ item, phase }) {
      if (phase !== "complete") return undefined;
      const split = splitNextSection(item.text);
      if (!split) return undefined;
      const recap = splitRecap(split.before);
      const panel = {
        type: "plugin" as const,
        kind: "next-prompt-panel",
        version: 1,
        data: {
          title: split.title,
          intro: sectionIntro(split.section),
          section: split.section,
          ...(recap.recap ? { recap: recap.recap } : {}),
        },
      };
      return recap.before
        ? { items: [panel], source: { placement: "first" as const, text: recap.before } }
        : { items: [panel] };
    },
  });
  const renderer = client.addTimelineRenderer({
    kind: "next-prompt-panel",
    version: 1,
    schema: panelSchema,
    Component: createNextPromptPanel(client),
  });
  const settings = client.addSettingsScreen({
    id: "send",
    title: "Next prompt actions",
    icon: "Send",
    Component: SendSettingsScreen,
  });
  return () => {
    renderer();
    transformer();
    settings();
    stopped = true;
    toggle?.();
  };
}
