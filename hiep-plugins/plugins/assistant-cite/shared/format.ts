export const QUOTE_LEAD = "Quoted from your earlier reply:";
export const MAX_QUOTE_LENGTH = 4000;

export type QuoteFailure = "empty" | "too_long";
export type FormattedQuote =
  | { ok: true; quote: string; text: string }
  | { ok: false; reason: QuoteFailure };

/**
 * The text the agent gets: the lead line, then the quote as a Markdown blockquote, so the agent
 * reads it as reference text. The host appends the chip comment after it.
 */
export function formatQuote(selection: string): FormattedQuote {
  const quote = selection.trim();
  if (!quote) return { ok: false, reason: "empty" };
  if (quote.length > MAX_QUOTE_LENGTH) return { ok: false, reason: "too_long" };
  const blockquote = quote
    .split(/\r?\n/)
    .map((line) => (line ? `> ${line}` : ">"))
    .join("\n");
  return { ok: true, quote, text: `${QUOTE_LEAD}\n\n${blockquote}` };
}

/** Reverses `formatQuote`, or returns null for text it did not make. */
export function quoteFromText(text: string): string | null {
  const prefix = `${QUOTE_LEAD}\n\n`;
  if (!text.startsWith(prefix)) return null;
  return text
    .slice(prefix.length)
    .split("\n")
    .map((line) => line.replace(/^> ?/, ""))
    .join("\n");
}

/**
 * The host highlights rendered text inside one block, so the jump back highlights the first
 * non-empty line of the quote with its Markdown syntax removed.
 */
export function highlightText(quote: string): string | undefined {
  const line = quote
    .split(/\r?\n/)
    .map((candidate) => candidate.trim())
    .find(Boolean);
  if (!line) return undefined;
  const plain = line
    .replace(/^(?:>\s*)+/, "")
    .replace(/^#{1,6}\s+/, "")
    .replace(/^(?:[-*+]|\d+[.)])\s+/, "")
    .replace(/!?\[([^\]]*)\]\([^)]*\)/g, "$1")
    .replace(/(\*\*|__|~~|`)/g, "")
    .trim();
  return plain || undefined;
}
