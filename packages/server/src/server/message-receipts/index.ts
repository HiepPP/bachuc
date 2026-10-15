import { createHash } from "node:crypto";
import { readdir, readFile, stat, unlink } from "node:fs/promises";
import path from "node:path";
import { z } from "zod";
import { writeJsonFileAtomic } from "../atomic-file.js";

// A receipt guards a retried request; 7 days is far past any retry window.
export const RECEIPT_MAX_AGE_MS = 7 * 24 * 60 * 60 * 1000;
const RECEIPT_SWEEP_INTERVAL_MS = 24 * 60 * 60 * 1000;

const ReceiptSchema = z.object({
  fingerprint: z.string(),
  state: z.enum(["pending", "completed"]),
  agentId: z.string(),
});
interface ReceiptSweepOptions {
  now?: () => number;
  maxAgeMs?: number;
  onError?: (error: unknown) => void;
}
interface SendMessageInput {
  agentId: string;
  messageId: string;
  request: unknown;
  send: () => Promise<void>;
  prepare?: () => Promise<void>;
}

/** Owns message delivery receipts; creation is owned by CreationService. */
export class MessageReceipts {
  private readonly pending = new Map<string, Promise<void>>();
  private sweepTimer: ReturnType<typeof setInterval> | null = null;
  constructor(
    private readonly directory: string,
    private readonly options: ReceiptSweepOptions = {},
  ) {}

  /** Sweeps now, then every 24 hours. The first sweep runs off the caller's path. */
  startSweeping(): void {
    if (this.sweepTimer) return;
    const run = () => {
      void this.sweep().catch((error: unknown) => this.options.onError?.(error));
    };
    this.sweepTimer = setInterval(run, RECEIPT_SWEEP_INTERVAL_MS);
    this.sweepTimer.unref();
    setImmediate(run);
  }

  stopSweeping(): void {
    if (!this.sweepTimer) return;
    clearInterval(this.sweepTimer);
    this.sweepTimer = null;
  }

  /** Deletes receipt files older than the age limit, judged by file mtime only. */
  async sweep(): Promise<number> {
    const { now = Date.now, maxAgeMs = RECEIPT_MAX_AGE_MS, onError } = this.options;
    let names: string[];
    try {
      names = await readdir(this.directory);
    } catch (error) {
      if (!isNotFound(error)) onError?.(error);
      return 0;
    }
    const cutoff = now() - maxAgeMs;
    let deleted = 0;
    for (const name of names) {
      if (!name.endsWith(".json")) continue;
      const file = path.join(this.directory, name);
      try {
        if ((await stat(file)).mtimeMs >= cutoff) continue;
        await unlink(file);
        deleted++;
      } catch (error) {
        if (!isNotFound(error)) onError?.(error);
      }
    }
    return deleted;
  }

  send(input: SendMessageInput): Promise<void> {
    // Preserve the existing on-disk identity and shape across daemon upgrades.
    const key = digest(["send", input.agentId, input.messageId]);
    const previous = this.pending.get(key);
    const result = (previous ? previous.catch(() => undefined) : Promise.resolve()).then(() =>
      this.sendOnce(key, input),
    );
    this.pending.set(key, result);
    void result
      .finally(() => {
        if (this.pending.get(key) === result) this.pending.delete(key);
      })
      .catch(() => undefined);
    return result;
  }

  private async sendOnce(key: string, input: SendMessageInput): Promise<void> {
    const file = path.join(this.directory, `${key}.json`);
    const fingerprint = digest(input.request);
    const existing = await readReceipt(file);
    if (existing) {
      if (existing.fingerprint !== fingerprint) throw new Error("agent_request_key_conflict");
      if (existing.state === "completed") return;
      // A provider may have accepted the message before its receipt was committed.
      throw new Error("agent_request_outcome_unknown");
    }
    await input.prepare?.();
    const receipt = { fingerprint, agentId: input.agentId };
    await writeJsonFileAtomic(file, { ...receipt, state: "pending" });
    await input.send();
    await writeJsonFileAtomic(file, { ...receipt, state: "completed" });
  }
}

async function readReceipt(file: string): Promise<z.infer<typeof ReceiptSchema> | null> {
  try {
    return ReceiptSchema.parse(JSON.parse(await readFile(file, "utf8")));
  } catch (error) {
    if (isNotFound(error)) return null;
    throw error;
  }
}

function isNotFound(error: unknown): boolean {
  return error instanceof Error && "code" in error && error.code === "ENOENT";
}

function digest(value: unknown): string {
  return createHash("sha256")
    .update(
      JSON.stringify(value, (_key, candidate: unknown) => {
        if (candidate !== null && typeof candidate === "object" && !Array.isArray(candidate)) {
          return Object.fromEntries(
            Object.entries(candidate).sort(([a], [b]) => a.localeCompare(b)),
          );
        }
        return candidate;
      }),
    )
    .digest("hex");
}
