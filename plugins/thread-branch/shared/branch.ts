import { defineRpc } from "@getpaseo/plugin";
import { z } from "zod";

export const pullRequestSchema = z.object({
  number: z.number().int().positive(),
  url: z.url(),
  state: z.string(),
});

export const changesSchema = z.object({
  /** Tracked files listed by `git status --porcelain`; untracked files are not counted. */
  files: z.number().int().positive(),
  /** Lines vs HEAD, staged and unstaged; null when `git diff --numstat HEAD` fails (no commit yet). */
  added: z.number().int().nonnegative().nullable(),
  deleted: z.number().int().nonnegative().nullable(),
});
export type Changes = z.infer<typeof changesSchema>;

export const branchInfoSchema = z.object({
  /** False when `cwd` is not inside a git work tree or git is unavailable. */
  repo: z.boolean(),
  branch: z.string().nullable(),
  detached: z.boolean(),
  sha: z.string().nullable(),
  dirty: z.boolean(),
  /** Null when the work tree is clean. */
  changes: changesSchema.nullable(),
  upstream: z.string().nullable(),
  ahead: z.number().int().nonnegative().nullable(),
  behind: z.number().int().nonnegative().nullable(),
  pr: pullRequestSchema.nullable(),
  /** Browser URL of the `origin` remote, or null when there is none or it is not http/ssh. */
  remoteUrl: z.string().nullable(),
  /** Why `pr` is null when a lookup could not run: gh missing, not a GitHub remote, or lookup failed. */
  prLookup: z.enum(["ok", "unavailable", "failed", "skipped"]),
});
export type BranchInfo = z.infer<typeof branchInfoSchema>;

export const getBranchRpc = defineRpc({
  name: "thread-branch.get",
  input: z.object({
    cwd: z.string().min(1),
    force: z.boolean().optional(),
    /** Run `git fetch` before counting so `behind` reflects the remote. */
    fetch: z.boolean().optional(),
  }),
  output: branchInfoSchema,
});
