import type { PaseoAgent, PaseoApi } from "@getpaseo/client";
import type { PluginSettingsState } from "@getpaseo/plugin/server";
import type { CloserSettings, closerSettings } from "../shared/settings";
import { selectIdle } from "./idle";

export const SWEEP_INTERVAL_MS = 60 * 1000;

type AgentsApi = Pick<PaseoApi, "agents">;
type Log = (line: string) => void;

export interface SweepResult {
  checked: number;
  idle: number;
  closed: number;
  failed: number;
}

function describe(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export async function listUnarchived(paseo: AgentsApi): Promise<PaseoAgent[]> {
  const agents: PaseoAgent[] = [];
  const cursors = new Set<string>();
  let cursor: string | undefined;
  for (;;) {
    const page = await paseo.agents.list({
      filter: { includeArchived: false },
      page: { limit: 200, ...(cursor ? { cursor } : {}) },
    });
    for (const entry of page.entries) agents.push(entry.agent);
    if (!page.pageInfo.hasMore) return agents;
    cursor = page.pageInfo.nextCursor ?? undefined;
    if (!cursor || cursors.has(cursor)) throw new Error("Agent list changed during sweep.");
    cursors.add(cursor);
  }
}

export async function sweep(
  paseo: AgentsApi,
  settings: CloserSettings,
  options: {
    now: () => Date;
    log: Log;
    signal: AbortSignal;
    openedAt?: ReadonlyMap<string, number>;
    forget?: (agentId: string) => void;
  },
): Promise<SweepResult> {
  const agents = await listUnarchived(paseo);
  const idle = selectIdle(agents, options.now(), settings, options.openedAt);
  const result: SweepResult = { checked: agents.length, idle: idle.length, closed: 0, failed: 0 };
  for (const agent of idle) {
    if (options.signal.aborted) break;
    try {
      // Close cancels a live turn, so re-check a fresh list just before closing.
      const current = await listUnarchived(paseo);
      if (
        !selectIdle(current, options.now(), settings, options.openedAt).some(
          (entry) => entry.id === agent.id,
        )
      )
        continue;
      if (options.signal.aborted) break;
      await paseo.agents.ref(agent.id).closeRuntime();
      result.closed += 1;
      options.log(`closed ${agent.id}`);
      options.forget?.(agent.id);
    } catch (error) {
      result.failed += 1;
      options.log(`close failed ${agent.id}: ${describe(error)}`);
    }
  }
  return result;
}

export function createCloser(options: {
  readSettings: () => Promise<PluginSettingsState<typeof closerSettings.schema>>;
  log: Log;
  now?: () => number;
}) {
  const now = options.now ?? Date.now;
  const controller = new AbortController();
  let paseo: AgentsApi | undefined;
  let running: Promise<void> | undefined;
  const openedAt = new Map<string, number>();

  const run = (reason: string): Promise<void> | undefined => {
    if (controller.signal.aborted || !paseo || running) return running;
    const api = paseo;
    running = (async () => {
      const state = await options.readSettings();
      if (state.status !== "ready") {
        options.log(`sweep skipped (${reason}): invalid settings: ${state.error}`);
        return;
      }
      if (!state.values.enabled) return;
      const result = await sweep(api, state.values, {
        now: () => new Date(now()),
        log: options.log,
        signal: controller.signal,
        openedAt,
        forget: (agentId) => openedAt.delete(agentId),
      });
      // The timer runs every minute, so only sweeps that found work are logged.
      if (result.idle > 0 || result.failed > 0) {
        options.log(
          `sweep (${reason}): closed ${result.closed} of ${result.idle} idle, ` +
            `checked ${result.checked}, failed ${result.failed}; idleMinutes ${state.values.idleMinutes}`,
        );
      }
    })()
      .catch((error: unknown) => options.log(`sweep failed (${reason}): ${describe(error)}`))
      .finally(() => {
        running = undefined;
      });
    return running;
  };

  return {
    /** Remembers the hook's API for later timer sweeps, then sweeps. */
    fromHook(api: AgentsApi, reason: string) {
      paseo = api;
      return run(reason);
    },
    /** Remembers a hook's API without sweeping, for hooks that are awaited inside an agent operation. */
    remember(api: AgentsApi, openedAgentId: string) {
      if (controller.signal.aborted) return;
      paseo = api;
      openedAt.set(openedAgentId, now());
    },
    /** Timer sweeps are skipped until a hook has supplied an API. */
    fromTimer() {
      return run("timer");
    },
    stop() {
      controller.abort();
      paseo = undefined;
      openedAt.clear();
    },
  };
}
