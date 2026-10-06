import { mkdir, readFile, rename, writeFile } from "node:fs/promises";
import path from "node:path";
import type { PrSnapshot } from "./diff";

export interface PrWatch {
  agentId: string;
  cwd: string;
  number: number;
  url: string;
  createdAt: string;
  /** The last successful read; the next read is compared with it. */
  snapshot: PrSnapshot;
  failedReads: number;
}

const key = (watch: Pick<PrWatch, "agentId" | "number">) => `${watch.agentId}#${watch.number}`;

export interface PrWatchStore {
  load(): Promise<void>;
  list(): PrWatch[];
  listForAgent(agentId: string): PrWatch[];
  put(watch: PrWatch): Promise<void>;
  remove(agentId: string, number?: number): Promise<PrWatch[]>;
}

export function createWatchStore(file: string): PrWatchStore {
  const watches = new Map<string, PrWatch>();

  async function save(): Promise<void> {
    await mkdir(path.dirname(file), { recursive: true });
    const temp = `${file}.${process.pid}.tmp`;
    await writeFile(temp, JSON.stringify([...watches.values()], null, 2));
    await rename(temp, file);
  }

  return {
    async load() {
      let raw: string;
      try {
        raw = await readFile(file, "utf8");
      } catch (error) {
        if ((error as NodeJS.ErrnoException).code === "ENOENT") return;
        throw error;
      }
      const parsed: unknown = JSON.parse(raw);
      if (!Array.isArray(parsed)) return;
      for (const entry of parsed as PrWatch[]) {
        if (typeof entry?.agentId === "string" && typeof entry.number === "number") {
          watches.set(key(entry), entry);
        }
      }
    },
    list: () => [...watches.values()],
    listForAgent: (agentId) => [...watches.values()].filter((watch) => watch.agentId === agentId),
    async put(watch) {
      watches.set(key(watch), watch);
      await save();
    },
    async remove(agentId, number) {
      const removed = [...watches.values()].filter(
        (watch) => watch.agentId === agentId && (number === undefined || watch.number === number),
      );
      for (const watch of removed) watches.delete(key(watch));
      if (removed.length > 0) await save();
      return removed;
    },
  };
}
