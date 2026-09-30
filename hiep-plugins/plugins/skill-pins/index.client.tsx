import type { PluginClientContext } from "@getpaseo/plugin/client";
import { Platform } from "react-native";
import {
  bindQueueSkillsRpc,
  cancelQueueSkillsRpc,
  hostRpc,
  prepareSkillsRpc,
} from "./shared/contracts";
import { composerHost, desktopSupported, type Doc, type El, type Observer } from "./client/dom";
import { createSkillState } from "./client/state";
import { installSkillsMenu } from "./client/skills-menu";
import { installQueueSnapshots } from "./client/queue";
import { SkillPinsSettingsScreen } from "./client/settings";

export default function contribute(client: PluginClientContext) {
  const settings = client.addSettingsScreen({
    id: "skill-pins",
    title: "Skill pins",
    icon: "Pin",
    Component: SkillPinsSettingsScreen,
  });
  if (Platform.OS !== "web" || !desktopSupported()) return settings;
  const scope = globalThis as unknown as { document: Doc; MutationObserver: Observer };
  let hostId: string | null = null;
  let stopped = false;
  // Every connected host evaluates its own bundle in the same document.
  const owns = (node: El) => hostId !== null && composerHost(node) === hostId;
  const state = createSkillState(client);
  const menu = installSkillsMenu(scope.document, scope.MutationObserver, state, owns);
  const queue = installQueueSnapshots(
    {
      skills: (agentId) => state.get(agentId),
      prepare: (input) => client.rpc(prepareSkillsRpc, input),
      bindQueue: (agentId, token, queueId) =>
        client.rpc(bindQueueSkillsRpc, { agentId, token, queueId }),
      cancelQueue: (agentId, queueId) => client.rpc(cancelQueueSkillsRpc, { agentId, queueId }),
    },
    scope.document,
    scope.MutationObserver,
    owns,
  );
  void client
    .rpc(hostRpc, {})
    .then(({ serverId }) => {
      if (stopped) return;
      hostId = serverId;
      menu.scan();
      menu.update();
      queue.scan();
    })
    .catch((error) => console.warn("[skill-pins] Host lookup failed", error));
  return () => {
    settings();
    stopped = true;
    hostId = null;
    menu.stop();
    queue.stop();
  };
}
