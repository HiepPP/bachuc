import type {
  PluginButton,
  PluginButtonRegistration,
  PluginClientContext,
} from "@getpaseo/plugin/client";
import { copyText } from "@getpaseo/plugin/client/react-native";
import { getBranchRpc, type BranchInfo } from "../shared/branch";
import { describeBranchPill, describeChangesPill, describePrPill } from "./buttons";
import { openUrl } from "./open";

export const POLL_MS = 15_000;

interface Tracked {
  workspaceId: string;
  cwd: string;
}

type Client = Pick<PluginClientContext, "rpc" | "addComposerPill"> & {
  paseo: Pick<PluginClientContext["paseo"], "agents">;
};

export interface PillDeps {
  open?: (url: string) => Promise<void>;
  copy?: (text: string) => Promise<void>;
  intervalMs?: number;
}

function signature(info: BranchInfo) {
  return JSON.stringify([
    info.repo,
    info.branch,
    info.detached,
    info.sha,
    info.dirty,
    info.changes,
    info.upstream,
    info.ahead,
    info.behind,
    info.pr,
    info.remoteUrl,
  ]);
}

export function installBranchPills(client: Client, deps: PillDeps = {}) {
  const open = deps.open ?? openUrl;
  const copy = deps.copy ?? copyText;
  const intervalMs = deps.intervalMs ?? POLL_MS;
  const agents = new Map<string, Tracked>();
  const pills = new Map<
    string,
    {
      branch: PluginButtonRegistration;
      pr: PluginButtonRegistration;
      changes: PluginButtonRegistration;
      signature: string;
    }
  >();
  const infoByCwd = new Map<string, BranchInfo>();
  let stopped = false;
  let timer: ReturnType<typeof setTimeout> | undefined;

  function actionsFor(cwd: string) {
    return {
      async openPr() {
        const pr = infoByCwd.get(cwd)?.pr;
        if (!pr) throw new Error("No pull request for this branch.");
        await open(pr.url);
      },
      async openRepo() {
        const url = infoByCwd.get(cwd)?.remoteUrl;
        if (!url) throw new Error("No remote repository for this directory.");
        await open(url);
      },
      async copy() {
        const branch = infoByCwd.get(cwd)?.branch;
        if (!branch) throw new Error("No branch name to copy.");
        await copy(branch);
      },
      async refresh() {
        await refreshCwd(cwd, true);
      },
      async fetch() {
        await refreshCwd(cwd, true, true);
      },
    };
  }

  function apply(cwd: string, info: BranchInfo) {
    infoByCwd.set(cwd, info);
    for (const [agentId, tracked] of agents) {
      if (tracked.cwd !== cwd) continue;
      const next = signature(info);
      const actions = actionsFor(cwd);
      const branch = describeBranchPill(info, actions);
      const pr = describePrPill(info, actions.openPr);
      const changes = describeChangesPill(info, actions.refresh);
      const existing = pills.get(agentId);
      if (existing) {
        // Updating behavior closes an open menu; skip no-op updates from the poll loop.
        if (existing.signature === next) continue;
        try {
          existing.branch.update(branch);
          existing.pr.update(pr);
          existing.changes.update(changes);
          existing.signature = next;
        } catch (error) {
          console.warn("[thread-branch] Failed to update pills", agentId, error);
        }
        continue;
      }
      const created: PluginButtonRegistration[] = [];
      const add = (id: string, button: PluginButton) => {
        const registration = client.addComposerPill({
          id,
          workspaceId: tracked.workspaceId,
          agentId,
          button,
        });
        created.push(registration);
        return registration;
      };
      try {
        pills.set(agentId, {
          branch: add("thread-branch", branch),
          pr: add("thread-branch-pr", pr),
          changes: add("thread-branch-changes", changes),
          signature: next,
        });
      } catch (error) {
        // A rejected pill must not leave half a set behind (retries would hit duplicate ids)
        // or stop the loop, which would hide pills of every later thread in this directory.
        for (const registration of created) registration.remove();
        console.warn("[thread-branch] Failed to add pills", agentId, error);
      }
    }
  }

  async function refreshCwd(cwd: string, force: boolean, fetch = false) {
    const input = fetch ? { cwd, fetch: true } : force ? { cwd, force: true } : { cwd };
    const info = await client.rpc(getBranchRpc, input);
    if (stopped) return;
    apply(cwd, info);
  }

  async function refreshAll() {
    if (!observation) await bootstrap().catch(() => {});
    const cwds = new Set<string>();
    for (const tracked of agents.values()) cwds.add(tracked.cwd);
    await Promise.all(
      [...cwds].map((cwd) =>
        refreshCwd(cwd, false).catch(() => {
          // A failed probe keeps the last known pill state; the next tick retries.
        }),
      ),
    );
  }

  function schedule() {
    if (stopped) return;
    clearTimeout(timer);
    timer = setTimeout(() => {
      void refreshAll().finally(schedule);
    }, intervalMs);
  }

  function track(agent: { id: string; cwd: string; workspaceId?: string | null }) {
    if (!agent.workspaceId || !agent.cwd) return;
    const previous = agents.get(agent.id);
    agents.set(agent.id, { workspaceId: agent.workspaceId, cwd: agent.cwd });
    if (previous && previous.cwd !== agent.cwd) untrack(agent.id, true);
    const known = infoByCwd.get(agent.cwd);
    if (known) apply(agent.cwd, known);
    else if (!previous || previous.cwd !== agent.cwd)
      void refreshCwd(agent.cwd, false).catch(() => {});
  }

  function untrack(agentId: string, keepAgent = false) {
    const existing = pills.get(agentId);
    existing?.branch.remove();
    existing?.pr.remove();
    existing?.changes.remove();
    pills.delete(agentId);
    if (!keepAgent) agents.delete(agentId);
  }

  const unsubscribe = client.paseo.agents.subscribe((update) => {
    if (stopped) return;
    if (update.kind === "upsert") track(update.agent);
    else if (update.kind === "remove") untrack(update.agentId);
  });

  const filter = { includeArchived: false };
  let observation: { release(): Promise<void> } | undefined;

  async function listRest(cursor: string | null | undefined) {
    const seen = new Set<string>();
    while (cursor && !seen.has(cursor)) {
      seen.add(cursor);
      const page = await client.paseo.agents.list({ filter, page: { limit: 200, cursor } });
      if (stopped) return;
      for (const { agent } of page.entries) track(agent);
      if (!page.pageInfo.hasMore) return;
      cursor = page.pageInfo.nextCursor;
    }
  }

  async function bootstrap() {
    // Plain list() creates no daemon demand, so agents.subscribe() would never fire; observing the
    // directory keeps new agents flowing and replays a snapshot after a reconnect.
    const first = await client.paseo.agents.list({ filter, page: { limit: 200 }, subscribe: {} });
    if (stopped) return void first.subscription.release().catch(() => {});
    observation = first.subscription;
    first.subscription.subscribe({
      snapshot({ entries, pageInfo }) {
        if (stopped) return;
        for (const { agent } of entries) track(agent);
        if (pageInfo.hasMore) return void listRest(pageInfo.nextCursor).catch(() => {});
        const live = new Set(entries.map(({ agent }) => agent.id));
        for (const agentId of agents.keys()) if (!live.has(agentId)) untrack(agentId);
      },
      update() {
        // Directory updates reach the agents.subscribe() handler.
      },
    });
  }

  void bootstrap()
    .catch(() => {
      // Each poll tick retries until the directory observation starts.
    })
    .finally(schedule);

  return () => {
    stopped = true;
    clearTimeout(timer);
    unsubscribe();
    void observation?.release().catch(() => {});
    for (const { branch, pr, changes } of pills.values()) {
      branch.remove();
      pr.remove();
      changes.remove();
    }
    pills.clear();
    agents.clear();
    infoByCwd.clear();
  };
}
