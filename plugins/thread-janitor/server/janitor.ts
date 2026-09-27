import type { PaseoAgent, PaseoApi, PaseoWorkspace } from "@getpaseo/client";
import type { PluginSettingsState } from "@getpaseo/plugin/server";
import type { JanitorSettings, janitorSettings } from "../shared/settings";
import { liveWorkspaces, selectStale, selectStaleWorkspaces } from "./stale";

export const SWEEP_THROTTLE_MS = 10 * 60 * 1000;
export const SWEEP_INTERVAL_MS = 30 * 60 * 1000;

type AgentsApi = Pick<PaseoApi, "agents" | "workspaces" | "terminals">;
type Log = (line: string) => void;

export interface SweepResult {
  checked: number;
  stale: number;
  archived: number;
  failed: number;
  workspacesStale: number;
  workspacesArchived: number;
  workspacesFailed: number;
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

export async function listWorkspaces(paseo: AgentsApi): Promise<PaseoWorkspace[]> {
  const workspaces: PaseoWorkspace[] = [];
  const cursors = new Set<string>();
  let cursor: string | undefined;
  for (;;) {
    const page = await paseo.workspaces.list({
      page: { limit: 200, ...(cursor ? { cursor } : {}) },
    });
    workspaces.push(...page.entries);
    if (!page.pageInfo.hasMore) return workspaces;
    cursor = page.pageInfo.nextCursor ?? undefined;
    if (!cursor || cursors.has(cursor)) throw new Error("Workspace list changed during sweep.");
    cursors.add(cursor);
  }
}

export async function sweep(
  paseo: AgentsApi,
  settings: JanitorSettings,
  options: { now: () => Date; log: Log; signal: AbortSignal },
): Promise<SweepResult> {
  const agents = await listUnarchived(paseo);
  const stale = selectStale(agents, options.now(), settings);
  const result: SweepResult = {
    checked: agents.length,
    stale: stale.length,
    archived: 0,
    failed: 0,
    workspacesStale: 0,
    workspacesArchived: 0,
    workspacesFailed: 0,
  };
  for (const agent of stale) {
    if (options.signal.aborted) break;
    try {
      const ref = paseo.agents.ref(agent.id);
      // Archive cancels a live turn, so re-check a fresh snapshot just before archiving.
      const current = await listUnarchived(paseo);
      const fresh = await ref.refresh();
      if (!fresh) continue;
      const candidates = current.map((entry) => (entry.id === agent.id ? fresh.agent : entry));
      if (!selectStale(candidates, options.now(), settings).some((entry) => entry.id === agent.id))
        continue;
      if (options.signal.aborted) break;
      await ref.archive();
      result.archived += 1;
      options.log(`archived ${agent.id} titleLength=${agent.title?.length ?? 0}`);
    } catch (error) {
      result.failed += 1;
      options.log(`archive failed ${agent.id}: ${describe(error)}`);
    }
  }
  if (options.signal.aborted) return result;

  // Archiving a workspace also archives its agents and kills its terminals.
  const live = liveWorkspaces(await listUnarchived(paseo));
  const staleWorkspaces = selectStaleWorkspaces(
    await listWorkspaces(paseo),
    live,
    options.now(),
    settings,
  );
  result.workspacesStale = staleWorkspaces.length;
  for (const workspace of staleWorkspaces) {
    if (options.signal.aborted) break;
    try {
      const ref = paseo.workspaces.ref(workspace.id);
      const fresh = await ref.refresh();
      if (!fresh || selectStaleWorkspaces([fresh], live, options.now(), settings).length === 0) {
        continue;
      }
      const terminals = await paseo.terminals.list({ workspaceId: workspace.id });
      if (terminals.entries.length > 0) continue;
      const archived = await ref.archive();
      if (archived.error) throw new Error(archived.error);
      result.workspacesArchived += 1;
      options.log(`archived workspace ${workspace.id}`);
    } catch (error) {
      result.workspacesFailed += 1;
      options.log(`workspace archive failed ${workspace.id}: ${describe(error)}`);
    }
  }
  return result;
}

export function createJanitor(options: {
  readSettings: () => Promise<PluginSettingsState<typeof janitorSettings.schema>>;
  log: Log;
  now?: () => number;
}) {
  const now = options.now ?? Date.now;
  const controller = new AbortController();
  let paseo: AgentsApi | undefined;
  let lastSweepAt = -Infinity;
  let running: Promise<void> | undefined;

  const run = (reason: string): Promise<void> | undefined => {
    if (controller.signal.aborted || !paseo || running) return running;
    const startedAt = now();
    if (startedAt - lastSweepAt < SWEEP_THROTTLE_MS) return undefined;
    lastSweepAt = startedAt;
    const api = paseo;
    running = (async () => {
      const state = await options.readSettings();
      if (state.status !== "ready") {
        options.log(`sweep skipped (${reason}): invalid settings: ${state.error}`);
        return;
      }
      if (!state.values.enabled) {
        options.log(`sweep skipped (${reason}): disabled`);
        return;
      }
      const result = await sweep(api, state.values, {
        now: () => new Date(now()),
        log: options.log,
        signal: controller.signal,
      });
      options.log(
        `sweep (${reason}): archived ${result.archived} of ${result.stale} stale, ` +
          `checked ${result.checked}, failed ${result.failed}; ` +
          `archived ${result.workspacesArchived} of ${result.workspacesStale} stale workspaces, ` +
          `failed ${result.workspacesFailed}; idleHours ${state.values.idleHours}; keepRecent ${state.values.keepRecent}`,
      );
    })()
      .catch((error: unknown) => options.log(`sweep failed (${reason}): ${describe(error)}`))
      .finally(() => {
        running = undefined;
      });
    return running;
  };

  return {
    /** Remembers the hook's API for later timer sweeps, then sweeps if not throttled. */
    fromHook(api: AgentsApi, reason: string) {
      paseo = api;
      return run(reason);
    },
    /** Timer sweeps are skipped until a hook has supplied an API. */
    fromTimer() {
      return run("timer");
    },
    stop() {
      controller.abort();
      paseo = undefined;
    },
  };
}
