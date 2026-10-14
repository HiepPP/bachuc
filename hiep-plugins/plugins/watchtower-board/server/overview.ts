import { execFile } from "node:child_process";
import { lstat, readdir, realpath } from "node:fs/promises";
import path from "node:path";
import { promisify } from "node:util";
import {
  type Decision,
  type DecisionDetail,
  HISTORY_LIMIT,
  type History,
  type ManualCheck,
  type Overview,
  type OverviewQuestion,
  type OverviewRun,
  type OverviewRunEntry,
  type OverviewTask,
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

// Log times are clock times such as `22:10`, or dated times such as `2026-10-09 22:10`. Walking the
// rows in order, a clock time over 12 hours earlier than the one before it starts the next day. A
// smaller step back is an overlap, such as a row that starts a minute before `Started:` or before
// the previous row ended, and stays on the same day. A dated time counts from `Started:` exactly,
// so it can show a gap of several days, and the clock times after it continue from its day.
export function parseOverviewRun(markdown: string, now = new Date()): OverviewRun {
  const started = field(markdown, "Started");
  const finished = field(markdown, "Finished");
  const rows = tableRows(section(markdown, "Log") ?? "", ["start", "end", "task", "result"]);
  // A `Started:` date with no time counts from midnight, so a stale run still reads as stopped.
  const startedAt =
    dateTime(started) ??
    dateTime(started?.match(/^\d{4}-\d{2}-\d{2}$/) ? `${started} 00:00` : null);
  let origin = startedAt ? startedAt.getHours() * 60 + startedAt.getMinutes() : null;
  let previous = origin;
  let dayOffset = 0;
  const offset = (value: string): number | null => {
    const dated = /^\d{4}-\d{2}-\d{2}[ T]\d{2}:\d{2}$/.test(value) ? dateTime(value) : null;
    if (dated && startedAt && origin !== null) {
      const fromStart = Math.round((dated.getTime() - startedAt.getTime()) / 60_000);
      previous = dated.getHours() * 60 + dated.getMinutes();
      dayOffset = fromStart + origin - previous;
      return fromStart;
    }
    // Without `Started:` a dated time has no origin to count from, so only its clock part counts.
    const minutes = clockMinutes(dated ? value.slice(-5) : value);
    if (minutes === null) return null;
    origin ??= minutes;
    previous ??= minutes;
    if (minutes < previous - DAY_MINUTES / 2) dayOffset += DAY_MINUTES;
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

const execFileAsync = promisify(execFile);
const GIT_TIMEOUT_MS = 5000;
const GIT_LOG_LIMIT = 500;
// A folder outside a repository, or a repository with no commits yet, has no run to show.
const NO_GIT_LOG = /not a git repository|does not have any commits yet/i;
// A TASK ID counts only inside parentheses, such as `(TASK-095)`. A subject like
// `add TASK-100..103 to the plan` names a range, not the TASK that the commit finished.
const PARENTHESES = /\(([^()]*)\)/g;

export type GitLog = {
  // `sha<TAB>iso time<TAB>subject`, newest first.
  lines: string[];
  // `sha<TAB>iso time<TAB>subject<TAB>body` of the commits that touch watchtower/NEXT.md.
  planLines: string[];
};
export type ReadGitLog = (directory: string) => Promise<GitLog>;

function gitMessage(error: unknown): string {
  const failure = error as NodeJS.ErrnoException & { killed?: boolean; stderr?: string };
  if (failure.killed) return `git log timed out after ${GIT_TIMEOUT_MS / 1000} s.`;
  if (failure.code === "ENOENT") return "git is not installed.";
  const reason = failure.stderr?.trim().split(/\r?\n/)[0] || failure.message;
  return reason.replace(/^fatal:\s*/, "");
}

export const readGitLog: ReadGitLog = async (directory) => {
  const git = async (separator: string[], format: string, args: string[] = []) => {
    const { stdout } = await execFileAsync(
      "git",
      ["log", "-n", String(GIT_LOG_LIMIT), ...separator, `--format=${format}`, ...args],
      { cwd: directory, timeout: GIT_TIMEOUT_MS, maxBuffer: 4 * 1024 * 1024 },
    );
    return stdout;
  };
  try {
    const [log, plan] = await Promise.all([
      git([], "%H%x09%cI%x09%s"),
      // NUL-separated, because a commit body spans several lines.
      git(["-z"], "%H%x09%cI%x09%s%x09%b", ["--", "watchtower/NEXT.md"]),
    ]);
    return {
      lines: log.split(/\r?\n/).filter(Boolean),
      planLines: plan
        .split("\0")
        .map((record) => record.replace(/\s+/g, " ").trim())
        .filter(Boolean),
    };
  } catch (error) {
    if (NO_GIT_LOG.test((error as { stderr?: string }).stderr ?? "")) {
      return { lines: [], planLines: [] };
    }
    throw error;
  }
};

type Commit = { sha: string; time: Date; subject: string };

function parseCommit(line: string): Commit | null {
  const [sha, iso, ...subject] = line.split("\t");
  const time = new Date(iso ?? "");
  return sha && !Number.isNaN(time.getTime()) ? { sha, time, subject: subject.join("\t") } : null;
}

function twoDigits(value: number): string {
  return String(value).padStart(2, "0");
}

// The run of a plan that `implement` ran with no RUN.md: each TASK ends at its newest commit that
// names it, and starts when the commit before its first commit landed. `lines` is `git log` output,
// newest first. `planLines` is the same with the body, for commits that touch watchtower/NEXT.md; the
// oldest one that names the plan slug sets `Started:`.
export function runFromGitLog(
  lines: string[],
  tasks: Pick<OverviewTask, "id" | "status">[],
  slug: string | null,
  planLines: string[] = [],
  now = new Date(),
): OverviewRun | null {
  const known = new Set(tasks.map((task) => task.id));
  const commits = lines.map(parseCommit).filter((commit): commit is Commit => commit !== null);
  // Index in `commits` of each TASK's newest and oldest counted commit.
  const newest = new Map<string, number>();
  const oldest = new Map<string, number>();
  commits.forEach((commit, index) => {
    const named = [...commit.subject.matchAll(PARENTHESES)].flatMap((group) => taskIds(group[1]));
    for (const id of new Set(named)) {
      if (!known.has(id)) continue;
      if (!newest.has(id)) newest.set(id, index);
      oldest.set(id, index);
    }
  });
  if (newest.size === 0) return null;

  // The commit before a counted commit is the next one in the log; the oldest in the log has none.
  const before = (index: number) => commits[Math.min(index + 1, commits.length - 1)].time;
  const planTimes = slug
    ? planLines
        .filter((record) => record.includes(slug))
        .map((record) => parseCommit(record)?.time)
        .filter((time): time is Date => time !== undefined)
    : [];
  const startedAt = planTimes.length
    ? new Date(Math.min(...planTimes.map((time) => time.getTime())))
    : before(Math.max(...oldest.values()));
  const minute = (time: Date) =>
    Math.floor(time.getTime() / 60_000) - Math.floor(startedAt.getTime() / 60_000);
  const day = (time: Date) =>
    `${time.getFullYear()}-${twoDigits(time.getMonth() + 1)}-${twoDigits(time.getDate())}`;
  const clock = (time: Date) => `${twoDigits(time.getHours())}:${twoDigits(time.getMinutes())}`;
  const full = (time: Date) => `${day(time)} ${clock(time)}`;
  // A time on another day than `Started:` carries its date, like a dated RUN.md time.
  const label = (time: Date) => (day(time) === day(startedAt) ? clock(time) : full(time));

  const log: OverviewRunEntry[] = [...newest]
    .map(([task, endIndex]) => {
      const end = commits[endIndex];
      const startTime = before(oldest.get(task) ?? endIndex);
      // Commit times from a rebase can run backwards; a row never ends before it starts.
      const start = startTime > end.time ? end.time : startTime;
      return {
        task,
        result: "DONE",
        detail: `commit ${end.sha.slice(0, 7)}`,
        start: label(start),
        end: label(end.time),
        startMinute: minute(start),
        endMinute: minute(end.time),
        endTime: end.time.getTime(),
      };
    })
    .sort((a, b) => a.endTime - b.endTime || a.startMinute - b.startMinute)
    .map(({ endTime: _endTime, ...row }) => row);

  const lastTime = new Date(
    Math.max(...[...newest.values()].map((i) => commits[i].time.getTime())),
  );
  const finished = tasks.length > 0 && tasks.every((task) => task.status.toUpperCase() === "DONE");
  const lastActivity = minute(lastTime);
  const elapsed = minute(finished ? lastTime : now);
  const stopped = !finished && elapsed - lastActivity > STALE_RUN_MINUTES;
  return {
    runner: "implement (git)",
    started: full(startedAt),
    finished: finished ? full(lastTime) : null,
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

export async function readOverview(
  directory: string,
  now = new Date(),
  git: ReadGitLog = readGitLog,
): Promise<Overview> {
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

  const warned = overview.warnings.length;
  const run = await optional("RUN.md");
  if (run !== null) overview.run = parseOverviewRun(run, now);
  // Only an absent RUN.md falls back to the git log; an unreadable one has warned already.
  else if (overview.warnings.length === warned && overview.tasks.length > 0) {
    try {
      const { lines, planLines } = await git(directory);
      overview.run = runFromGitLog(lines, overview.tasks, overview.plan.slug, planLines, now);
    } catch (error) {
      overview.warnings.push(`git log: ${gitMessage(error)}`);
    }
  }
  const questions = await optional("QUESTIONS.md");
  if (questions !== null) overview.questions = parseOverviewQuestions(questions);
  const decisions = await optional("DECISIONS.md");
  if (decisions !== null) overview.decisions = parseDecisions(decisions);
  overview.history = await readHistory(watchtower, optional, overview.warnings);
  return overview;
}

// The ADR file is `<id>.md` or `<id>-*.md` in decisions/. The ID only picks a directory entry; it
// never becomes part of a path, and the read goes through the same checks as every other file.
export async function readDecision(directory: string, id: string): Promise<DecisionDetail> {
  const place = await locate(directory);
  if (!place) throw new Error("This workspace has no watchtower directory.");
  const folder = path.join(place.watchtower, "decisions");
  let name: string | undefined;
  try {
    name = (await readdir(folder)).sort().find((entry) => {
      return entry.endsWith(".md") && (entry === `${id}.md` || entry.startsWith(`${id}-`));
    });
  } catch (error) {
    throw new Error(`Cannot read watchtower/decisions. ${message(error)}`);
  }
  if (!name) throw new Error(`${id} has no file in watchtower/decisions.`);
  const target = path.join(folder, name);
  let markdown: string;
  try {
    markdown = await readFile(place.watchtower, target);
  } catch (error) {
    throw new Error(`${name}: ${message(error)}`);
  }
  const heading = markdown.match(/^#\s+(.+)$/m)?.[1] ?? "";
  return {
    id,
    title: plain(heading).replace(/^ADR-\d+\b[\s:-]*/, ""),
    status: plain(markdown.match(/^Status:(.*)$/m)?.[1] ?? ""),
    file: path.relative(place.root, target).split(path.sep).join("/"),
    markdown,
  };
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
