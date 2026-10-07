import { lstat, readdir, realpath } from "node:fs/promises";
import path from "node:path";
import {
  type Decision,
  HISTORY_LIMIT,
  type History,
  type ManualCheck,
  type Overview,
  type OverviewQuestion,
  type OverviewRun,
} from "../shared/overview";
import { inside, message, plain, readBoard, readFile, section, tableRows } from "./board";

const MANUAL_CHECK = /^Manual check pending:\s*/i;
// readBoard caps NEXT.md and the specs at 1 MiB; the overview's own reads get the same cap.
const MAX_OVERVIEW_BYTES = 1024 * 1024;
const DAY_MINUTES = 24 * 60;
// A run with no `Finished:` line and no log activity for this long has stopped: the loop died or
// was stopped by hand. Its "now" stays at the last activity instead of growing with the clock.
export const STALE_RUN_MINUTES = 120;

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

// A local date and time such as `2026-10-06 22:52`, or null.
function dateTime(value: string | null): Date | null {
  const match = value?.match(/(\d{4})-(\d{2})-(\d{2})[ T](\d{2}):(\d{2})/);
  if (!match) return null;
  const [, year, month, day, hour, minute] = match.map(Number);
  return new Date(year, month - 1, day, hour, minute);
}

function clockMinutes(value: string): number | null {
  const match = value.match(/^(\d{1,2}):(\d{2})$/);
  if (!match) return null;
  const minutes = Number(match[1]) * 60 + Number(match[2]);
  return minutes < DAY_MINUTES ? minutes : null;
}

async function locate(directory: string): Promise<{ root: string; watchtower: string } | null> {
  try {
    const root = await realpath(directory);
    const watchtower = await realpath(path.join(root, "watchtower"));
    return inside(root, watchtower) ? { root, watchtower } : null;
  } catch {
    return null;
  }
}

export function parseManualChecks(handoff: string | null): ManualCheck[] {
  return bullets(handoff)
    .filter((line) => MANUAL_CHECK.test(line))
    .map((line) => {
      const rest = line.replace(MANUAL_CHECK, "").trim();
      const text = rest.charAt(0).toUpperCase() + rest.slice(1);
      return { text, tasks: taskIds(text) };
    });
}

// Log times are clock times with no date. Walking the rows in order, a time earlier than the one
// before it starts the next day.
export function parseOverviewRun(markdown: string, now = new Date()): OverviewRun {
  const started = field(markdown, "Started");
  const finished = field(markdown, "Finished");
  const rows = tableRows(section(markdown, "Log") ?? "", ["start", "end", "task", "result"]);
  const startedAt = dateTime(started);
  let origin = startedAt ? startedAt.getHours() * 60 + startedAt.getMinutes() : null;
  let previous = origin;
  let dayOffset = 0;
  const offset = (value: string): number | null => {
    const minutes = clockMinutes(value);
    if (minutes === null) return null;
    origin ??= minutes;
    previous ??= minutes;
    if (minutes < previous) dayOffset += DAY_MINUTES;
    previous = minutes;
    return dayOffset + minutes - origin;
  };
  const log = rows.map((row) => {
    const startMinute = offset(row.start);
    const endMinute = offset(row.end);
    return {
      task: row.task,
      result: row.result,
      detail: row["pr or reason"] ?? "",
      start: row.start,
      end: row.end,
      startMinute,
      endMinute,
    };
  });
  const end = dateTime(finished) ?? now;
  const elapsed = startedAt ? Math.floor((end.getTime() - startedAt.getTime()) / 60_000) : null;
  const lastActivity = [...log].reverse().find((row) => row.endMinute !== null)?.endMinute ?? 0;
  const stopped = !finished && elapsed !== null && elapsed - lastActivity > STALE_RUN_MINUTES;
  return {
    runner: field(markdown, "Runner"),
    started,
    finished,
    log,
    nowMinute: stopped ? lastActivity : elapsed,
    stopped,
  };
}

export function parseOverviewQuestions(markdown: string): OverviewQuestion[] {
  return tableRows(markdown, ["id", "blocks", "question", "status"])
    .filter((row) => ["OPEN", "DEFAULTED"].includes(row.status.toUpperCase()))
    .map((row) => ({
      id: row.id,
      tasks: taskIds(row.task ?? ""),
      blocks: taskIds(row.blocks),
      question: row.question,
      default: row.default ?? "",
      status: row.status.toUpperCase(),
    }));
}

export function parseDecisions(markdown: string): Decision[] {
  return tableRows(section(markdown, "Index") ?? markdown, ["id", "status"]).map((row) => ({
    id: row.id,
    date: row.date ?? "",
    title: row.title ?? "",
    status: row.status,
  }));
}

export async function readOverview(directory: string, now = new Date()): Promise<Overview> {
  const board = await readBoard(directory, { runState: false });
  const overview: Overview = {
    plan: { title: board.title, slug: null, status: null, updated: null },
    tasks: board.tasks.map(({ brief: _brief, ...task }) => ({
      ...task,
      depIds: taskIds(task.deps),
      group: null,
    })),
    planVerify: [],
    manualChecks: [],
    run: null,
    questions: [],
    decisions: [],
    history: [],
    message: board.message,
    warnings: [...board.warnings],
  };
  const place = await locate(directory);
  if (!place) return overview;
  const { watchtower } = place;
  let bytes = 0;
  // A missing file gives null quietly; an unreadable one adds a warning, like readBoard does.
  const optional = async (name: string, warn = true): Promise<string | null> => {
    try {
      const text = await readFile(watchtower, path.join(watchtower, name));
      bytes += Buffer.byteLength(text);
      if (bytes > MAX_OVERVIEW_BYTES) throw new Error("Overview files exceed the 1 MiB limit.");
      return text;
    } catch (error) {
      if (warn && (error as NodeJS.ErrnoException).code !== "ENOENT")
        overview.warnings.push(`${name}: ${message(error)}`);
      return null;
    }
  };

  // readBoard already reports a missing or misplaced NEXT.md, so this read stays quiet.
  const markdown = await optional("NEXT.md", false);
  if (markdown !== null) {
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
  }

  const run = await optional("RUN.md");
  if (run !== null) overview.run = parseOverviewRun(run, now);
  const questions = await optional("QUESTIONS.md");
  if (questions !== null) overview.questions = parseOverviewQuestions(questions);
  const decisions = await optional("DECISIONS.md");
  if (decisions !== null) overview.decisions = parseDecisions(decisions);
  overview.history = await readHistory(watchtower, optional, overview.warnings);
  return overview;
}

// The newest archive folders by name. Symlinks and files are skipped.
async function readHistory(
  watchtower: string,
  optional: (name: string, warn?: boolean) => Promise<string | null>,
  warnings: string[],
): Promise<History[]> {
  let entries;
  try {
    const archive = await realpath(path.join(watchtower, "archive"));
    if (!inside(watchtower, archive)) throw new Error("Archive must stay inside Watchtower.");
    entries = await readdir(archive, { withFileTypes: true });
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== "ENOENT")
      warnings.push(`archive: ${message(error)}`);
    return [];
  }
  const slugs = entries
    .filter((entry) => entry.isDirectory() && !entry.isSymbolicLink())
    .map((entry) => entry.name)
    .sort()
    .reverse()
    .slice(0, HISTORY_LIMIT);
  const history: History[] = [];
  for (const slug of slugs) {
    const next = await optional(path.join("archive", slug, "NEXT.md"));
    const title = next?.match(/^\s*-?\s*Title:\s*(.+)$/m)?.[1];
    const prefix = slug.match(/^(\d{4})(\d{2})(\d{2})-/);
    let hasLearn = false;
    try {
      hasLearn = (await lstat(path.join(watchtower, "archive", slug, "LEARN.md"))).isFile();
    } catch {
      hasLearn = false;
    }
    history.push({
      slug,
      date: prefix ? `${prefix[1]}-${prefix[2]}-${prefix[3]}` : null,
      title: title ? plain(title) : slug,
      hasLearn,
    });
  }
  return history;
}
