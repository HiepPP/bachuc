import assert from "node:assert/strict";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import type { PrSnapshot } from "../server/diff";
import { createWatchStore, type PrWatch } from "../server/store";
import { createWatcher, MAX_FAILED_READS, POLL_INTERVAL_MS } from "../server/watcher";

const BASE: PrSnapshot = {
  number: 42,
  url: "https://github.com/acme/app/pull/42",
  state: "OPEN",
  mergeable: "MERGEABLE",
  checks: [{ name: "test", result: "pending" }],
  notes: [],
};

async function withScenario(
  body: (scenario: {
    clock: { now: number };
    reads: number[];
    wakes: Array<{ agentId: string; prompt: string }>;
    setNext: (next: PrSnapshot | Error) => void;
    setBusy: (busy: boolean) => void;
    watcher: ReturnType<typeof createWatcher>;
    store: ReturnType<typeof createWatchStore>;
    file: string;
  }) => Promise<void>,
) {
  const dir = await mkdtemp(join(tmpdir(), "pr-watch-"));
  try {
    const file = join(dir, "watches.json");
    const store = createWatchStore(file);
    const clock = { now: 0 };
    const reads: number[] = [];
    const wakes: Array<{ agentId: string; prompt: string }> = [];
    let next: PrSnapshot | Error = BASE;
    let busy = false;
    const watcher = createWatcher({
      store,
      readPr: async () => {
        reads.push(clock.now);
        if (next instanceof Error) throw next;
        return next;
      },
      viewerLogin: async () => "me",
      isAgentBusy: async () => busy,
      wake: async (agentId, prompt) => {
        wakes.push({ agentId, prompt });
      },
      now: () => clock.now,
      log: () => {},
    });
    const watch: PrWatch = {
      agentId: "agent-1",
      cwd: dir,
      number: 42,
      url: BASE.url,
      createdAt: "2026-10-07T00:00:00.000Z",
      snapshot: BASE,
      failedReads: 0,
    };
    await store.put(watch);
    watcher.schedule(watch);
    await body({
      clock,
      reads,
      wakes,
      setNext: (value) => {
        next = value;
      },
      setBusy: (value) => {
        busy = value;
      },
      watcher,
      store,
      file,
    });
  } finally {
    await rm(dir, { recursive: true, force: true });
  }
}

test("reads each watch at most once per poll interval", async () => {
  await withScenario(async ({ clock, reads, watcher }) => {
    for (const time of [0, 30_000, 60_000, 90_000]) {
      clock.now = time;
      await watcher.tick();
    }
    assert.deepEqual(reads, []);

    for (const time of [POLL_INTERVAL_MS, POLL_INTERVAL_MS + 30_000, 2 * POLL_INTERVAL_MS - 1]) {
      clock.now = time;
      await watcher.tick();
    }
    assert.deepEqual(reads, [POLL_INTERVAL_MS]);

    clock.now = 2 * POLL_INTERVAL_MS;
    await Promise.all([watcher.tick(), watcher.tick()]);
    assert.deepEqual(reads, [POLL_INTERVAL_MS, 2 * POLL_INTERVAL_MS]);
  });
});

test("wakes the agent when a check fails, and holds the wake while it runs", async () => {
  await withScenario(async ({ clock, wakes, setNext, setBusy, watcher }) => {
    setBusy(true);
    setNext({ ...BASE, checks: [{ name: "test", result: "failed" }] });
    clock.now = POLL_INTERVAL_MS;
    await watcher.tick();
    assert.equal(wakes.length, 0);

    await watcher.flushHeld("agent-1");
    assert.equal(wakes.length, 1);
    assert.equal(wakes[0]?.agentId, "agent-1");
    assert.match(wakes[0]?.prompt ?? "", /Checks failed: test\./);
  });
});

test("stops after too many failed reads in a row and says so once", async () => {
  await withScenario(async ({ clock, wakes, setNext, watcher, store }) => {
    setNext(new Error("gh: not logged in"));
    for (let index = 1; index <= MAX_FAILED_READS; index += 1) {
      clock.now = index * POLL_INTERVAL_MS;
      await watcher.tick();
    }
    assert.equal(store.list().length, 0);
    assert.equal(wakes.length, 1);
    assert.match(
      wakes[0]?.prompt ?? "",
      /after 8 failed reads in a row. Last error: gh: not logged in/,
    );
  });
});

test("a merged PR ends its watch with one wake", async () => {
  await withScenario(async ({ clock, wakes, setNext, watcher, store }) => {
    setNext({ ...BASE, state: "MERGED" });
    clock.now = POLL_INTERVAL_MS;
    await watcher.tick();
    assert.equal(store.list().length, 0);
    assert.equal(wakes.length, 1);
    assert.match(wakes[0]?.prompt ?? "", /is merged/);
  });
});

test("watches survive a reload of the store", async () => {
  await withScenario(async ({ file }) => {
    const reloaded = createWatchStore(file);
    await reloaded.load();
    assert.deepEqual(
      reloaded.list().map((watch) => [watch.agentId, watch.number]),
      [["agent-1", 42]],
    );
    const removed = await reloaded.remove("agent-1");
    assert.equal(removed.length, 1);
    assert.equal(reloaded.list().length, 0);
  });
});
