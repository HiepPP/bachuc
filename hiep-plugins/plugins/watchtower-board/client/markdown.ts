// A small reader for ADR files: headings, bullets, paragraphs, and fenced code. It keeps inline
// marks such as backticks and links as text, and it never renders HTML.
export type MarkdownBlock =
  | { kind: "heading"; level: number; text: string }
  | { kind: "bullet"; depth: number; text: string }
  | { kind: "paragraph"; text: string }
  | { kind: "code"; text: string };

export function parseMarkdown(markdown: string): MarkdownBlock[] {
  const blocks: MarkdownBlock[] = [];
  let paragraph: string[] = [];
  let code: string[] | null = null;
  const flush = () => {
    if (paragraph.length > 0) blocks.push({ kind: "paragraph", text: paragraph.join("\n") });
    paragraph = [];
  };
  for (const line of markdown.split(/\r?\n/)) {
    if (/^\s{0,3}(```|~~~)/.test(line)) {
      if (code) {
        blocks.push({ kind: "code", text: code.join("\n") });
        code = null;
      } else {
        flush();
        code = [];
      }
      continue;
    }
    if (code) {
      code.push(line);
      continue;
    }
    const heading = line.match(/^\s{0,3}(#{1,6})\s+(.*?)\s*$/);
    const bullet = line.match(/^(\s*)[-*]\s+(.*)$/);
    if (heading) {
      flush();
      blocks.push({ kind: "heading", level: heading[1].length, text: heading[2] });
    } else if (bullet) {
      flush();
      blocks.push({ kind: "bullet", depth: Math.floor(bullet[1].length / 2), text: bullet[2] });
    } else if (line.trim() === "") {
      flush();
    } else {
      paragraph.push(line.trim());
    }
  }
  flush();
  // An unclosed fence still shows what it held.
  if (code) blocks.push({ kind: "code", text: code.join("\n") });
  return blocks;
}
