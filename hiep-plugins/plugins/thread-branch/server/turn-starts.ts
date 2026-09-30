import { mkdir, readdir, readFile, rename, rm, stat, writeFile } from "node:fs/promises";
import path from "node:path";

export interface StartRecord {
  turnId: string | null;
  cwd: string;
  root: string;
  head: string;
  tree: string;
  savedAt: number;
}

export interface StartStore {
  save(agentId: string, record: StartRecord): Promise<void>;
  /** Reads and deletes the record, so each start is used once. */
  take(agentId: string): Promise<StartRecord | null>;
  remove(agentId: string): Promise<void>;
}

// A turn older than this is not resumed; its record is left over from a crash.
const MAX_AGE_MS = 24 * 60 * 60 * 1000;

function isRecord(value: unknown): value is StartRecord {
  const record = value as StartRecord;
  return (
    typeof record === "object" &&
    record !== null &&
    (record.turnId === null || typeof record.turnId === "string") &&
    ["cwd", "root", "head", "tree"].every(
      (key) => typeof record[key as keyof StartRecord] === "string",
    ) &&
    typeof record.savedAt === "number"
  );
}

/**
 * One small file per running agent with its turn-start snapshot. A plugin reload mid-turn drops
 * the in-memory start, and this lets the turn end still produce its card.
 */
export function createStartStore(dir: string, now = () => Date.now()): StartStore {
  const file = (agentId: string) =>
    path.join(dir, `${agentId.replace(/[^A-Za-z0-9._-]/g, "_")}.json`);
  let pruned = false;

  async function prune() {
    const names = await readdir(dir).catch(() => []);
    for (const name of names) {
      const entry = path.join(dir, name);
      const info = await stat(entry).catch(() => null);
      if (info && now() - info.mtimeMs > MAX_AGE_MS) await rm(entry, { force: true });
    }
  }

  return {
    async save(agentId, record) {
      await mkdir(dir, { recursive: true, mode: 0o700 });
      const target = file(agentId);
      await writeFile(`${target}.tmp`, JSON.stringify(record), { mode: 0o600 });
      await rename(`${target}.tmp`, target);
      if (!pruned) {
        pruned = true;
        await prune();
      }
    },
    async take(agentId) {
      const target = file(agentId);
      const text = await readFile(target, "utf8").catch(() => null);
      if (text === null) return null;
      await rm(target, { force: true });
      try {
        const record: unknown = JSON.parse(text);
        return isRecord(record) && now() - record.savedAt < MAX_AGE_MS ? record : null;
      } catch {
        return null;
      }
    },
    async remove(agentId) {
      await rm(file(agentId), { force: true });
    },
  };
}
