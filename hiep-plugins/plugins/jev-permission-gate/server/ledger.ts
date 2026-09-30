import { createHash } from "node:crypto";
import { appendFile, mkdir } from "node:fs/promises";
import path from "node:path";
import type { GateResult } from "./policy";

export interface LedgerEntry extends GateResult {
  ts: string;
  agentId: string;
  requestId: string;
  commandHash: string;
  ms: number;
}

export function hashCommand(command: string): string {
  return createHash("sha256").update(command).digest("hex").slice(0, 16);
}

/** Append-only JSONL ledger; command text is never stored, only its hash. */
export function createLedger(file: string) {
  let ready: Promise<void> | undefined;
  return async (entry: LedgerEntry) => {
    ready ??= mkdir(path.dirname(file), { recursive: true, mode: 0o700 }).then(() => {});
    await ready;
    await appendFile(file, JSON.stringify(entry) + "\n", { mode: 0o600 });
  };
}
