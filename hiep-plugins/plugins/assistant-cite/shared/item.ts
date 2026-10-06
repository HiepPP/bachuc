import type { PluginAttachmentItem } from "@getpaseo/plugin";
import { formatQuote, highlightText, quoteFromText, type QuoteFailure } from "./format";

export const QUOTE_SOURCE_ID = "quote";
const SOURCE_URL = "paseo-cite://quote";
const TITLE_WORDS = 8;
const TITLE_LENGTH = 60;

export interface QuoteSource {
  serverId: string;
  agentId: string;
  messageId?: string;
}

export interface QuotePassage {
  serverId: string;
  agentId: string;
  messageId: string;
  text?: string;
}

export type QuoteItemResult =
  | { ok: true; item: PluginAttachmentItem }
  | { ok: false; reason: QuoteFailure };

// FNV-1a: a short stable id, so citing the same passage again replaces its chip.
function hash(value: string): string {
  let result = 0x811c9dc5;
  for (let index = 0; index < value.length; index += 1) {
    result ^= value.charCodeAt(index);
    result = Math.imul(result, 0x01000193);
  }
  return (result >>> 0).toString(36);
}

function firstWords(quote: string): string {
  const words = quote.split(/\s+/).filter(Boolean);
  let title = words.slice(0, TITLE_WORDS).join(" ");
  if (title.length > TITLE_LENGTH) title = title.slice(0, TITLE_LENGTH).trimEnd();
  return title.length < words.join(" ").length ? `${title}…` : title;
}

/**
 * The chip item keeps its source in `url` and the quote in `text`, so a chip opens its source
 * after a reload and on another client.
 */
export function buildQuoteItem(source: QuoteSource, selection: string): QuoteItemResult {
  const formatted = formatQuote(selection);
  if (!formatted.ok) return formatted;
  const params = new URLSearchParams({ server: source.serverId, agent: source.agentId });
  if (source.messageId) params.set("message", source.messageId);
  return {
    ok: true,
    item: {
      id: `${source.messageId ?? source.agentId}:${hash(formatted.quote)}`,
      identifier: "reply",
      title: firstWords(formatted.quote),
      url: `${SOURCE_URL}?${params.toString()}`,
      text: formatted.text,
      resourceType: "quote",
    },
  };
}

/** The passage a chip jumps back to, or null when the chip has no source message. */
export function parseQuoteItem(item: PluginAttachmentItem): QuotePassage | null {
  if (!item.url.startsWith(`${SOURCE_URL}?`)) return null;
  const params = new URLSearchParams(item.url.slice(SOURCE_URL.length + 1));
  const serverId = params.get("server");
  const agentId = params.get("agent");
  const messageId = params.get("message");
  if (!serverId || !agentId || !messageId) return null;
  const quote = quoteFromText(item.text);
  const text = quote ? highlightText(quote) : undefined;
  return { serverId, agentId, messageId, ...(text ? { text } : {}) };
}
