import type { PluginClientContext } from "@getpaseo/plugin/client";
import { EditDiffRow } from "./client/edit-row";
import { DisplaySettingsScreen } from "./client/settings";
import { editRowSchema } from "./shared/contracts";
import { toEditRow } from "./shared/edit-item";
import { EDIT_DIFFS_MODE_LABEL, editDiffsOff } from "./shared/thread-mode";

export default function contribute(client: PluginClientContext) {
  const transformer = client.addTimelineTransformer({
    id: "edit-diff",
    query: { itemType: "tool_call" },
    transform({ item, agent }) {
      // Hosts without the agent input pass none; those threads keep their diffs.
      // An off thread keeps Paseo's own rows, which collapse into the tool call groups.
      if (editDiffsOff(agent?.labels)) return undefined;
      const data = toEditRow(item);
      if (!data) return undefined;
      return { items: [{ type: "plugin", kind: "edit-diff", version: 1, data: { ...data } }] };
    },
  });
  const renderer = client.addTimelineRenderer({
    kind: "edit-diff",
    version: 1,
    schema: editRowSchema,
    Component: EditDiffRow,
  });
  const settings = client.addSettingsScreen({
    id: "display",
    title: "Edit diffs",
    icon: "Pencil",
    Component: DisplaySettingsScreen,
  });
  const toggle = client.addCommandCenterItem({
    id: "toggle-thread",
    title: "Turn edit diffs on or off in this thread",
    icon: "PencilOff",
    keywords: ["edit", "diff", "thread"],
    context: "agent",
    async onSelect({ agent, paseo }) {
      await paseo.agents
        .ref(agent.id)
        .setLabels({ [EDIT_DIFFS_MODE_LABEL]: editDiffsOff(agent.labels) ? "on" : "off" });
    },
  });
  return () => {
    toggle();
    settings();
    renderer();
    transformer();
  };
}
