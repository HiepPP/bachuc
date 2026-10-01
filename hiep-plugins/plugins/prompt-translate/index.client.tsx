import type { PluginClientContext } from "@getpaseo/plugin/client";
import { settingsRpc } from "@getpaseo/plugin";
import { enhanceRpc, modeReadRpc, originalRpc, translateRpc } from "./shared/contracts";
import { translateSettings } from "./shared/settings";
import { hasVietnamese } from "./shared/vietnamese";
import { createNoteCache } from "./client/notes";
import { createNoteView, noteSchema } from "./client/note-view";
import { PromptPrefsPopover } from "./client/prompt-pill";
import { TranslateSettingsScreen } from "./client/settings";
import { current } from "./client/state";

export default function contribute(client: PluginClientContext) {
  void client
    .rpc(settingsRpc(translateSettings.id).read, {})
    .then((saved) => {
      if (saved.status === "ready") current.apply(translateSettings.schema.parse(saved.values));
    })
    .catch(() => undefined);
  const lookup = createNoteCache({
    translate: async (text, cacheOnly) =>
      (await client.rpc(translateRpc, { text, cacheOnly })).translation,
    original: async (text) => (await client.rpc(originalRpc, { text })).original,
  });
  // The native bubble stays; the EN or VI gốc note renders under it on every client.
  const transformer = client.addTimelineTransformer({
    id: "prompt-note",
    query: { itemType: "user_message" },
    transform({ item, phase }) {
      if (phase !== "complete" || !current.values.translate || !item.text.trim()) return undefined;
      return {
        items: [{ type: "plugin", kind: "prompt-note", version: 1, data: { text: item.text } }],
        source: { placement: "first" },
      };
    },
  });
  const renderer = client.addTimelineRenderer({
    kind: "prompt-note",
    version: 1,
    schema: noteSchema,
    Component: createNoteView(lookup, () => current.activeSince),
  });
  const interceptor = client.addComposerInterceptor({
    id: "rewrite-english",
    async intercept({ target, text }) {
      if (!hasVietnamese(text)) return undefined;
      const rewrite = target.agentId
        ? (await client.rpc(modeReadRpc, { agentId: target.agentId })).rewrite
        : current.values.enhanceShortcut;
      if (!rewrite) return undefined;
      return { text: (await client.rpc(enhanceRpc, { text, deferCaveman: true })).prompt };
    },
  });
  const pill = client.addComposerPill({
    id: "prompt-prefs",
    button: {
      title: "Prompt language and Caveman mode",
      icon: "Languages",
      label: "Prompt",
      behavior: { kind: "popover", Content: PromptPrefsPopover },
    },
  });
  const settings = client.addSettingsScreen({
    id: "translate",
    title: "Prompt translate",
    icon: "Languages",
    Component: TranslateSettingsScreen,
  });
  return () => {
    pill.remove();
    interceptor();
    renderer();
    transformer();
    settings();
  };
}
