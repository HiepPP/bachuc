import type { PaseoApi } from "@getpaseo/client";
import type { PluginServerContext } from "@getpaseo/plugin/server";
import { createExporter, exportDir, exportThread } from "./server/export";
import { createIndexer, createRunner, findQmd } from "./server/qmd";
import { getThreadSnapshot, listThreads, searchThreadAttachments } from "./server/threads";
import { getThreadSnapshotRpc, listThreadsRpc, searchThreadsRpc } from "./shared/threads";

export default function contribute(server: PluginServerContext) {
  const dir = exportDir();
  const log = (line: string) => console.info(`[thread-context-attach] ${line}`);
  const exporter = createExporter({
    exportOne: (paseo, agentId) => exportThread(paseo, agentId, dir),
    log,
  });
  const qmd = findQmd();
  const indexer = qmd ? createIndexer({ dir, run: createRunner(qmd), log }) : null;
  if (indexer) void indexer.start();
  else log("qmd not found; threads are exported but not indexed");
  exporter.onExported(() => indexer?.notify());
  // The plugin gets a Paseo API only inside hooks and RPCs, so backfill starts on first use.
  let backfillStarted = false;
  const backfill = (paseo: PaseoApi) => {
    if (backfillStarted) return;
    backfillStarted = true;
    void paseo.agents
      .list({ filter: { includeArchived: false }, page: { limit: 200 } })
      .then((page) =>
        exporter.backfill(
          paseo,
          page.entries.map((entry) => entry.agent.id),
        ),
      )
      .catch((error) => log(`backfill failed: ${error instanceof Error ? error.message : error}`));
  };
  const removeTurnEnded = server.on("agent.turn_ended", ({ agent }, { paseo }) => {
    backfill(paseo);
    exporter.schedule(paseo, agent.id);
  });
  server.handle(listThreadsRpc, async (input, { paseo }) => {
    backfill(paseo);
    return { threads: await listThreads(paseo, input) };
  });
  server.handle(getThreadSnapshotRpc, ({ agentId }, { paseo }) =>
    getThreadSnapshot(paseo, agentId),
  );
  server.handle(searchThreadsRpc, ({ query }, { paseo }) => {
    backfill(paseo);
    return searchThreadAttachments(query, paseo);
  });
  return () => {
    removeTurnEnded();
    exporter.stop();
    indexer?.stop();
  };
}
