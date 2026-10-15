import { mkdir, mkdtemp, readdir, rm, utimes, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { afterEach, expect, test, vi } from "vitest";
import { MessageReceipts, RECEIPT_MAX_AGE_MS } from "./index.js";

const directories: string[] = [];
afterEach(async () => {
  await Promise.all(
    directories.splice(0).map((directory) => rm(directory, { recursive: true, force: true })),
  );
});
async function fixture() {
  const directory = await mkdtemp(path.join(tmpdir(), "agent-requests-"));
  directories.push(directory);
  return { directory, requests: new MessageReceipts(directory) };
}

test("message retries survive reconstruction without submitting twice", async () => {
  const { requests, directory } = await fixture();
  let deliveries = 0;
  const input = {
    agentId: "agent",
    messageId: "arrival",
    request: { text: "hello" },
    send: async () => {
      deliveries++;
    },
  };
  await Promise.all([requests.send(input), requests.send(input)]);
  await new MessageReceipts(directory).send(input);
  expect(deliveries).toBe(1);
  await requests.send({ ...input, agentId: "another" });
  expect(deliveries).toBe(2);
});

test("ambiguous provider delivery is never blindly replayed after restart", async () => {
  const { requests, directory } = await fixture();
  let deliveries = 0;
  const input = {
    agentId: "agent",
    messageId: "arrival",
    request: {},
    send: async () => {
      deliveries++;
      throw new Error("connection lost");
    },
  };
  await expect(requests.send(input)).rejects.toThrow("connection lost");
  await expect(new MessageReceipts(directory).send(input)).rejects.toThrow(
    "agent_request_outcome_unknown",
  );
  expect(deliveries).toBe(1);
});

test("failed local message preparation does not leave an ambiguous receipt", async () => {
  const { requests, directory } = await fixture();
  let available = false;
  let sends = 0;
  const input = {
    agentId: "agent",
    messageId: "message",
    request: {},
    prepare: async () => {
      if (!available) throw new Error("load failed");
    },
    send: async () => {
      sends++;
    },
  };
  await expect(requests.send(input)).rejects.toThrow("load failed");
  available = true;
  await new MessageReceipts(directory).send(input);
  available = false;
  await requests.send(input);
  expect(sends).toBe(1);
});

const NOW = Date.parse("2026-10-15T00:00:00Z");
async function receiptFile(directory: string, name: string, ageMs: number): Promise<string> {
  const file = path.join(directory, name);
  await writeFile(file, "{}");
  const time = new Date(NOW - ageMs);
  await utimes(file, time, time);
  return file;
}

test("sweep deletes receipts older than the age limit and keeps newer ones", async () => {
  const { directory } = await fixture();
  await receiptFile(directory, "old.json", RECEIPT_MAX_AGE_MS + 1000);
  await receiptFile(directory, "new.json", RECEIPT_MAX_AGE_MS - 60_000);
  const requests = new MessageReceipts(directory, { now: () => NOW });
  expect(await requests.sweep()).toBe(1);
  expect(await readdir(directory)).toEqual(["new.json"]);
});

test("sweep skips a file it cannot delete and reports the error", async () => {
  const { directory } = await fixture();
  // A directory named like a receipt fails to unlink, and sorts before the real files.
  await mkdir(path.join(directory, "a-stuck.json"));
  const stuck = new Date(NOW - RECEIPT_MAX_AGE_MS - 1000);
  await utimes(path.join(directory, "a-stuck.json"), stuck, stuck);
  await receiptFile(directory, "b-old.json", RECEIPT_MAX_AGE_MS + 1000);
  const errors: unknown[] = [];
  const requests = new MessageReceipts(directory, {
    now: () => NOW,
    onError: (error) => errors.push(error),
  });
  expect(await requests.sweep()).toBe(1);
  expect(errors).toHaveLength(1);
  expect(await readdir(directory)).toEqual(["a-stuck.json"]);
});

test("a retried request inside the age limit still finds its receipt after a sweep", async () => {
  const { directory } = await fixture();
  let deliveries = 0;
  const input = {
    agentId: "agent",
    messageId: "arrival",
    request: {},
    send: async () => {
      deliveries++;
    },
  };
  await new MessageReceipts(directory).send(input);
  const requests = new MessageReceipts(directory);
  await requests.sweep();
  await requests.send(input);
  expect(deliveries).toBe(1);
});

test("startSweeping returns before it deletes anything, then sweeps in the background", async () => {
  const { directory } = await fixture();
  await receiptFile(directory, "old.json", RECEIPT_MAX_AGE_MS + 1000);
  const requests = new MessageReceipts(directory, { now: () => NOW });
  requests.startSweeping();
  try {
    expect(await readdir(directory)).toEqual(["old.json"]);
    await vi.waitFor(async () => expect(await readdir(directory)).toEqual([]));
  } finally {
    requests.stopSweeping();
  }
});
