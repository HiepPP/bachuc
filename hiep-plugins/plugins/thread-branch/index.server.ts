import type { PluginServerContext } from "@getpaseo/plugin/server";
import { homedir } from "node:os";
import path from "node:path";
import { createBranchReader } from "./server/git";
import { createPrSearch, listWorkspaceDirs } from "./server/pr-search";
import {
  createTurnDiffTracker,
  defaultGit,
  dropTrees,
  keepTrees,
  readFileDiff,
  readFileImage,
} from "./server/turn-diff";
import { createTurnJournal } from "./server/turn-journal";
import { createStartStore } from "./server/turn-starts";
import { getBranchRpc } from "./shared/branch";
import { searchPrsRpc } from "./shared/pr-search";
import {
  fileDiffRpc,
  fileImageRpc,
  TURN_DIFF_KIND,
  TURN_DIFF_VERSION,
  turnHistoryRpc,
  type TurnDiff,
} from "./shared/turn-diff";

export default function contribute(server: PluginServerContext) {
  const reader = createBranchReader();
  const prs = createPrSearch();
  const log = (line: string) => console.warn(`[thread-branch] ${line}`);
  const git = defaultGit;
  const home = process.env.PASEO_HOME || path.join(homedir(), ".paseo");
  const journal = createTurnJournal(path.join(home, "plugin-data/thread-branch/turn-diffs.jsonl"));
  const diffs = createTurnDiffTracker(
    git,
    createStartStore(path.join(home, "plugin-data/thread-branch/turn-starts")),
  );
  async function record(agentId: string, rowId: string, diff: TurnDiff) {
    const endedAt = new Date();
    // The row id already carries the end time, which keeps journal keys and refs unique.
    const key = rowId.replace(/^turn-diff:/, "");
    if (diff.source) await keepTrees(git, diff.source.root, key, diff.source.from, diff.source.to);
    const expired = await journal.add({ key, agentId, endedAt: endedAt.toISOString(), diff });
    for (const old of expired) {
      if (old.diff.source)
        await dropTrees(git, old.diff.source.root, old.key).catch(() => undefined);
    }
  }
  server.handle(getBranchRpc, ({ cwd, force, fetch }) =>
    reader.get(cwd, force === true || fetch === true, fetch === true),
  );
  server.handle(searchPrsRpc, ({ query }, { paseo }) =>
    prs.search(query, () => listWorkspaceDirs(paseo)),
  );
  server.handle(fileDiffRpc, (input) => readFileDiff(git, input));
  server.handle(turnHistoryRpc, async ({ agentId }) => ({ turns: await journal.list(agentId) }));
  server.handle(fileImageRpc, (input) => readFileImage(input));
  const started = server.on("agent.turn_started", ({ agent, turnId }) => {
    void diffs.started(agent, turnId);
  });
  const ended = server.on("agent.turn_ended", ({ agent, turnId }, { paseo }) => {
    // Git snapshots run detached so the lifecycle hook returns immediately.
    void diffs
      .ended(agent, turnId)
      .then(async (row) => {
        if (!row) return;
        await paseo.agents
          .ref(agent.id)
          .timeline.append({
            type: "plugin",
            id: row.rowId,
            kind: TURN_DIFF_KIND,
            version: TURN_DIFF_VERSION,
            data: row.diff,
          })
          .catch((error: unknown) => log(`could not add turn diff row: ${String(error)}`));
        await record(agent.id, row.rowId, row.diff);
      })
      .catch((error) => log(`turn diff failed: ${error instanceof Error ? error.message : error}`));
  });
  return () => {
    started();
    ended();
  };
}
