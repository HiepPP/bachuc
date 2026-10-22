// Copied from hiep-plugins/plugins/prompt-translate/server/store.ts without its `pairs` map,
// `original`, and `pair` (ADR-0062). A fix to the cache must be made in both plugins.
import { createHash } from "node:crypto";
import { mkdir, readFile, rename, writeFile } from "node:fs/promises";
import path from "node:path";

export function hash(...parts: string[]): string {
  return createHash("sha256").update(parts.join("\u0000")).digest("hex");
}

type Saved = { version: 1; cache: [string, string][] };

export class Store {
  private readonly cache = new Map<string, string>();
  private readonly inflight = new Map<string, Promise<string>>();
  private readonly loaded: Promise<void>;
  private timer: ReturnType<typeof setTimeout> | undefined;

  constructor(
    private readonly file: string,
    private readonly limits = { cache: 500 },
    private readonly delay = 500,
  ) {
    this.loaded = this.load();
  }

  async get(key: string): Promise<string | undefined> {
    await this.loaded;
    const value = this.cache.get(key);
    if (value !== undefined) {
      this.cache.delete(key);
      this.cache.set(key, value);
    }
    return value;
  }

  // Register the in-flight promise before any await so a concurrent caller cannot miss it.
  run(key: string, task: () => Promise<string>): Promise<string> {
    const pending = this.inflight.get(key);
    if (pending) return pending;
    const promise = (async () => {
      const hit = await this.get(key);
      if (hit !== undefined) return hit;
      const value = await task();
      this.remember(key, value);
      return value;
    })().finally(() => this.inflight.delete(key));
    this.inflight.set(key, promise);
    return promise;
  }

  async flush(): Promise<void> {
    clearTimeout(this.timer);
    this.timer = undefined;
    const saved: Saved = { version: 1, cache: [...this.cache] };
    try {
      await mkdir(path.dirname(this.file), { recursive: true });
      const temporary = `${this.file}.tmp`;
      await writeFile(temporary, JSON.stringify(saved));
      await rename(temporary, this.file);
    } catch (error) {
      // A failed write only costs future cache hits; never fail the user's request for it.
      console.warn(
        `selection-translate: cache write failed: ${error instanceof Error ? error.message : String(error)}`,
      );
    }
  }

  close(): void {
    if (this.timer) void this.flush();
  }

  private async load(): Promise<void> {
    try {
      const saved = JSON.parse(await readFile(this.file, "utf8")) as Partial<Saved>;
      for (const [key, value] of saved.cache ?? [])
        if (typeof key === "string" && typeof value === "string") this.cache.set(key, value);
    } catch {
      // Missing or corrupt: start empty; the next flush replaces the file.
    }
  }

  private remember(key: string, value: string): void {
    this.cache.delete(key);
    this.cache.set(key, value);
    while (this.cache.size > this.limits.cache)
      this.cache.delete(this.cache.keys().next().value as string);
    this.timer ??= setTimeout(() => void this.flush(), this.delay);
  }
}
