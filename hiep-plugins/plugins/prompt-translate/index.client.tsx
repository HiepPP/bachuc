import type { PluginClientContext } from "@getpaseo/plugin/client";
import { settingsRpc } from "@getpaseo/plugin";
import { enhanceRpc, originalRpc, translateRpc } from "./shared/contracts";
import { rewritesOnSendKey, translateSettings } from "./shared/settings";
import { hasVietnamese } from "./shared/vietnamese";
import { createNoteCache } from "./client/notes";
import { createNoteView, noteSchema } from "./client/note-view";
import { CavemanModeMenu, PromptPillLabel } from "./client/prompt-pill";
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
    async intercept({ text, sendKey }) {
      // The host setting decides; the pill only picks the Caveman mode.
      if (!current.values.enhanceShortcut || !text.trim()) return undefined;
      // The key that does not rewrite in this mode sends the draft as typed.
      if (!rewritesOnSendKey(sendKey, current.values.rewriteMode)) return undefined;
      // A one-line English command such as /compact or $caveman ultra reaches the agent as typed.
      if (!hasVietnamese(text) && /^[/$][^\n]*$/.test(text.trim())) return undefined;
      return { text: (await client.rpc(enhanceRpc, { text, deferCaveman: true })).prompt };
    },
  });
  const pill = client.addComposerPill({
    id: "prompt-prefs",
    showOnDraft: true,
    placement: "toolbar",
    button: {
      title: "Caveman mode",
      label: PromptPillLabel,
      behavior: { kind: "popover", Content: CavemanModeMenu, flush: true },
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
