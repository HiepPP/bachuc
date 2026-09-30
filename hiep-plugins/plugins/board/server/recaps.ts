import { appendFile, mkdir, readFile, rename, writeFile } from "node:fs/promises";
import path from "node:path";
import type { PluginHookAgent } from "@getpaseo/plugin/server";
import { recapEntrySchema, type RecapDay, type RecapEntry } from "../shared/recaps";

export const RECAP_RAW_LIMIT = 2_000;
export const RECAP_MAX_DAYS = 90;
export const RECAP_MAX_ENTRIES = 2_000;
const DAY_MS = 86_400_000;

export type RecapFields = Pick<RecapEntry, "branch" | "did" | "commitPush" | "raw">;

const FIELD = /^(?:[-*]\s+)?(branch|did|commit\/push):\s*(.*)$/i;

/** Reads the last `## Recap` section of a reply; null when the reply has none. */
export function parseRecap(text: string): RecapFields | null {
  const lines = text.split(/\r?\n/);
  let start = -1;
  for (let index = lines.length - 1; index >= 0; index--) {
    if (/^##\s+Recap\s*$/i.test(lines[index].trim())) {
      start = index;
      break;
    }
  }
  if (start === -1) return null;
  let end = start + 1;
  while (end < lines.length && !/^##\s/.test(lines[end].trimStart())) end++;
  const fields: RecapFields = {
    branch: null,
    did: null,
    commitPush: null,
    raw: lines.slice(start, end).join("\n").trim().slice(0, RECAP_RAW_LIMIT),
  };
  for (const line of lines.slice(start + 1, end)) {
    const match = FIELD.exec(line.trim());
    const value = match?.[2].trim();
    if (!match || !value) continue;
    const key = match[1].toLowerCase();
    if (key === "branch") fields.branch ??= value;
    else if (key === "did") fields.did ??= value;
    else fields.commitPush ??= value;
  }
  return fields;
}

/** Names the project as the run store does: host placement, else the working directory name. */
export function recapEntry(
  agent: PluginHookAgent,
  turnId: string | null,
  fields: RecapFields,
  endedAt: Date,
  placement: { projectName: string; projectKey: string } | null,
  title?: string | null,
): RecapEntry {
  return {
    agentId: agent.id,
    turnId,
    title: agent.title || title || "Untitled run",
    cwd: agent.cwd,
    workspaceId: agent.workspaceId,
    project:
      placement?.projectName || agent.cwd.split(/[\\/]/).filter(Boolean).pop() || "Unknown project",
    projectKey: placement ? `project:${placement.projectKey}` : `cwd:${agent.cwd}`,
    endedAt: endedAt.toISOString(),
    day: localDay(endedAt),
    ...fields,
  };
}

export function localDay(date: Date): string {
  const pad = (value: number) => String(value).padStart(2, "0");
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
}

/** Days newest first, then projects by their newest recap, then recaps newest first. */
export function groupRecaps(entries: readonly RecapEntry[], days: number, now: Date): RecapDay[] {
  const first = new Date(now);
  first.setDate(first.getDate() - (days - 1));
  const cutoff = localDay(first);
  const sorted = entries
    .filter((entry) => entry.day >= cutoff)
    .sort((a, b) => b.day.localeCompare(a.day) || b.endedAt.localeCompare(a.endedAt));
  const result: RecapDay[] = [];
  for (const entry of sorted) {
    let day = result.at(-1);
    if (day?.day !== entry.day) {
      day = { day: entry.day, projects: [] };
      result.push(day);
    }
    let project = day.projects.find((item) => item.key === entry.projectKey);
    if (!project) {
      project = { key: entry.projectKey, name: entry.project, entries: [] };
      day.projects.push(project);
    }
    project.entries.push(entry);
  }
  return result;
}

export function createRecapStore(file: string, now = () => new Date()) {
  let entries: RecapEntry[] | undefined;
  let torn = false;
  let queue: Promise<unknown> = Promise.resolve();

  async function load() {
    if (entries) return entries;
    const text = await readFile(file, "utf8").catch((error: NodeJS.ErrnoException) => {
      if (error.code === "ENOENT") return "";
      throw error;
    });
    entries = [];
    torn = text.length > 0 && !text.endsWith("\n");
    for (const line of text.split("\n")) {
      if (!line.trim()) continue;
      try {
        const parsed = recapEntrySchema.safeParse(JSON.parse(line));
        if (parsed.success) entries.push(parsed.data);
      } catch {
        // A torn final line from a crash must not hide the rest of the log; the next add rewrites it.
      }
    }
    return entries;
  }

  function prune(list: RecapEntry[]) {
    const oldest = now().getTime() - RECAP_MAX_DAYS * DAY_MS;
    const kept = list.filter((entry) => Date.parse(entry.endedAt) >= oldest);
    return kept.slice(-RECAP_MAX_ENTRIES);
  }

  async function rewrite(list: RecapEntry[]) {
    const temporary = `${file}.tmp`;
    const body = list.map((entry) => `${JSON.stringify(entry)}\n`).join("");
    await writeFile(temporary, body, { mode: 0o600 });
    await rename(temporary, file);
  }

  function serial<T>(task: () => Promise<T>): Promise<T> {
    const next = queue.then(task, task);
    queue = next.catch(() => undefined);
    return next;
  }

  return {
    /** Returns false for a repeated delivery of the agent's latest turn. */
    add(entry: RecapEntry): Promise<boolean> {
      return serial(async () => {
        const list = await load();
        // Turn ids restart at 1 when Paseo reloads a session, so an id alone is not a repeat.
        const last = list.findLast((item) => item.agentId === entry.agentId);
        if (entry.turnId !== null && last?.turnId === entry.turnId && last.raw === entry.raw)
          return false;
        await mkdir(path.dirname(file), { recursive: true });
        const kept = prune([...list, entry]);
        if (!torn && kept.length === list.length + 1) {
          await appendFile(file, `${JSON.stringify(entry)}\n`, { mode: 0o600 });
        } else {
          await rewrite(kept);
          torn = false;
        }
        entries = kept;
        return true;
      });
    },
    list(days: number): Promise<RecapDay[]> {
      return serial(async () => groupRecaps(prune(await load()), days, now()));
    },
  };
}
