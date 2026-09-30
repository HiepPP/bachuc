import { mkdir, readFile, rename, writeFile } from "node:fs/promises";
import path from "node:path";
import { runStateSchema, type RunState } from "./store";

export function createRunPersistence(file: string) {
  let saved: string | null = null;
  let queue: Promise<void> = Promise.resolve();

  return {
    async load(): Promise<RunState | null> {
      const body = await readFile(file, "utf8").catch((error: NodeJS.ErrnoException) => {
        if (error.code === "ENOENT") return null;
        throw error;
      });
      if (body === null) return null;
      const state = runStateSchema.parse(JSON.parse(body));
      saved = JSON.stringify(state, null, 2) + "\n";
      return state;
    },
    save(state: RunState): Promise<void> {
      const body = JSON.stringify(state, null, 2) + "\n";
      const next = queue.then(async () => {
        if (body === saved) return;
        await mkdir(path.dirname(file), { recursive: true, mode: 0o700 });
        const temporary = `${file}.${process.pid}.tmp`;
        await writeFile(temporary, body, { mode: 0o600 });
        await rename(temporary, file);
        saved = body;
      });
      queue = next.catch(() => undefined);
      return next;
    },
    flush(): Promise<void> {
      return queue;
    },
  };
}
