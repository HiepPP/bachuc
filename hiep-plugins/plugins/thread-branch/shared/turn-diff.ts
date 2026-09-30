import { defineRpc } from "@getpaseo/plugin";
import { z } from "zod";

export const TURN_DIFF_KIND = "thread-branch-turn-diff";
export const TURN_DIFF_VERSION = 1;
export const TURN_DIFF_PANEL = "turn-diff";
/** Files kept in a row; the rest are only counted. */
export const TURN_DIFF_MAX_FILES = 20;
export const TURN_DIFF_MAX_COMMITS = 10;

export const turnDiffSchema = z.object({
  fileCount: z.number().int().nonnegative(),
  added: z.number().int().nonnegative(),
  deleted: z.number().int().nonnegative(),
  /** Null counts mean a binary file. */
  files: z.array(
    z.object({
      path: z.string(),
      added: z.number().int().nonnegative().nullable(),
      deleted: z.number().int().nonnegative().nullable(),
      /** An untracked file too large to snapshot: listed, but with no diff. */
      large: z.boolean().optional(),
    }),
  ),
  commits: z.array(z.object({ sha: z.string(), subject: z.string() })),
  /** Another agent ran a turn in the same directory at the same time. */
  shared: z.boolean(),
  /** Work-tree snapshots of the turn. Rows without them cannot open a file diff. */
  source: z
    .object({
      workspaceId: z.string(),
      root: z.string(),
      from: z.string(),
      to: z.string(),
    })
    .optional(),
});

export type TurnDiff = z.output<typeof turnDiffSchema>;

export const turnHistoryEntrySchema = z.object({
  /** Unique per turn, also used in the git ref names that keep its snapshots. */
  key: z.string(),
  agentId: z.string(),
  endedAt: z.string(),
  diff: turnDiffSchema,
});
export type TurnHistoryEntry = z.output<typeof turnHistoryEntrySchema>;

export const turnHistoryRpc = defineRpc({
  name: "thread-branch.turn-history",
  input: z.object({ agentId: z.string().min(1).max(256) }),
  output: z.object({ turns: z.array(turnHistoryEntrySchema) }),
});

export function turnDiffHeader(data: TurnDiff) {
  const files = `${data.fileCount} ${data.fileCount === 1 ? "file" : "files"} changed`;
  const commits = data.commits.length
    ? `${data.commits.length} ${data.commits.length === 1 ? "commit" : "commits"}`
    : null;
  if (data.fileCount === 0) return commits ?? files;
  return [`${files} +${data.added} -${data.deleted}`, commits].filter(Boolean).join(" · ");
}

const objectId = z.string().regex(/^[0-9a-f]{40}([0-9a-f]{24})?$/);

export const fileDiffRequestSchema = z.object({
  root: z.string().min(1),
  from: objectId,
  to: objectId,
  path: z.string().min(1),
});
export type FileDiffRequest = z.output<typeof fileDiffRequestSchema>;

export const fileDiffRpc = defineRpc({
  name: "thread-branch.file-diff",
  input: fileDiffRequestSchema,
  output: z.object({ diff: z.string(), truncated: z.boolean() }),
});

const IMAGE_TYPES: Record<string, string> = {
  png: "image/png",
  jpg: "image/jpeg",
  jpeg: "image/jpeg",
  gif: "image/gif",
  webp: "image/webp",
  bmp: "image/bmp",
  ico: "image/x-icon",
  avif: "image/avif",
  svg: "image/svg+xml",
};

/** The image MIME type of a path by its extension, or null when it is not a previewable image. */
export function imageType(path: string) {
  return IMAGE_TYPES[path.split(".").pop()?.toLowerCase() ?? ""] ?? null;
}

export const fileImageRpc = defineRpc({
  name: "thread-branch.file-image",
  input: fileDiffRequestSchema,
  output: z.object({
    /** `data:` URIs of the file before and after the turn; null when that side has no file. */
    before: z.string().nullable(),
    after: z.string().nullable(),
    tooLarge: z.boolean(),
  }),
});

export type DiffRow =
  | { type: "hunk"; text: string }
  | { type: "note"; text: string }
  | {
      type: "add" | "remove" | "context";
      text: string;
      oldLine: number | null;
      newLine: number | null;
    };

/** Parses a one-file `git diff` into numbered rows, like the gutters of Paseo's own diff view. */
export function parseUnifiedDiff(diff: string) {
  const rows: DiffRow[] = [];
  let status: "modified" | "added" | "deleted" = "modified";
  let binary = false;
  let added = 0;
  let removed = 0;
  let oldLine = 0;
  let newLine = 0;
  let inHunk = false;
  for (const line of diff.replace(/\n$/, "").split("\n")) {
    const hunk = line.match(/^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@/);
    if (hunk) {
      inHunk = true;
      oldLine = Number(hunk[1]);
      newLine = Number(hunk[2]);
      rows.push({ type: "hunk", text: line });
    } else if (!inHunk) {
      if (line.startsWith("new file mode")) status = "added";
      else if (line.startsWith("deleted file mode")) status = "deleted";
      else if (line.startsWith("Binary files")) binary = true;
    } else if (line.startsWith("+")) {
      rows.push({ type: "add", text: line.slice(1), oldLine: null, newLine: newLine++ });
      added++;
    } else if (line.startsWith("-")) {
      rows.push({ type: "remove", text: line.slice(1), oldLine: oldLine++, newLine: null });
      removed++;
    } else if (line.startsWith("\\")) {
      rows.push({ type: "note", text: line.slice(2) });
    } else {
      rows.push({ type: "context", text: line.slice(1), oldLine: oldLine++, newLine: newLine++ });
    }
  }
  return { status, binary, added, removed, rows };
}
