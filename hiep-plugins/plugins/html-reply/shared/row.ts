import { defineRpc } from "@getpaseo/plugin";
import { z } from "zod";

export const HTML_REPLY_KIND = "html-reply";
export const HTML_REPLY_VERSION = 1;
export const MAX_TITLE_LENGTH = 200;
/** Page ids name files, so they stay plain: lowercase letters, digits, and hyphens. */
export const PAGE_ID = /^[a-z0-9-]{1,64}$/;

// The row holds only the page id and title; the page itself is a file, because row data is
// capped at 64 KiB and a page may be up to 1 MiB.
export const htmlReplyRowSchema = z.object({
  id: z.string().regex(PAGE_ID),
  title: z.string().max(MAX_TITLE_LENGTH),
});
export type HtmlReplyRow = z.output<typeof htmlReplyRowSchema>;

export const getHtmlReplyRpc = defineRpc({
  name: "html-reply.get",
  input: z.object({ id: z.string().regex(PAGE_ID) }),
  output: z.object({ html: z.string().nullable() }),
});

// C0 and C1 control characters, which JSON would escape to up to six bytes each.
function isControl(character: string): boolean {
  const code = character.codePointAt(0) ?? 0;
  return code <= 0x1f || (code >= 0x7f && code <= 0x9f);
}

/** A one-line title of at most 200 characters, so a row stays under 1 KiB. */
export function cleanTitle(title: string): string {
  return Array.from(title, (character) => (isControl(character) ? " " : character))
    .join("")
    .replace(/\s+/g, " ")
    .trim()
    .slice(0, MAX_TITLE_LENGTH);
}

/** Serialized size of the row data, as the daemon measures it. */
export function rowDataBytes(row: HtmlReplyRow): number {
  return new TextEncoder().encode(JSON.stringify(row)).length;
}
