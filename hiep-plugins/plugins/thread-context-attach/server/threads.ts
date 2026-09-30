import type { PaseoAgent, PaseoAgentHandle, PaseoApi } from "@getpaseo/client";
import type { PluginAttachmentItem, RpcInput } from "@getpaseo/plugin";
import { pathToFileURL } from "node:url";
import type { listThreadsRpc, Thread, ThreadSnapshot } from "../shared/threads";
import { buildSnapshot, lastAssistantText, matchesQuery, orderThreads } from "./snapshot";

const TIMELINE_PAGE_SIZE = 200;
const MAX_TIMELINE_PAGES = 5;
const SEARCH_LIMIT = 12;
const CACHE_LIMIT = 100;

function latest(...times: (string | null | undefined)[]): string {
  let best = "";
  for (const time of times) if (time && (!best || Date.parse(time) > Date.parse(best))) best = time;
  return best;
}

export function toThread(agent: PaseoAgent): Thread {
  return {
    id: agent.id,
    title: agent.title?.trim() || "Untitled thread",
    provider: agent.provider,
    cwd: agent.cwd,
    workspaceId: agent.workspaceId ?? null,
    status: agent.status,
    // The daemon snapshot has no lastActivityAt; use the newest agent timestamp instead.
    lastActivityAt: latest(agent.updatedAt, agent.lastUserMessageAt, agent.attentionTimestamp),
  };
}

export async function listThreads(
  paseo: PaseoApi,
  input: RpcInput<typeof listThreadsRpc> = {},
): Promise<Thread[]> {
  const result = await paseo.agents.list({
    page: { limit: 200 },
    sort: [{ key: "updated_at", direction: "desc" }],
  });
  const threads = result.entries
    .map((entry) => entry.agent)
    .filter((agent) => !agent.archivedAt)
    .map(toThread);
  return orderThreads(threads, input)
    .filter((thread) => matchesQuery(thread, input.query ?? ""))
    .slice(0, input.limit ?? 50);
}

async function readLastReply(handle: PaseoAgentHandle): Promise<string | null> {
  let cursor: { epoch: string; seq: number } | undefined;
  for (let page = 0; page < MAX_TIMELINE_PAGES; page++) {
    const result = await handle.timeline.refetch(
      cursor
        ? { direction: "before", cursor, limit: TIMELINE_PAGE_SIZE }
        : { direction: "tail", limit: TIMELINE_PAGE_SIZE },
    );
    if (result.error) throw new Error(result.error);
    const text = lastAssistantText(result.entries);
    if (text !== null) return text;
    if (!result.hasOlder || !result.startCursor) return null;
    cursor = result.startCursor;
  }
  return null;
}

export async function getThreadSnapshot(paseo: PaseoApi, agentId: string): Promise<ThreadSnapshot> {
  const handle = paseo.agents.ref(agentId);
  const refreshed = await handle.refresh();
  if (!refreshed) throw new Error("Thread is unavailable on this host.");
  return buildSnapshot(toThread(refreshed.agent), await readLastReply(handle));
}

// Search runs per keystroke; reuse snapshots until the thread's activity changes.
const snapshotCache = new Map<string, ThreadSnapshot>();

async function cachedSnapshot(paseo: PaseoApi, thread: Thread): Promise<ThreadSnapshot> {
  const key = `${thread.id}@${thread.lastActivityAt}`;
  const cached = snapshotCache.get(key);
  if (cached) return cached;
  const snapshot = buildSnapshot(thread, await readLastReply(paseo.agents.ref(thread.id)));
  if (snapshotCache.size >= CACHE_LIMIT) snapshotCache.clear();
  snapshotCache.set(key, snapshot);
  return snapshot;
}

export async function searchThreadAttachments(
  query: string,
  paseo: PaseoApi,
): Promise<{ items: PluginAttachmentItem[] }> {
  // The host passes only { query } to attachment searches, so the composer's own
  // agent and workspace are unknown here; results are ordered newest first.
  const threads = await listThreads(paseo, { query, limit: SEARCH_LIMIT });
  const items = await Promise.all(
    threads.map(async (thread): Promise<PluginAttachmentItem> => {
      const base = {
        id: thread.id,
        identifier: thread.id.slice(0, 8),
        title: thread.title,
        resourceType: "Paseo thread",
        url: pathToFileURL(thread.cwd || "/").href,
      };
      try {
        const snapshot = await cachedSnapshot(paseo, thread);
        return {
          ...base,
          subtitle: `${thread.provider} · ${thread.cwd}${snapshot.hasReply ? "" : " · no reply yet"}`,
          text: snapshot.text,
        };
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        return {
          ...base,
          subtitle: `${thread.provider} · ${thread.cwd} · timeline unavailable`,
          text: buildSnapshot(thread, null, `Could not read this thread's timeline: ${message}`)
            .text,
        };
      }
    }),
  );
  return { items };
}
