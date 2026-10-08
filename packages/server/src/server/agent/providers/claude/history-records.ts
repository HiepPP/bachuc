import { readFile, stat } from "node:fs/promises";
import { Worker } from "node:worker_threads";

export type ClaudeHistoryRecord = Record<string, unknown>;

// Large transcripts (100MB+ is common for long sessions) cost over a second of read and
// JSON.parse; below this size a worker's startup costs more than parsing inline.
const WORKER_THRESHOLD_BYTES = 4 * 1024 * 1024;
// Decoding one structured-clone message runs on the main thread in a single task. Posting the
// records of ~4MB of transcript per message keeps each decode short, where one message for a
// 115MB transcript blocked the daemon for ~185ms.
const WORKER_BATCH_BYTES = 4 * 1024 * 1024;

// The worker is evaluated from a string so it runs the same from TS source (tsx, vitest) and
// from the compiled dist, without a separate entry file or loader.
const PARSE_WORKER_SOURCE = `
const { parentPort, workerData } = require("node:worker_threads");
const { readFile } = require("node:fs/promises");
async function parseFiles() {
  for (let file = 0; file < workerData.filePaths.length; file++) {
    const content = await readFile(workerData.filePaths[file], "utf8");
    let records = [];
    let batchBytes = 0;
    for (const line of content.split(/\\r?\\n/)) {
      const trimmed = line.trim();
      if (!trimmed) continue;
      try {
        const value = JSON.parse(trimmed);
        if (!value || typeof value !== "object" || Array.isArray(value)) continue;
        records.push(value);
        batchBytes += trimmed.length;
      } catch {
        continue;
      }
      if (batchBytes >= workerData.batchBytes) {
        parentPort.postMessage({ type: "records", file, records });
        records = [];
        batchBytes = 0;
      }
    }
    if (records.length > 0) parentPort.postMessage({ type: "records", file, records });
  }
  parentPort.postMessage({ type: "done" });
}
parseFiles().catch((error) =>
  parentPort.postMessage({
    type: "error",
    error: String(error && error.message ? error.message : error),
  }),
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
  workerBatchBytes?: number;
}

type ParseWorkerMessage =
  | { type: "records"; file: number; records: ClaudeHistoryRecord[] }
  | { type: "done" }
  | { type: "error"; error: string };

/**
 * Reads Claude JSONL transcripts and returns each file's records in input order. When the files
 * are large together, one worker reads and parses all of them off the main thread.
 */
export async function readClaudeHistoryRecordFiles(
  filePaths: readonly string[],
  options: ReadClaudeHistoryRecordsOptions = {},
): Promise<ClaudeHistoryRecord[][]> {
  const sizes = await Promise.all(filePaths.map(async (filePath) => (await stat(filePath)).size));
  const totalBytes = sizes.reduce((sum, size) => sum + size, 0);
  if (totalBytes < (options.workerThresholdBytes ?? WORKER_THRESHOLD_BYTES)) {
    const recordsByFile: ClaudeHistoryRecord[][] = [];
    for (const filePath of filePaths) {
      recordsByFile.push(parseClaudeHistoryRecords(await readFile(filePath, "utf8")));
    }
    return recordsByFile;
  }
  return new Promise((resolve, reject) => {
    const recordsByFile: ClaudeHistoryRecord[][] = filePaths.map(() => []);
    const worker = new Worker(PARSE_WORKER_SOURCE, {
      eval: true,
      workerData: { filePaths, batchBytes: options.workerBatchBytes ?? WORKER_BATCH_BYTES },
    });
    worker.on("message", (message: ParseWorkerMessage) => {
      if (message.type === "records") {
        const records = recordsByFile[message.file];
        for (const record of message.records) records?.push(record);
      } else if (message.type === "done") {
        resolve(recordsByFile);
      } else {
        reject(new Error(message.error));
      }
    });
    worker.once("error", reject);
    worker.once("exit", (code) => {
      reject(new Error(`Claude history worker exited with code ${code}`));
    });
  });
}
