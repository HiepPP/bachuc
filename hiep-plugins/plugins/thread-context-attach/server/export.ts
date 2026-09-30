import type { PaseoAgentHandle, PaseoApi } from "@getpaseo/client";
import { mkdir, readFile, rename, writeFile } from "node:fs/promises";
import { homedir } from "node:os";
import path from "node:path";
import type { Thread } from "../shared/threads";
import { toThread } from "./threads";

const PAGE_SIZE = 200;
const MAX_PAGES = 10;
const DEBOUNCE_MS = 2000;
const MAX_EXPORTS = 2;

export const CUT_LINE = "Older messages were cut. Turn numbers start at the oldest kept message.";

interface TimelineEntryLike {
  item: { type: string; text?: unknown };
}

type ExportedThread = Pick<Thread, "id" | "title" | "provider" | "cwd" | "lastActivityAt">;

export function exportDir(env: NodeJS.ProcessEnv = process.env): string {
  const home = env.PASEO_HOME || path.join(homedir(), ".paseo");
  return path.join(home, "plugin-data", "thread-context-attach", "threads");
}

function textOf(entry: TimelineEntryLike, type: string): string | null {
  const { item } = entry;
  return item.type === type && typeof item.text === "string" && item.text.trim() ? item.text : null;
}

/**
 * One `## Turn N` per user message with the assistant replies after it, so a search hit
 * points at one turn. Replies before the first user message form Turn 0. Other items are skipped.
 */
export function renderThreadMarkdown(
  thread: ExportedThread,
  entries: readonly TimelineEntryLike[],
  cut: boolean,
): string | null {
  const turns: { asked: string | null; answers: string[] }[] = [];
  for (const entry of entries) {
    const asked = textOf(entry, "user_message");
    if (asked !== null) {
      turns.push({ asked, answers: [] });
      continue;
    }
    const answer = textOf(entry, "assistant_message");
    if (answer === null) continue;
    if (!turns.length) turns.push({ asked: null, answers: [] });
    turns[turns.length - 1].answers.push(answer);
  }
  if (!turns.length) return null;
  const lines = [
    `# ${thread.title}`,
    "",
    `Thread: ${thread.id} · ${thread.provider} · ${thread.cwd}`,
    `Updated: ${thread.lastActivityAt}`,
    ...(cut ? [CUT_LINE] : []),
  ];
  const offset = turns[0].asked === null ? 0 : 1;
  turns.forEach((turn, index) => {
    lines.push("", `## Turn ${index + offset}`);
    if (turn.asked !== null) lines.push("", "### Asked", "", turn.asked);
    lines.push(
      "",
      "### Answer",
      "",
      turn.answers.length ? turn.answers.join("\n\n") : "(no reply yet)",
    );
  });
  return `${lines.join("\n")}\n`;
}

/** Reads up to MAX_PAGES pages, newest first; returns entries oldest first. */
export async function readTimeline(
  handle: PaseoAgentHandle,
): Promise<{ entries: TimelineEntryLike[]; cut: boolean }> {
  let entries: TimelineEntryLike[] = [];
  let cursor: { epoch: string; seq: number } | undefined;
  for (let page = 0; page < MAX_PAGES; page++) {
    const result = await handle.timeline.refetch(
      cursor
        ? { direction: "before", cursor, limit: PAGE_SIZE }
        : { direction: "tail", limit: PAGE_SIZE },
    );
    if (result.error) throw new Error(result.error);
    entries = [...result.entries, ...entries];
    if (!result.hasOlder || !result.startCursor) return { entries, cut: false };
    cursor = result.startCursor;
  }
  return { entries, cut: true };
}

/** Writes atomically with mode 0600; returns false when the file already holds this text. */
export async function writeIfChanged(dir: string, id: string, text: string): Promise<boolean> {
  const file = path.join(dir, `${id}.md`);
  const current = await readFile(file, "utf8").catch(() => null);
  if (current === text) return false;
  await mkdir(dir, { recursive: true, mode: 0o700 });
  await writeFile(`${file}.tmp`, text, { mode: 0o600 });
  await rename(`${file}.tmp`, file);
  return true;
}

export async function exportThread(
  paseo: Pick<PaseoApi, "agents">,
  agentId: string,
  dir: string,
): Promise<boolean> {
  const handle = paseo.agents.ref(agentId);
  const refreshed = await handle.refresh();
  if (!refreshed) return false;
  const { entries, cut } = await readTimeline(handle);
  const text = renderThreadMarkdown(toThread(refreshed.agent), entries, cut);
  return text === null ? false : writeIfChanged(dir, agentId, text);
}

type ExportOne = (paseo: PaseoApi, agentId: string) => Promise<boolean>;

export interface ExporterOptions {
  exportOne: ExportOne;
  log: (line: string) => void;
  debounceMs?: number;
  maxRunning?: number;
}

/** Debounces exports per thread, runs at most `maxRunning` at once, and reports changed files. */
export function createExporter(options: ExporterOptions) {
  const debounceMs = options.debounceMs ?? DEBOUNCE_MS;
  const maxRunning = options.maxRunning ?? MAX_EXPORTS;
  const timers = new Map<string, ReturnType<typeof setTimeout>>();
  const listeners = new Set<() => void>();
  const waiting: (() => void)[] = [];
  let running = 0;
  let stopped = false;
  let backfilled = false;

  async function withSlot<T>(run: () => Promise<T>): Promise<T> {
    if (running < maxRunning) running++;
    else await new Promise<void>((resolve) => waiting.push(resolve));
    try {
      return await run();
    } finally {
      const next = waiting.shift();
      if (next) next();
      else running--;
    }
  }

  async function run(paseo: PaseoApi, agentId: string): Promise<boolean> {
    if (stopped) return false;
    try {
      return await withSlot(() => options.exportOne(paseo, agentId));
    } catch (error) {
      options.log(
        `export ${agentId} failed: ${error instanceof Error ? error.message : String(error)}`,
      );
      return false;
    }
  }

  function notify() {
    for (const listener of listeners) listener();
  }

  return {
    schedule(paseo: PaseoApi, agentId: string) {
      if (stopped) return;
      clearTimeout(timers.get(agentId));
      timers.set(
        agentId,
        setTimeout(() => {
          timers.delete(agentId);
          void run(paseo, agentId).then((changed) => {
            if (changed) notify();
          });
        }, debounceMs),
      );
    },
    /** Exports every listed thread once, one at a time; later calls do nothing. */
    async backfill(paseo: PaseoApi, agentIds: readonly string[]) {
      if (backfilled || stopped) return;
      backfilled = true;
      let changed = 0;
      for (const agentId of agentIds) {
        if (stopped) return;
        if (await run(paseo, agentId)) changed++;
      }
      options.log(`backfill exported ${changed} of ${agentIds.length} threads`);
      notify();
    },
    onExported(listener: () => void) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    stop() {
      stopped = true;
      for (const timer of timers.values()) clearTimeout(timer);
      timers.clear();
      listeners.clear();
    },
  };
}
