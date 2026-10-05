export type RowKind = "add" | "remove" | "context";

export interface Row {
  kind: RowKind;
  text: string;
}

/** Starts are 1-based file lines; null until the hunk is located in the file. */
export interface Hunk {
  oldStart: number | null;
  newStart: number | null;
  rows: Row[];
}

export interface EditSource {
  oldString?: string;
  newString?: string;
  unifiedDiff?: string;
  /** A whole written file; every line is new. */
  content?: string;
}

export interface NumberedRow extends Row {
  line: number | null;
}

/** Above this many cells the line diff skips LCS and shows old lines removed, new lines added. */
const MAX_LCS_CELLS = 250_000;
const HUNK_HEADER = /^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@/;

function splitLines(text: string): string[] {
  if (text === "") return [];
  const lines = text.split("\n");
  if (lines.at(-1) === "") lines.pop();
  return lines;
}

export function lineDiff(oldText: string, newText: string): Row[] {
  const a = splitLines(oldText);
  const b = splitLines(newText);
  if (a.length * b.length > MAX_LCS_CELLS) {
    return [
      ...a.map((text) => ({ kind: "remove" as const, text })),
      ...b.map((text) => ({ kind: "add" as const, text })),
    ];
  }
  const width = b.length + 1;
  const lcs = new Uint32Array((a.length + 1) * width);
  for (let i = a.length - 1; i >= 0; i--) {
    for (let j = b.length - 1; j >= 0; j--) {
      lcs[i * width + j] =
        a[i] === b[j]
          ? lcs[(i + 1) * width + j + 1] + 1
          : Math.max(lcs[(i + 1) * width + j], lcs[i * width + j + 1]);
    }
  }
  const rows: Row[] = [];
  let i = 0;
  let j = 0;
  while (i < a.length && j < b.length) {
    if (a[i] === b[j]) {
      rows.push({ kind: "context", text: a[i] });
      i++;
      j++;
    } else if (lcs[(i + 1) * width + j] >= lcs[i * width + j + 1]) {
      rows.push({ kind: "remove", text: a[i++] });
    } else {
      rows.push({ kind: "add", text: b[j++] });
    }
  }
  while (i < a.length) rows.push({ kind: "remove", text: a[i++] });
  while (j < b.length) rows.push({ kind: "add", text: b[j++] });
  return rows;
}

function isFileHeader(lines: string[], index: number): boolean {
  const line = lines[index];
  return (
    line.startsWith("diff --git ") ||
    line.startsWith("index ") ||
    (line.startsWith("--- ") && (lines[index + 1]?.startsWith("+++ ") ?? false)) ||
    (line.startsWith("+++ ") && (lines[index - 1]?.startsWith("--- ") ?? false))
  );
}

export function parseUnifiedDiff(diff: string): Hunk[] {
  const lines = diff.split("\n");
  const hunks: Hunk[] = [];
  let hunk: Hunk | null = null;
  // Remaining counts from the hunk header; null for Codex's bare `@@`, which has none.
  let oldLeft: number | null = null;
  let newLeft: number | null = null;
  for (let index = 0; index < lines.length; index++) {
    const line = lines[index];
    if (line.startsWith("@@")) {
      const match = HUNK_HEADER.exec(line);
      hunk = {
        oldStart: match ? Number(match[1]) : null,
        newStart: match ? Number(match[3]) : null,
        rows: [],
      };
      oldLeft = match ? Number(match[2] ?? 1) : null;
      newLeft = match ? Number(match[4] ?? 1) : null;
      hunks.push(hunk);
      continue;
    }
    if (hunk && oldLeft === 0 && newLeft === 0) {
      hunk = null;
      oldLeft = null;
      newLeft = null;
    }
    const counted = oldLeft !== null;
    if (!counted && isFileHeader(lines, index)) continue;
    const marker = line[0];
    if (!hunk) {
      if (marker !== "+" && marker !== "-") continue;
      hunk = { oldStart: null, newStart: null, rows: [] };
      hunks.push(hunk);
    }
    const text = line.slice(1);
    if (marker === "+") {
      hunk.rows.push({ kind: "add", text });
      if (newLeft !== null) newLeft--;
    } else if (marker === "-") {
      hunk.rows.push({ kind: "remove", text });
      if (oldLeft !== null) oldLeft--;
    } else if (marker === " " || (line === "" && counted && index < lines.length - 1)) {
      // Some tools strip the space from empty context lines; only trust that inside a counted hunk.
      hunk.rows.push({ kind: "context", text });
      if (oldLeft !== null) oldLeft--;
      if (newLeft !== null) newLeft--;
    }
  }
  return hunks.filter((candidate) => candidate.rows.length > 0);
}

export function buildHunks(source: EditSource): Hunk[] {
  if (source.content !== undefined) {
    const rows = splitLines(source.content).map((text) => ({ kind: "add" as const, text }));
    return rows.length > 0 ? [{ oldStart: 0, newStart: 1, rows }] : [];
  }
  if (source.unifiedDiff) return parseUnifiedDiff(source.unifiedDiff);
  const rows = lineDiff(source.oldString ?? "", source.newString ?? "");
  return rows.length > 0 ? [{ oldStart: null, newStart: null, rows }] : [];
}

export function sideText(hunk: Hunk, side: "old" | "new"): string {
  const skip = side === "old" ? "add" : "remove";
  return hunk.rows
    .filter((row) => row.kind !== skip)
    .map((row) => row.text)
    .join("\n");
}

export function needsLocating(hunks: Hunk[]): boolean {
  return hunks.some((hunk) => hunk.oldStart === null || hunk.newStart === null);
}

export interface Located {
  line: number;
  side: "old" | "new";
}

/** Fills unknown starts from file positions; a hunk's old and new starts differ by the earlier hunks' net line change. */
export function applyLocations(hunks: Hunk[], located: (Located | null)[]): Hunk[] {
  let delta = 0;
  return hunks.map((hunk, index) => {
    const next = { ...hunk };
    const found = located[index];
    if ((next.oldStart === null || next.newStart === null) && found) {
      next.oldStart = found.side === "old" ? found.line : found.line - delta;
      next.newStart = found.side === "new" ? found.line : found.line + delta;
    }
    for (const row of hunk.rows) {
      if (row.kind === "add") delta++;
      if (row.kind === "remove") delta--;
    }
    return next;
  });
}

/** Claude Code style: removed rows show the old line number, other rows the new one. */
export function numberRows(hunk: Hunk): NumberedRow[] {
  let oldLine = hunk.oldStart;
  let newLine = hunk.newStart;
  return hunk.rows.map((row) => {
    if (row.kind === "remove") {
      const line = oldLine;
      if (oldLine !== null) oldLine++;
      return { ...row, line };
    }
    const line = newLine;
    if (newLine !== null) newLine++;
    if (row.kind === "context" && oldLine !== null) oldLine++;
    return { ...row, line };
  });
}

export function countChanges(hunks: Hunk[]): { added: number; removed: number } {
  let added = 0;
  let removed = 0;
  for (const hunk of hunks) {
    for (const row of hunk.rows) {
      if (row.kind === "add") added++;
      if (row.kind === "remove") removed++;
    }
  }
  return { added, removed };
}
