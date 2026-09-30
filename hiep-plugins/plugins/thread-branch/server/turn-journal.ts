import { appendFile, mkdir, readFile, rename, writeFile } from "node:fs/promises";
import path from "node:path";
import { turnHistoryEntrySchema, type TurnHistoryEntry } from "../shared/turn-diff";

export const JOURNAL_MAX_DAYS = 30;
export const JOURNAL_MAX_ENTRIES = 2_000;
export const HISTORY_LIMIT = 50;
// Pruning rereads the whole file, so it runs on the first add and then every Nth add.
const PRUNE_EVERY = 50;
const DAY_MS = 24 * 60 * 60 * 1000;

/**
 * Turn diff cards on disk. The daemon keeps plugin timeline rows only in memory, so this file is
 * what survives a daemon restart. Nothing is cached here: each read goes to the file.
 */
export function createTurnJournal(file: string, now = () => Date.now()) {
  let adds = 0;
  let queue: Promise<unknown> = Promise.resolve();

  function serial<T>(task: () => Promise<T>): Promise<T> {
    const next = queue.then(task, task);
    queue = next.catch(() => undefined);
    return next;
  }

  async function readAll() {
    const text = await readFile(file, "utf8").catch((error: NodeJS.ErrnoException) => {
      if (error.code === "ENOENT") return "";
      throw error;
    });
    const entries: TurnHistoryEntry[] = [];
    for (const line of text.split("\n")) {
      if (!line.trim()) continue;
      try {
        const parsed = turnHistoryEntrySchema.safeParse(JSON.parse(line));
        if (parsed.success) entries.push(parsed.data);
      } catch {
        // A torn line from a crash must not hide the rest; the next prune drops it.
      }
    }
    return { entries, torn: text.length > 0 && !text.endsWith("\n") };
  }

  const fresh = (entry: TurnHistoryEntry) =>
    Date.parse(entry.endedAt) >= now() - JOURNAL_MAX_DAYS * DAY_MS;

  /** Rewrites the file without expired entries and returns what was dropped. */
  async function prune(): Promise<TurnHistoryEntry[]> {
    const { entries, torn } = await readAll();
    const kept = entries.filter(fresh).slice(-JOURNAL_MAX_ENTRIES);
    if (kept.length === entries.length && !torn) return [];
    const keys = new Set(kept.map((entry) => entry.key));
    const temporary = `${file}.tmp`;
    await writeFile(temporary, kept.map((entry) => `${JSON.stringify(entry)}\n`).join(""), {
      mode: 0o600,
    });
    await rename(temporary, file);
    return entries.filter((entry) => !keys.has(entry.key));
  }

  return {
    /** Appends one card and returns the entries that expired, so their git refs can go. */
    add(entry: TurnHistoryEntry): Promise<TurnHistoryEntry[]> {
      return serial(async () => {
        await mkdir(path.dirname(file), { recursive: true, mode: 0o700 });
        await appendFile(file, `${JSON.stringify(entry)}\n`, { mode: 0o600 });
        return adds++ % PRUNE_EVERY === 0 ? prune() : [];
      });
    },
    /** One agent's cards, newest first. */
    list(agentId: string, limit = HISTORY_LIMIT): Promise<TurnHistoryEntry[]> {
      return serial(async () =>
        (await readAll()).entries
          .filter((entry) => entry.agentId === agentId && fresh(entry))
          .reverse()
          .slice(0, limit),
      );
    },
  };
}
