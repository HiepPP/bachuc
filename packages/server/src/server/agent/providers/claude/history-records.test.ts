import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import { afterEach, beforeEach, describe, expect, test } from "vitest";

import { readClaudeHistoryRecords } from "./history-records.js";

const TRANSCRIPT = [
  JSON.stringify({ type: "user", uuid: "u1", message: { role: "user", content: "hi" } }),
  "",
  "   ",
  '{"type":"assistant","uuid":"a1"',
  JSON.stringify(["not", "a", "record"]),
  JSON.stringify({ type: "assistant", uuid: "a2", isSidechain: true }),
].join("\r\n");

const EXPECTED = [
  { type: "user", uuid: "u1", message: { role: "user", content: "hi" } },
  { type: "assistant", uuid: "a2", isSidechain: true },
];

describe("readClaudeHistoryRecords", () => {
  let tempRoot: string;
  let filePath: string;

  beforeEach(() => {
    tempRoot = mkdtempSync(path.join(os.tmpdir(), "claude-history-records-"));
    filePath = path.join(tempRoot, "session.jsonl");
    writeFileSync(filePath, TRANSCRIPT);
  });

  afterEach(() => {
    rmSync(tempRoot, { recursive: true, force: true });
  });

  test("parses small transcripts inline, skipping blank, partial, and non-object lines", async () => {
    await expect(readClaudeHistoryRecords(filePath)).resolves.toEqual(EXPECTED);
  });

  test("parses large transcripts in a worker with the same result", async () => {
    await expect(readClaudeHistoryRecords(filePath, { workerThresholdBytes: 0 })).resolves.toEqual(
      EXPECTED,
    );
  });

  test("keeps record order when the worker posts several batches", async () => {
    await expect(
      readClaudeHistoryRecords(filePath, { workerThresholdBytes: 0, workerBatchBytes: 1 }),
    ).resolves.toEqual(EXPECTED);
  });

  test("rejects when the transcript is missing", async () => {
    await expect(
      readClaudeHistoryRecords(path.join(tempRoot, "missing.jsonl"), { workerThresholdBytes: 0 }),
    ).rejects.toThrow();
  });
});
