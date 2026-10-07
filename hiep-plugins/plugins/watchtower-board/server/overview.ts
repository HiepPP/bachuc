import { realpath } from "node:fs/promises";
import path from "node:path";
import type { ManualCheck, Overview } from "../shared/overview";
import { inside, plain, readBoard, readFile, section, tableRows } from "./board";

const MANUAL_CHECK = /^Manual check pending:\s*/i;

function taskIds(text: string): string[] {
  return [...(text.match(/\bTASK-\d+\b/g) ?? [])];
}

// Top-level bullet lines of a section, as plain text.
function bullets(markdown: string | null): string[] {
  return (markdown ?? "")
    .split(/\r?\n/)
    .map((line) => line.match(/^[-*]\s+(.*)$/)?.[1])
    .filter((line): line is string => Boolean(line))
    .map(plain);
}

function field(markdown: string, name: string): string | null {
  const value = plain(markdown.match(new RegExp(`^[ \\t]*-[ \\t]*${name}:(.*)$`, "im"))?.[1] ?? "");
  return value && value !== "-" ? value : null;
}

// readBoard already reports a missing or misplaced NEXT.md, so a failed read here returns null.
async function readNext(directory: string): Promise<string | null> {
  try {
    const root = await realpath(directory);
    const watchtower = await realpath(path.join(root, "watchtower"));
    if (!inside(root, watchtower)) return null;
    return await readFile(watchtower, path.join(watchtower, "NEXT.md"));
  } catch {
    return null;
  }
}

export function parseManualChecks(handoff: string | null): ManualCheck[] {
  return bullets(handoff)
    .filter((line) => MANUAL_CHECK.test(line))
    .map((line) => {
      const text = line.replace(MANUAL_CHECK, "").trim();
      return { text, tasks: taskIds(text) };
    });
}

export async function readOverview(directory: string): Promise<Overview> {
  const board = await readBoard(directory, { runState: false });
  const overview: Overview = {
    plan: { title: board.title, slug: null, status: null, updated: null },
    tasks: board.tasks.map((task) => ({ ...task, depIds: taskIds(task.deps), group: null })),
    planVerify: [],
    manualChecks: [],
    message: board.message,
    warnings: board.warnings,
  };
  const markdown = await readNext(directory);
  if (markdown === null) return overview;
  const meta = section(markdown, "Current Active Plan") ?? markdown;
  overview.plan.slug = field(meta, "Slug");
  overview.plan.status = field(meta, "Status");
  overview.plan.updated = field(meta, "Updated");
  const groups = new Map<string, string | null>();
  for (const row of tableRows(section(markdown, "Tracker") ?? "", ["task", "status"])) {
    const id = taskIds(row.task)[0];
    if (id && "group" in row) groups.set(id, row.group || null);
  }
  for (const task of overview.tasks) task.group = groups.get(task.id) ?? null;
  overview.planVerify = bullets(section(markdown, "Plan Verify"));
  overview.manualChecks = parseManualChecks(section(markdown, "Handoff"));
  return overview;
}
