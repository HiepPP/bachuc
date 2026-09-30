import { defineRpc } from "@getpaseo/plugin";
import { z } from "zod";

export const recapEntrySchema = z.object({
  agentId: z.string(),
  turnId: z.string().nullable(),
  title: z.string(),
  cwd: z.string(),
  workspaceId: z.string().nullable(),
  project: z.string(),
  projectKey: z.string(),
  endedAt: z.string(),
  day: z.string(),
  branch: z.string().nullable(),
  did: z.string().nullable(),
  commitPush: z.string().nullable(),
  raw: z.string(),
});
export type RecapEntry = z.infer<typeof recapEntrySchema>;

const recapProjectSchema = z.object({
  key: z.string(),
  name: z.string(),
  projectId: z.string().optional(),
  entries: z.array(recapEntrySchema),
});
export type RecapProject = z.infer<typeof recapProjectSchema>;

const recapDaySchema = z.object({ day: z.string(), projects: z.array(recapProjectSchema) });
export type RecapDay = z.infer<typeof recapDaySchema>;

export const recapsRpc = defineRpc({
  name: "board.recaps",
  input: z.object({ days: z.number().int().min(1).max(30).default(7) }),
  output: z.object({ days: z.array(recapDaySchema) }),
});

export function recapDayMarkdown(day: RecapDay): string {
  const lines = [`## ${day.day}`];
  for (const project of day.projects) {
    lines.push("", `### ${project.name}`);
    for (const entry of project.entries) {
      const details = [
        entry.did,
        entry.branch && `Branch: ${entry.branch}`,
        entry.commitPush && `Commit/push: ${entry.commitPush}`,
      ].filter(Boolean);
      lines.push(`- **${entry.title}**${details.length ? ` — ${details.join(" · ")}` : ""}`);
    }
  }
  return `${lines.join("\n")}\n`;
}
