import { parsePrompts } from "./prompts";

const HEADING = /^ {0,3}#{1,6}\s+(.+?)\s*#*\s*$/;
const FENCE = /^ {0,3}(`{3,}|~{3,})/;

/**
 * Splits a reply at its last top-level "What Next" or "Next Steps" heading. Returns null when the
 * section holds no eligible suggestion block, so the reply renders unchanged.
 */
export function splitNextSection(
  markdown: string,
): { before: string; title: string; section: string } | null {
  const lines = markdown.replace(/\r\n/g, "\n").split("\n");
  let fence: string | null = null;
  let start = -1;
  for (let index = 0; index < lines.length; index++) {
    const opening = FENCE.exec(lines[index])?.[1];
    if (opening) {
      if (fence === null) fence = opening[0];
      else if (opening[0] === fence) fence = null;
      continue;
    }
    if (fence !== null) continue;
    const heading = HEADING.exec(lines[index])?.[1]?.trim();
    if (heading && /^(?:what(?:['’]s)? next|next steps)$/i.test(heading)) start = index;
  }
  if (start < 0) return null;
  const section = lines.slice(start).join("\n");
  if (parsePrompts(section).length === 0) return null;
  return {
    before: lines.slice(0, start).join("\n").trimEnd(),
    title: HEADING.exec(lines[start])![1].trim(),
    section,
  };
}

/** Text lines of the section outside the heading and fences, for the panel intro. */
export function sectionIntro(section: string): string {
  const lines = section.split("\n").slice(1);
  const kept: string[] = [];
  let fence: string | null = null;
  for (const line of lines) {
    const opening = FENCE.exec(line)?.[1];
    if (opening) {
      if (fence === null) fence = opening[0];
      else if (opening[0] === fence) fence = null;
      continue;
    }
    if (fence === null) kept.push(line);
  }
  return kept.join("\n").trim();
}
