import { defineRpc } from "@getpaseo/plugin";
import { z } from "zod";

const locatedSchema = z.object({ line: z.number().int().positive(), side: z.enum(["old", "new"]) });

/** Finds where each hunk sits in the file, for edits that carry no line positions. */
export const locateRpc = defineRpc({
  name: "edit-diffs.locate",
  input: z.object({
    filePath: z.string(),
    cwd: z.string().nullable(),
    hunks: z.array(z.object({ oldText: z.string(), newText: z.string() })),
  }),
  output: z.object({ located: z.array(locatedSchema.nullable()) }),
});

export const editRowSchema = z.object({
  action: z.enum(["edit", "write"]).default("edit"),
  filePath: z.string(),
  oldString: z.string().optional(),
  newString: z.string().optional(),
  unifiedDiff: z.string().optional(),
  content: z.string().optional(),
  status: z.enum(["running", "completed", "failed", "canceled"]),
  error: z.string().optional(),
});

export type EditRowData = z.output<typeof editRowSchema>;
