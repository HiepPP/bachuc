import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import { afterEach, beforeEach, describe, expect, test } from "vitest";

import { readClaudeHistoryRecordFiles } from "./history-records.js";

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

const SIDECHAIN = [
  JSON.stringify({ type: "assistant", uuid: "s1", isSidechain: true, agentId: "a" }),
  JSON.stringify({ type: "user", uuid: "s2", isSidechain: true, agentId: "a" }),
].join("\n");

const SIDECHAIN_EXPECTED = [
  { type: "assistant", uuid: "s1", isSidechain: true, agentId: "a" },
  { type: "user", uuid: "s2", isSidechain: true, agentId: "a" },
];

describe("readClaudeHistoryRecordFiles", () => {
  let tempRoot: string;
  let filePath: string;
  let sidechainPath: string;

  beforeEach(() => {
    tempRoot = mkdtempSync(path.join(os.tmpdir(), "claude-history-records-"));
    filePath = path.join(tempRoot, "session.jsonl");
    sidechainPath = path.join(tempRoot, "agent-a.jsonl");
    writeFileSync(filePath, TRANSCRIPT);
    writeFileSync(sidechainPath, SIDECHAIN);
  });

  afterEach(() => {
    rmSync(tempRoot, { recursive: true, force: true });
  });

  test("parses small transcripts inline, skipping blank, partial, and non-object lines", async () => {
    await expect(readClaudeHistoryRecordFiles([filePath])).resolves.toEqual([EXPECTED]);
  });

  test("parses large transcripts in a worker with the same result", async () => {
    await expect(
      readClaudeHistoryRecordFiles([filePath], { workerThresholdBytes: 0 }),
    ).resolves.toEqual([EXPECTED]);
  });

  test("keeps each file's records apart and in order across worker batches", async () => {
    await expect(
      readClaudeHistoryRecordFiles([filePath, sidechainPath], {
        workerThresholdBytes: 0,
        workerBatchBytes: 1,
      }),
    ).resolves.toEqual([EXPECTED, SIDECHAIN_EXPECTED]);
  });

  test("rejects when a transcript is missing", async () => {
    await expect(
      readClaudeHistoryRecordFiles([filePath, path.join(tempRoot, "missing.jsonl")], {
        workerThresholdBytes: 0,
      }),
    ).rejects.toThrow();
  });
});
