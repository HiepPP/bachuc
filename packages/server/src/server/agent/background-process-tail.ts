import { open } from "node:fs/promises";

import type { AgentBackgroundProcess } from "./agent-sdk-types.js";

const TAIL_BYTES = 8192;
const TAIL_LINES = 5;
const MAX_LINE_LENGTH = 300;
// eslint-disable-next-line no-control-regex -- ANSI escape sequences start with ESC.
const ANSI_PATTERN = /\u001b\[[0-9;?]*[ -/]*[@-~]/g;

/** The last non-empty lines of a process output file. A missing file has no lines yet. */
export async function readOutputTail(path: string): Promise<string[]> {
  let handle;
  try {
    handle = await open(path, "r");
  } catch {
    return [];
  }
  try {
    const { size } = await handle.stat();
    const length = Math.min(size, TAIL_BYTES);
    const buffer = Buffer.alloc(length);
    await handle.read(buffer, 0, length, size - length);
    return buffer
      .toString("utf8")
      .replace(ANSI_PATTERN, "")
      .split(/\r?\n|\r/)
      .map((line) => line.trimEnd())
      .filter((line) => line.trim().length > 0)
      .slice(-TAIL_LINES)
      .map((line) => (line.length > MAX_LINE_LENGTH ? `${line.slice(0, MAX_LINE_LENGTH)}…` : line));
  } finally {
    await handle.close();
  }
}

export interface BackgroundProcessOwner {
  agentId: string;
  process: AgentBackgroundProcess;
}

export interface BackgroundProcessTailPollerInput {
  intervalMs: number;
  /** Processes whose output file should be read on the next tick. */
  listWatched: () => BackgroundProcessOwner[];
  onTail: (input: { agentId: string; processId: string; tail: string[] }) => void;
}

/** One timer for every agent. It runs only while some process has an output file to watch. */
export class BackgroundProcessTailPoller {
  private timer: ReturnType<typeof setInterval> | null = null;
  private reading = false;

  constructor(private readonly input: BackgroundProcessTailPollerInput) {}

  ensureRunning(): void {
    if (this.timer) return;
    this.timer = setInterval(() => void this.tick(), this.input.intervalMs);
    (this.timer as unknown as { unref?: () => void }).unref?.();
  }

  stop(): void {
    if (this.timer) clearInterval(this.timer);
    this.timer = null;
  }

  async tick(): Promise<void> {
    if (this.reading) return;
    const watched = this.input.listWatched();
    if (watched.length === 0) {
      this.stop();
      return;
    }
    this.reading = true;
    try {
      await Promise.all(
        watched.map(async ({ agentId, process }) => {
          if (!process.outputFile) return;
          const tail = await readOutputTail(process.outputFile);
          this.input.onTail({ agentId, processId: process.id, tail });
        }),
      );
    } finally {
      this.reading = false;
    }
  }
}
