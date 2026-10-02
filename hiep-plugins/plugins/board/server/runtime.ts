import type { PaseoAgent, PaseoApi } from "@getpaseo/client";
import { getParentAgentIdFromLabels } from "./agent-labels";

type Agent = Pick<PaseoAgent, "id" | "status" | "pendingPermissions" | "labels">;
type AgentsApi = Pick<PaseoApi, "agents">;

const isActive = (agent: Agent) => agent.status === "running" || agent.status === "initializing";
// `idle` and `error` hold a runtime without a turn; `closed` holds nothing.
const atRest = (agent: Agent) =>
  (agent.status === "idle" || agent.status === "error") && agent.pendingPermissions.length === 0;

/**
 * The removed thread and all its subagents, split into runtimes to release and runtimes to keep.
 * `close` lists subagents before their parent, so the removed thread goes last. Closing cancels a
 * turn, so a thread that is working or waiting for permission is kept. So is every ancestor of a
 * working thread: it is still waiting for that subagent's result.
 */
export function selectRuntimes(agents: readonly Agent[], rootId: string) {
  const byId = new Map(agents.map((agent) => [agent.id, agent]));
  const parentOf = (agent: Agent) => getParentAgentIdFromLabels(agent.labels);
  const waiting = new Set<string>();
  for (const agent of agents) {
    if (!isActive(agent)) continue;
    let parentId = parentOf(agent);
    while (parentId && !waiting.has(parentId)) {
      waiting.add(parentId);
      const parent = byId.get(parentId);
      parentId = parent ? parentOf(parent) : null;
    }
  }
  const children = new Map<string, string[]>();
  for (const agent of agents) {
    const parentId = parentOf(agent);
    if (parentId) children.set(parentId, [...(children.get(parentId) ?? []), agent.id]);
  }
  // Subagents first, at every depth, and the removed thread last.
  const family: string[] = [];
  const seen = new Set<string>();
  const visit = (id: string) => {
    if (seen.has(id)) return;
    seen.add(id);
    for (const childId of children.get(id) ?? []) visit(childId);
    family.push(id);
  };
  visit(rootId);
  const close: string[] = [];
  let kept = 0;
  for (const id of family) {
    const agent = byId.get(id);
    if (!agent || agent.status === "closed") continue;
    if (atRest(agent) && !waiting.has(id)) close.push(id);
    else kept += 1;
  }
  return { close, kept };
}

async function listUnarchived(paseo: AgentsApi): Promise<PaseoAgent[]> {
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
    if (!cursor || cursors.has(cursor)) throw new Error("Agent list changed. Refresh to retry.");
    cursors.add(cursor);
  }
}

/** Releases the processes of a removed thread and its subagents. One failed close does not stop the rest. */
export async function closeRuntimes(
  paseo: AgentsApi,
  rootId: string,
  log: (line: string) => void = () => {},
) {
  const selected = selectRuntimes(await listUnarchived(paseo), rootId);
  const result = { closed: 0, kept: selected.kept, failed: 0 };
  for (const id of selected.close) {
    try {
      const handle = paseo.agents.ref(id);
      // The list is seconds old by the last close; never cancel a turn that started since.
      const fresh = await handle.refresh();
      if (!fresh || !atRest(fresh.agent)) {
        result.kept += 1;
        continue;
      }
      await handle.closeRuntime();
      result.closed += 1;
    } catch (error) {
      result.failed += 1;
      log(`[board] could not close runtime ${id}: ${String(error)}`);
    }
  }
  return result;
}
