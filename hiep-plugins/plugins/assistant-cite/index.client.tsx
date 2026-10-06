import type { PluginClientContext } from "@getpaseo/plugin/client";
import { MAX_QUOTE_LENGTH } from "./shared/format";
import { buildQuoteItem, parseQuoteItem, QUOTE_SOURCE_ID } from "./shared/item";

export default function contribute(client: PluginClientContext) {
  // A chip-only source: it stays out of the attachment picker and opens quote chips.
  const removeSource = client.addAttachmentSource({
    id: QUOTE_SOURCE_ID,
    title: "Quote",
    icon: "Quote",
    onOpen: (item) => {
      const passage = parseQuoteItem(item);
      if (passage) client.revealTimelinePassage(passage);
    },
  });
  const removeAction = client.addAssistantSelectionAction({
    id: "cite",
    title: "Cite",
    onSelect: ({ serverId, agentId, messageId, text }) => {
      const built = buildQuoteItem({ serverId, agentId, messageId }, text);
      if (!built.ok) {
        // Plugins have no toast API yet (Q-011), so an over-long quote only logs.
        if (built.reason === "too_long") {
          console.warn(`[assistant-cite] Quotes are limited to ${MAX_QUOTE_LENGTH} characters`);
        }
        return;
      }
      client.addComposerAttachment({
        agentId,
        sourceId: QUOTE_SOURCE_ID,
        sourceTitle: "Quote",
        icon: "Quote",
        item: built.item,
        commentable: true,
      });
    },
  });
  return () => {
    removeAction();
    removeSource();
  };
}
