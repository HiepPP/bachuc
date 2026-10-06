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

// `notYet` and `need` exist only in the five-field Recap; they select the labeled sections.
// `sections` exists only in the compact Recap and keeps the labels as the reply wrote them.
export type Recap = {
  branch: string;
  did: string;
  commit: string;
  notYet?: string;
  need?: string;
  sections?: { label: string; text: string }[];
};

const FULL = ["branch", "commit/push", "did", "not yet", "need from you"];
// Only free-text fields hold a list: nested in the item, or as the block after the line.
const FREE = new Set(["did", "not yet", "need from you"]);
const LABEL = /^(?:\*\*)?(Branch|Commit\/push|Did|Not yet|Need from you):(?:\*\*)?\s*(.*?)\s*$/i;
const BULLET = /^ {0,3}[-*+]\s+(.*)$/;
const LIST = /^ {0,3}(?:[-*+]|\d{1,9}[.)])\s+\S/;
const NOTHING = /^nothing\.?$/i;
const indentOf = (line: string) => line.length - line.trimStart().length;

/** Branch, Did, Commit/push as lines or list items; only Did may hold indented lines. */
function legacyRecap(body: string[]): Recap | null {
  const fields: string[] = [];
  const labels = ["branch", "did", "commit/push"];
  for (const line of body) {
    if (!line.trim()) continue;
    const match = /^\s*(?:[-*]\s+)?(?:\*\*)?(Branch|Did|Commit\/push):(?:\*\*)?\s*(.+?)\s*$/.exec(
      line,
    );
    if (match) {
      if (match[1].toLowerCase() !== labels[fields.length]) return null;
      fields.push(match[2]);
    } else if (fields.length === 2 && /^\s+\S/.test(line)) {
      fields[1] += `\n${line}`;
    } else return null;
  }
  if (fields.length !== 3) return null;
  return { branch: fields[0], did: fields[1], commit: fields[2] };
}

/**
 * The five fields in order, as one bullet list of five items or as labeled lines. A list item may
 * nest a list, and a labeled line may be followed by a top-level list, in free-text fields only.
 * An empty value needs such a list. Anything else is not a Recap.
 */
function fullRecap(body: string[]): Recap | null {
  const lines = body.filter((line) => line.trim()).map((line) => line.replace(/\t/g, "    "));
  if (!lines.length) return null;
  const listed = BULLET.test(lines[0]);
  const base = indentOf(lines[0]);
  const fields: { text: string; rest: string[] }[] = [];
  for (const line of lines) {
    const indent = indentOf(line);
    const current = fields[fields.length - 1];
    const holds = current && FREE.has(FULL[fields.length - 1]);
    const nested = listed
      ? indent >= base + 2
      : LIST.test(line) || (indent >= 2 && !!current?.rest.length);
    if (nested) {
      if (!holds) return null;
      current.rest.push(line);
      continue;
    }
    const label = LABEL.exec(
      listed ? (BULLET.exec(line)?.[1] ?? "") : indent <= 3 ? line.trim() : "",
    );
    if (!label || label[1].toLowerCase() !== FULL[fields.length]) return null;
    fields.push({ text: label[2], rest: [] });
  }
  if (fields.length !== FULL.length || fields.some((field) => !field.text && !field.rest.length))
    return null;
  const [branch, commit, did, notYet, need] = fields.map(({ text, rest }) => {
    const margin = Math.min(...rest.map(indentOf));
    return [text, ...rest.map((line) => line.slice(margin))].filter(Boolean).join("\n");
  });
  return { branch, did, commit, notYet, need };
}

// Each compact field accepts its English or Vietnamese label from the global rules.
const COMPACT_FIELDS = [
  ["did", "đã làm"],
  ["open", "còn lại"],
  ["need from you", "cần bạn"],
];
const COMPACT_LABEL =
  /^(?:\*\*)?(Did|Đã làm|Open|Còn lại|Need from you|Cần bạn):(?:\*\*)?\s*(.*?)\s*$/iu;
const BRANCH_STATE = /^(.+?)\s+·\s+(.+)$/;
const UNCOMMITTED = /^(no changes|not committed|không có thay đổi|chưa commit)(?=$|[\s(,.;:])/iu;

/**
 * The global rules' Recap, as one bullet list: `<branch> · <commit state>`, then Did, then the
 * optional Open and Need from you, in that order. Did, Open, and Need from you may nest a list.
 */
function compactRecap(body: string[]): Recap | null {
  const lines = body.filter((line) => line.trim()).map((line) => line.replace(/\t/g, "    "));
  const head = BRANCH_STATE.exec(BULLET.exec(lines[0] ?? "")?.[1] ?? "");
  if (!head) return null;
  const base = indentOf(lines[0]);
  const fields: { index: number; label: string; text: string; rest: string[] }[] = [];
  for (const line of lines.slice(1)) {
    const current = fields[fields.length - 1];
    if (indentOf(line) >= base + 2) {
      if (!current) return null;
      current.rest.push(line);
      continue;
    }
    const label = COMPACT_LABEL.exec(BULLET.exec(line)?.[1] ?? "");
    if (!label) return null;
    const index = COMPACT_FIELDS.findIndex((names) => names.includes(label[1].toLowerCase()));
    if (current ? index <= current.index : index !== 0) return null;
    fields.push({ index, label: label[1], text: label[2], rest: [] });
  }
  if (!fields.length || fields.some((field) => !field.text && !field.rest.length)) return null;
  const sections = fields.map(({ label, text, rest }) => {
    const margin = Math.min(...rest.map(indentOf));
    return {
      label,
      text: [text, ...rest.map((line) => line.slice(margin))].filter(Boolean).join("\n"),
    };
  });
  return { branch: head[1], commit: head[2], did: sections[0].text, sections };
}

/** Fold only a complete, recognized Recap immediately before What Next. Unknown prose stays native. */
export function splitRecap(before: string): { before: string; recap?: Recap } {
  const lines = before.split("\n");
  let fence: string | null = null;
  let start = -1;
  for (let i = 0; i < lines.length; i++) {
    const opening = FENCE.exec(lines[i])?.[1];
    if (opening) {
      if (fence === null) fence = opening;
      else if (opening[0] === fence[0] && opening.length >= fence.length) fence = null;
      continue;
    }
    if (fence !== null) continue;
    const heading = HEADING.exec(lines[i])?.[1]?.trim();
    if (heading) start = /^recap$/i.test(heading) ? i : -1;
  }
  if (start < 0) return { before };
  const body = lines.slice(start + 1);
  const recap = legacyRecap(body) ?? fullRecap(body) ?? compactRecap(body);
  if (!recap) return { before };
  return { before: lines.slice(0, start).join("\n").trimEnd(), recap };
}

/**
 * Labeled sections of a five-field Recap: Did always, Not yet and Need from you unless they say
 * `nothing`. A field holding a list never says nothing. A compact Recap shows the fields it wrote.
 * Null for the legacy Recap, which keeps one unlabeled Did block.
 */
export function recapSections(recap: Recap): { label: string; text: string }[] | null {
  if (recap.sections) return recap.sections;
  if (recap.notYet === undefined || recap.need === undefined) return null;
  return [
    { label: "Did", text: recap.did },
    ...[
      { label: "Not yet", text: recap.notYet },
      { label: "Need from you", text: recap.need },
    ].filter(({ text }) => !NOTHING.test(text)),
  ];
}

/** Commit chip of a Recap. `label` keeps the full value for assistive tech. */
export function commitChip(value: string): { text: string; done: boolean; label: string } {
  const said = value.replace(/`/g, "").trim();
  const yes = /^yes\b/i.test(said);
  // `yes` only confirms; the chip shows the detail after it. Leftover punctuation is no detail.
  const detail = said.replace(/^yes\b[\s,;:.\u2013\u2014-]*/i, "");
  // A compact Recap state for a turn without a commit; the chip drops any note after it.
  const uncommitted = UNCOMMITTED.exec(said)?.[1];
  return {
    text: uncommitted
      ? uncommitted[0].toUpperCase() + uncommitted.slice(1)
      : /^(?:no|none)\.?$/i.test(said)
        ? "No commit"
        : !yes
          ? said
          : /[\p{L}\p{N}]/u.test(detail)
            ? detail
            : "Committed",
    done: yes || /^(?:committed|pushed)\b/i.test(said),
    label: `Commit/push: ${said}`,
  };
}

/** Splits `**bold**` runs out of a one-line reason; odd indexes are bold. */
export function boldRuns(text: string): string[] {
  return text.split(/\*\*(.+?)\*\*/);
}
