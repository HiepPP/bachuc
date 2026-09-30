import { starredFirst, type BoardRun } from "./board";

export type RunTree = {
  run: BoardRun;
  children: RunTree[];
  count: number;
  running: boolean;
  starred: boolean;
  needsInput: boolean;
  endedAt: number;
};

export function buildRunTrees(runs: readonly BoardRun[]): RunTree[] {
  const nodes = new Map(
    runs.map((run) => [
      run.agentId,
      {
        run,
        children: [],
        count: 1,
        running: run.status === "running",
        starred: run.starred,
        needsInput: Boolean(run.needsInput),
        endedAt: Date.parse(run.endedAt ?? "") || 0,
      } as RunTree,
    ]),
  );
  const roots: RunTree[] = [];
  for (const node of nodes.values()) {
    const parent = nodes.get(node.run.parentAgentId ?? "");
    const seen = new Set([node.run.agentId]);
    let ancestor = parent;
    while (ancestor && !seen.has(ancestor.run.agentId)) {
      seen.add(ancestor.run.agentId);
      ancestor = nodes.get(ancestor.run.parentAgentId ?? "");
    }
    // Missing parents and malformed cycles must never hide conversations.
    if (parent && !ancestor) parent.children.push(node);
    else roots.push(node);
  }
  function summarize(node: RunTree) {
    for (const child of node.children) {
      summarize(child);
      node.count += child.count;
      node.running ||= child.running;
      node.starred ||= child.starred;
      node.needsInput ||= child.needsInput;
      node.endedAt = Math.max(node.endedAt, child.endedAt);
    }
  }
  roots.forEach(summarize);
  return roots;
}

export function boardColumns(runs: readonly BoardRun[]) {
  const trees = buildRunTrees(runs);
  return {
    running: trees.filter((tree) => tree.running).sort(starredFirst),
    finished: trees
      .filter((tree) => !tree.running)
      .sort((a, b) => starredFirst(a, b) || b.endedAt - a.endedAt),
  };
}
