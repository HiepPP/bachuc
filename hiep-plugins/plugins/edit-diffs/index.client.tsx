import type { PluginClientContext } from "@getpaseo/plugin/client";
import { EditDiffRow } from "./client/edit-row";
import { DisplaySettingsScreen } from "./client/settings";
import { editRowSchema } from "./shared/contracts";
import { toEditRow } from "./shared/edit-item";

export default function contribute(client: PluginClientContext) {
  const transformer = client.addTimelineTransformer({
    id: "edit-diff",
    query: { itemType: "tool_call" },
    transform({ item }) {
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
  return () => {
    settings();
    renderer();
    transformer();
  };
}
