import { readFile, stat } from "node:fs/promises";
import { Worker } from "node:worker_threads";

export type ClaudeHistoryRecord = Record<string, unknown>;

// Large transcripts (100MB+ is common for long sessions) cost over a second of read and
// JSON.parse; below this size a worker's startup costs more than parsing inline.
const WORKER_THRESHOLD_BYTES = 4 * 1024 * 1024;

// The worker is evaluated from a string so it runs the same from TS source (tsx, vitest) and
// from the compiled dist, without a separate entry file or loader.
const PARSE_WORKER_SOURCE = `
const { parentPort, workerData } = require("node:worker_threads");
const { readFile } = require("node:fs/promises");
readFile(workerData, "utf8").then(
  (content) => {
    const records = [];
    for (const line of content.split(/\\r?\\n/)) {
      const trimmed = line.trim();
      if (!trimmed) continue;
      try {
        const value = JSON.parse(trimmed);
        if (value && typeof value === "object" && !Array.isArray(value)) records.push(value);
      } catch {}
    }
    parentPort.postMessage({ records });
  },
  (error) => parentPort.postMessage({ error: String(error && error.message ? error.message : error) }),
);
`;

export function parseClaudeHistoryRecords(content: string): ClaudeHistoryRecord[] {
  const records: ClaudeHistoryRecord[] = [];
  for (const line of content.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (!trimmed) continue;
    try {
      const value: unknown = JSON.parse(trimmed);
      if (value && typeof value === "object" && !Array.isArray(value)) {
        records.push(value as ClaudeHistoryRecord);
      }
    } catch {
      // Claude can leave a partial last line while it is still writing.
    }
  }
  return records;
}

export interface ReadClaudeHistoryRecordsOptions {
  workerThresholdBytes?: number;
}

interface ParseWorkerMessage {
  records?: ClaudeHistoryRecord[];
  error?: string;
}

/** Reads a Claude JSONL transcript; large files are read and parsed off the main thread. */
export async function readClaudeHistoryRecords(
  filePath: string,
  options: ReadClaudeHistoryRecordsOptions = {},
): Promise<ClaudeHistoryRecord[]> {
  const { size } = await stat(filePath);
  if (size < (options.workerThresholdBytes ?? WORKER_THRESHOLD_BYTES)) {
    return parseClaudeHistoryRecords(await readFile(filePath, "utf8"));
  }
  return new Promise((resolve, reject) => {
    const worker = new Worker(PARSE_WORKER_SOURCE, { eval: true, workerData: filePath });
    worker.once("message", (message: ParseWorkerMessage) => {
      if (message.records) {
        resolve(message.records);
      } else {
        reject(new Error(message.error ?? "Claude history worker failed"));
      }
    });
    worker.once("error", reject);
    worker.once("exit", (code) => {
      reject(new Error(`Claude history worker exited with code ${code}`));
    });
  });
}
