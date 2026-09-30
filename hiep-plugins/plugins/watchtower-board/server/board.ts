import { constants } from "node:fs";
import { open, realpath } from "node:fs/promises";
import path from "node:path";
import type { Board, Task } from "../shared/board";

const MAX_FILE_BYTES = 128 * 1024;
const MAX_BOARD_BYTES = 1024 * 1024;
const MAX_TASKS = 200;

function inside(root: string, target: string): boolean {
  const relative = path.relative(root, target);
  return relative !== ".." && !relative.startsWith(`..${path.sep}`) && !path.isAbsolute(relative);
}

async function readFile(root: string, target: string): Promise<string> {
  const actual = await realpath(target);
  if (!inside(root, actual))
    throw new Error("Linked file is outside the workspace Watchtower directory.");
  const file = await open(actual, constants.O_RDONLY | constants.O_NONBLOCK | constants.O_NOFOLLOW);
  try {
    const stat = await file.stat();
    if (!stat.isFile()) throw new Error("Expected a regular Markdown file.");
    if (stat.size > MAX_FILE_BYTES) throw new Error("Markdown file exceeds the 128 KiB limit.");
    const buffer = Buffer.alloc(MAX_FILE_BYTES + 1);
    let length = 0;
    while (length < buffer.length) {
      const { bytesRead } = await file.read(buffer, length, buffer.length - length, null);
      if (!bytesRead) break;
      length += bytesRead;
    }
    if (length > MAX_FILE_BYTES) throw new Error("Markdown file exceeds the 128 KiB limit.");
    return buffer.subarray(0, length).toString("utf8");
  } finally {
    await file.close();
  }
}

function message(error: unknown): string {
  const code = (error as NodeJS.ErrnoException)?.code;
  if (code === "ENOENT") return "File not found.";
  if (code === "EACCES" || code === "EPERM") return "File is not readable.";
  return error instanceof Error && !code ? error.message : "Could not read the Markdown file.";
}

// Ignore headings inside fences while preserving the original section text.
export function section(markdown: string, name: string): string | null {
  const lines = markdown.split(/\r?\n/);
  let fence: string | null = null;
  let start = -1;
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const marker = line.match(/^\s{0,3}(`{3,}|~{3,})/);
    if (marker) {
      if (!fence) fence = marker[1];
      else if (marker[1][0] === fence[0] && marker[1].length >= fence.length) fence = null;
      continue;
    }
    if (fence) continue;
    if (start >= 0 && /^#{1,2}\s/.test(line)) return lines.slice(start, i).join("\n").trim();
    if (line.trim().toLowerCase() === `## ${name}`.toLowerCase()) start = i + 1;
  }
  return start < 0 ? null : lines.slice(start).join("\n").trim();
}

function cells(line: string): string[] {
  return line
    .trim()
    .replace(/^\|/, "")
    .replace(/\|$/, "")
    .split(/(?<!\\)\|/)
    .map((value) => value.trim().replace(/\\\|/g, "|"));
}
function plain(value: string): string {
  return value
    .replace(/\[([^\]]+)\]\([^)]+\)/g, "$1")
    .replace(/[`*]/g, "")
    .trim();
}

export function parseManifest(markdown: string): Board {
  const tracker = section(markdown, "Tracker");
  const title = markdown.match(/^\s*-?\s*Title:\s*(.+)$/m)?.[1] ?? "Watchtower";
  if (tracker === null)
    return {
      title,
      tasks: [],
      message:
        "Unsupported plan: expected a Tracker table with TASK, Status, Spec, Deps, and Context columns.",
    };
  const lines = tracker.split(/\r?\n/);
  const headerIndex = lines.findIndex((line) => {
    const headers = cells(line).map((value) => value.toLowerCase());
    return ["task", "status", "spec", "deps", "context"].every((value) => headers.includes(value));
  });
  if (headerIndex < 0)
    return {
      title,
      tasks: [],
      message: "Unsupported legacy or malformed Tracker. Use the current Watchtower table format.",
    };
  const headers = cells(lines[headerIndex]).map((value) => value.toLowerCase());
  const tasks: Task[] = [];
  const seen = new Set<string>();
  for (const line of lines.slice(headerIndex + 1)) {
    if (!line.trim().startsWith("|")) break;
    if (/^[\s|:-]+$/.test(line)) continue;
    const row = cells(line);
    const get = (key: string) => row[headers.indexOf(key)] ?? "";
    const label = plain(get("task"));
    const id = label.match(/\bTASK-\d+\b/)?.[0];
    if (!id || seen.has(id) || row.length !== headers.length) {
      return {
        title,
        tasks: [],
        message: "Malformed Tracker: each row needs a unique TASK ID and all table cells.",
      };
    }
    seen.add(id);
    const spec = get("spec").match(/\]\(<?([^)>]+)>?\)/)?.[1] ?? "";
    const status = plain(get("status"));
    tasks.push({
      id,
      title: label,
      status,
      deps: plain(get("deps")),
      notes: plain(get("notes")),
      spec,
      brief: null,
      blocker: null,
      error: ["TODO", "IN PROGRESS", "BLOCKED", "DONE"].includes(status)
        ? null
        : "Unknown task status.",
    });
    if (tasks.length > MAX_TASKS)
      return { title, tasks: [], message: "Plan exceeds the 200-task limit." };
  }
  return { title, tasks, message: tasks.length ? null : "No tasks in this plan." };
}

function specPath(directory: string, watchtower: string, link: string): string {
  const decoded = decodeURIComponent(link);
  if (
    !decoded ||
    path.isAbsolute(decoded) ||
    /^[a-z][a-z0-9+.-]*:/i.test(decoded) ||
    /[?#]/.test(decoded)
  ) {
    throw new Error("Spec must link to a local Markdown file in Watchtower.");
  }
  const target = path.resolve(decoded.startsWith("watchtower/") ? directory : watchtower, decoded);
  if (!inside(watchtower, target) || path.extname(target) !== ".md")
    throw new Error("Spec link must stay inside Watchtower.");
  return target;
}

export async function readBoard(directory: string): Promise<Board> {
  const empty = (text: string): Board => ({ title: "Watchtower", tasks: [], message: text });
  let root: string;
  let watchtower: string;
  let markdown: string;
  try {
    root = await realpath(directory);
    watchtower = await realpath(path.join(root, "watchtower"));
    if (!inside(root, watchtower))
      return empty("Watchtower directory must stay inside the workspace.");
    markdown = await readFile(watchtower, path.join(watchtower, "NEXT.md"));
  } catch (error) {
    return empty(`Cannot load watchtower/NEXT.md. ${message(error)}`);
  }
  const board = parseManifest(markdown);
  let bytes = Buffer.byteLength(markdown);
  for (const task of board.tasks) {
    try {
      if (task.error) continue;
      const target = specPath(root, watchtower, task.spec);
      const spec = await readFile(watchtower, target);
      bytes += Buffer.byteLength(spec);
      if (bytes > MAX_BOARD_BYTES) throw new Error("Plan content exceeds the 1 MiB limit.");
      const heading = spec.match(/^#\s+(TASK-\d+)\b/m)?.[1];
      if (heading !== task.id) throw new Error("Spec TASK ID does not match the Tracker.");
      task.brief = section(spec, "Brief");
      if (!task.brief) throw new Error("Spec has no non-empty Brief section.");
      if (task.status === "BLOCKED") {
        const outcomePath = path.join(watchtower, "tasks", `${task.id}-outcome.md`);
        try {
          const outcome = await readFile(watchtower, outcomePath);
          const body = section(outcome, "Outcome") ?? outcome;
          task.blocker =
            body.match(/^Blocked:\s*([\s\S]*?)(?=^\w[\w ]*:\s|^##?\s|$(?![\s\S]))/m)?.[1].trim() ||
            null;
        } catch (error) {
          if ((error as NodeJS.ErrnoException).code !== "ENOENT")
            task.error = `Outcome: ${message(error)}`;
        }
        task.blocker ??=
          task.notes && task.notes !== "-" ? task.notes : "No blocker details recorded.";
      }
    } catch (error) {
      task.brief = null;
      task.error = message(error);
    }
  }
  return board;
}
