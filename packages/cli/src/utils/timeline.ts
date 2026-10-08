import type {
  DaemonClient,
  FetchAgentTimelinePayload,
} from "@getpaseo/client/internal/daemon-client";
import type { AgentTimelineItem } from "@getpaseo/protocol/agent-types";

export const LIVE_HISTORY_FETCH_TIMEOUT_MS = 2_000;

// A long session's whole projected timeline is 15k+ entries and ~17MB; the daemon serializes a
// response in one event-loop task, so one unbounded fetch stalled it for ~90ms. Pages keep each
// response small; the CLI still prints the full history.
const TIMELINE_PAGE_SIZE = 200;
// A reset between pages (rewind, epoch change) restarts the walk from the tail.
const MAX_TIMELINE_WALKS = 3;

interface FetchProjectedTimelineItemsInput {
  client: Pick<DaemonClient, "fetchAgentTimeline">;
  agentId: string;
  timeoutMs?: number;
  pageSize?: number;
}

export async function fetchProjectedTimelineItems(
  input: FetchProjectedTimelineItemsInput,
): Promise<AgentTimelineItem[]> {
  for (let walk = 1; ; walk++) {
    const pages = await walkProjectedTimeline(input);
    if (pages) {
      return pages.flatMap((page) => page.entries.map((entry) => entry.item));
    }
    if (walk >= MAX_TIMELINE_WALKS) {
      throw new Error(`Timeline for agent ${input.agentId} kept changing while it was read`);
    }
  }
}

/** Reads the tail page, then older pages until none remain. Returns null after a reset. */
async function walkProjectedTimeline(
  input: FetchProjectedTimelineItemsInput,
): Promise<FetchAgentTimelinePayload[] | null> {
  const limit = input.pageSize ?? TIMELINE_PAGE_SIZE;
  let page = await input.client.fetchAgentTimeline(input.agentId, {
    direction: "tail",
    limit,
    projection: "projected",
    timeout: input.timeoutMs,
  });
  const pages = [page];
  while (page.hasOlder && page.startCursor && page.entries.length > 0) {
    const epoch = page.epoch;
    page = await input.client.fetchAgentTimeline(input.agentId, {
      direction: "before",
      cursor: page.startCursor,
      limit,
      projection: "projected",
      timeout: input.timeoutMs,
    });
    if (page.reset || page.staleCursor || page.epoch !== epoch) {
      return null;
    }
    pages.unshift(page);
  }
  return pages;
}
