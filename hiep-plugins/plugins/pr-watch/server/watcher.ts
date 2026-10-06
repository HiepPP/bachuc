import { diffSnapshots, formatWakePrompt, type PrSnapshot } from "./diff";
import type { PrWatch, PrWatchStore } from "./store";

export const POLL_INTERVAL_MS = 2 * 60_000;
export const MAX_FAILED_READS = 8;

export interface WatcherDeps {
  store: PrWatchStore;
  readPr: (cwd: string, number: number) => Promise<PrSnapshot>;
  viewerLogin: (cwd: string) => Promise<string | null>;
  isAgentBusy: (agentId: string) => Promise<boolean>;
  wake: (agentId: string, prompt: string) => Promise<void>;
  now: () => number;
  log: (line: string) => void;
}

export interface Watcher {
  /** Polls every watch whose interval has passed. Safe to call as often as you like. */
  tick(): Promise<void>;
  /** The first poll of a new watch waits one interval; its read was the baseline. */
  schedule(watch: Pick<PrWatch, "agentId" | "number">): void;
  /** Sends the wakes held while the agent was running. */
  flushHeld(agentId: string): Promise<void>;
  forget(agentId: string): void;
}

const key = (watch: Pick<PrWatch, "agentId" | "number">) => `${watch.agentId}#${watch.number}`;

export function createWatcher(deps: WatcherDeps): Watcher {
  const nextPollAt = new Map<string, number>();
  const inFlight = new Set<string>();
  const heldWakes = new Map<string, string[]>();

  // A trigger never steers a running turn; it waits for the turn to end (Q-005).
  async function deliver(agentId: string, prompt: string): Promise<void> {
    if (await deps.isAgentBusy(agentId).catch(() => false)) {
      heldWakes.set(agentId, [...(heldWakes.get(agentId) ?? []), prompt]);
      return;
    }
    await deps.wake(agentId, prompt);
  }

  async function poll(watch: PrWatch): Promise<void> {
    let snapshot: PrSnapshot;
    try {
      snapshot = await deps.readPr(watch.cwd, watch.number);
    } catch (error) {
      const failedReads = watch.failedReads + 1;
      if (failedReads < MAX_FAILED_READS) {
        await deps.store.put({ ...watch, failedReads });
        return;
      }
      await deps.store.remove(watch.agentId, watch.number);
      const reason = error instanceof Error ? error.message : String(error);
      await deliver(
        watch.agentId,
        `<pr-watch>\nStopped watching PR #${watch.number} (${watch.url}) after ${MAX_FAILED_READS} failed reads in a row. Last error: ${reason}\n</pr-watch>`,
      );
      return;
    }

    const triggers = diffSnapshots(watch.snapshot, snapshot, await deps.viewerLogin(watch.cwd));
    if (snapshot.state === "OPEN") {
      await deps.store.put({ ...watch, snapshot, failedReads: 0 });
    } else {
      await deps.store.remove(watch.agentId, watch.number);
    }
    if (triggers.length > 0) await deliver(watch.agentId, formatWakePrompt(snapshot, triggers));
  }

  return {
    async tick() {
      for (const watch of deps.store.list()) {
        const watchKey = key(watch);
        const dueAt = nextPollAt.get(watchKey) ?? 0;
        if (inFlight.has(watchKey) || deps.now() < dueAt) continue;
        nextPollAt.set(watchKey, deps.now() + POLL_INTERVAL_MS);
        inFlight.add(watchKey);
        try {
          await poll(watch);
        } catch (error) {
          deps.log(`poll of PR #${watch.number} failed: ${String(error)}`);
        } finally {
          inFlight.delete(watchKey);
        }
      }
    },
    schedule(watch) {
      nextPollAt.set(key(watch), deps.now() + POLL_INTERVAL_MS);
    },
    async flushHeld(agentId) {
      const prompts = heldWakes.get(agentId);
      if (!prompts) return;
      heldWakes.delete(agentId);
      await deps.wake(agentId, prompts.join("\n\n"));
    },
    forget(agentId) {
      heldWakes.delete(agentId);
    },
  };
}
