import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { mkdir } from "node:fs/promises";
import path from "node:path";

/** A named index keeps `qmd update` away from the user's default index and its collections. */
export const QMD_INDEX = "paseo-threads";
export const QMD_COLLECTION = "paseo-threads";
const UPDATE_INTERVAL_MS = 30_000;
/** Embedding loads a ~330 MB model, so changed threads are batched into one run per interval. */
const EMBED_INTERVAL_MS = 600_000;
const TIMEOUT_MS = 120_000;
/** The first embed downloads a ~330 MB model and embeds every thread. */
const EMBED_TIMEOUT_MS = 600_000;
const MAX_STDOUT = 64_000;
const FALLBACK_BINS = ["/opt/homebrew/bin/qmd", "/usr/local/bin/qmd"];

/** The daemon PATH may miss Homebrew, so known install paths are checked last. */
export function findQmd(
  env: NodeJS.ProcessEnv = process.env,
  exists: (file: string) => boolean = existsSync,
): string | null {
  if (env.QMD_BIN && exists(env.QMD_BIN)) return env.QMD_BIN;
  for (const dir of (env.PATH ?? "").split(path.delimiter)) {
    if (dir && exists(path.join(dir, "qmd"))) return path.join(dir, "qmd");
  }
  return FALLBACK_BINS.find((file) => exists(file)) ?? null;
}

export interface RunResult {
  code: number | null;
  stdout: string;
}
export type Runner = (
  args: string[],
  signal: AbortSignal,
  timeoutMs?: number,
) => Promise<RunResult>;

/** Runs `qmd --index paseo-threads <args>` without a shell, with a timeout. */
export function createRunner(bin: string, env: NodeJS.ProcessEnv = process.env): Runner {
  // The qmd launcher runs `node` from PATH, and its native modules match the user's first
  // `node`. Keep the user's order; qmd's own folder is only a fallback.
  const PATH = [env.PATH, path.dirname(bin)].filter(Boolean).join(path.delimiter);
  return (args, signal, timeoutMs = TIMEOUT_MS) =>
    new Promise((resolve, reject) => {
      const child = spawn(bin, ["--index", QMD_INDEX, ...args], {
        stdio: ["ignore", "pipe", "ignore"],
        env: { ...env, PATH },
      });
      let stdout = "";
      const kill = () => child.kill("SIGKILL");
      const timer = setTimeout(kill, timeoutMs);
      signal.addEventListener("abort", kill, { once: true });
      const done = () => {
        clearTimeout(timer);
        signal.removeEventListener("abort", kill);
      };
      child.stdout.on("data", (data) => {
        if (stdout.length < MAX_STDOUT) stdout += data;
      });
      child.on("error", (error) => {
        done();
        reject(error);
      });
      child.on("close", (code) => {
        done();
        resolve({ code, stdout });
      });
    });
}

export interface IndexerOptions {
  dir: string;
  run: Runner;
  log: (line: string) => void;
  intervalMs?: number;
  embedIntervalMs?: number;
}

/**
 * Reads `Indexed: 1 new, 2 updated, 3 unchanged, 0 removed` from `qmd update`. Returns true when
 * documents were added or changed, and also when the summary is missing, so a format change in
 * qmd delays vectors at worst instead of stopping them.
 */
export function updateChanged(stdout: string): boolean {
  const match = /Indexed:\s*(\d+) new,\s*(\d+) updated/.exec(stdout);
  return !match || Number(match[1]) + Number(match[2]) > 0;
}

/**
 * Adds the export folder as a collection once, then runs `update` at most once per interval.
 * `update` alone leaves new and changed threads without vectors, so `query` misses them; `embed`
 * runs only after an update added or changed documents, at most once per embed interval.
 * qmd commands never overlap; a notify during an update schedules one more update.
 * qmd output is not logged because it can quote thread text.
 */
export function createIndexer(options: IndexerOptions) {
  const intervalMs = options.intervalMs ?? UPDATE_INTERVAL_MS;
  const embedIntervalMs = options.embedIntervalMs ?? EMBED_INTERVAL_MS;
  const controller = new AbortController();
  let ready: Promise<boolean> | undefined;
  let timer: ReturnType<typeof setTimeout> | undefined;
  let embedTimer: ReturnType<typeof setTimeout> | undefined;
  let queue: Promise<unknown> = Promise.resolve();
  let running = false;
  let dirty = false;
  let lastRun = -Infinity;
  let lastEmbed = -Infinity;
  // Vectors may be missing from an earlier process, so the first successful update embeds once.
  let embedPending = true;
  let updated = false;

  // update and embed share the qmd index, so they run one at a time.
  function exclusive<T>(task: () => Promise<T>): Promise<T> {
    const next = queue.then(task, task);
    queue = next.catch(() => {});
    return next;
  }

  async function ensureCollection(): Promise<boolean> {
    try {
      await mkdir(options.dir, { recursive: true, mode: 0o700 });
      const list = await options.run(["collection", "list"], controller.signal);
      if (list.code !== 0) {
        options.log(`qmd collection list failed with exit ${list.code}`);
        return false;
      }
      if (list.stdout.includes(`qmd://${QMD_COLLECTION}/`)) return true;
      const added = await options.run(
        ["collection", "add", options.dir, "--name", QMD_COLLECTION, "--mask", "**/*.md"],
        controller.signal,
      );
      options.log(`qmd collection add exit ${added.code}`);
      return added.code === 0;
    } catch (error) {
      options.log(`qmd setup failed: ${error instanceof Error ? error.message : String(error)}`);
      return false;
    }
  }

  async function update() {
    timer = undefined;
    if (controller.signal.aborted || !(await (ready ?? start()))) return;
    running = true;
    try {
      await exclusive(async () => {
        lastRun = Date.now();
        const result = await options.run(["update"], controller.signal);
        options.log(`qmd update exit ${result.code} in ${Date.now() - lastRun} ms`);
        if (result.code !== 0) return;
        updated = true;
        if (updateChanged(result.stdout)) embedPending = true;
      });
    } catch (error) {
      options.log(`qmd update failed: ${error instanceof Error ? error.message : String(error)}`);
    } finally {
      running = false;
    }
    scheduleEmbed();
    if (dirty) {
      dirty = false;
      notify();
    }
  }

  async function embed() {
    embedTimer = undefined;
    if (controller.signal.aborted || !embedPending) return;
    embedPending = false;
    try {
      await exclusive(async () => {
        lastEmbed = Date.now();
        const result = await options.run(["embed"], controller.signal, EMBED_TIMEOUT_MS);
        options.log(`qmd embed exit ${result.code} in ${Date.now() - lastEmbed} ms`);
        if (result.code !== 0) embedPending = true;
      });
    } catch (error) {
      embedPending = true;
      options.log(`qmd embed failed: ${error instanceof Error ? error.message : String(error)}`);
    }
    scheduleEmbed();
  }

  function scheduleEmbed() {
    if (controller.signal.aborted || !embedPending || !updated || embedTimer) return;
    embedTimer = setTimeout(
      () => void embed(),
      Math.max(0, lastEmbed + embedIntervalMs - Date.now()),
    );
  }

  function start(): Promise<boolean> {
    ready ??= ensureCollection();
    return ready;
  }

  function notify() {
    if (controller.signal.aborted) return;
    if (running) {
      dirty = true;
      return;
    }
    if (timer) return;
    timer = setTimeout(() => void update(), Math.max(0, lastRun + intervalMs - Date.now()));
  }

  return {
    start,
    notify,
    stop() {
      controller.abort();
      clearTimeout(timer);
      clearTimeout(embedTimer);
      timer = undefined;
      embedTimer = undefined;
    },
  };
}
