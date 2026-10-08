import type {
  FetchAgentTimelineOptions,
  FetchAgentTimelinePayload,
} from "@getpaseo/client/internal/daemon-client";
import { describe, expect, it } from "vitest";

import { fetchProjectedTimelineItems } from "./timeline.js";

type Entry = FetchAgentTimelinePayload["entries"][number];

function entry(seq: number): Entry {
  return {
    provider: "claude",
    item: { type: "assistant_message", text: `message ${seq}` },
    timestamp: "2026-10-09T00:00:00.000Z",
    seqStart: seq,
    seqEnd: seq,
    sourceSeqRanges: [{ startSeq: seq, endSeq: seq }],
    collapsed: [],
  };
}

/** Serves pages the way the daemon's timeline store selects them. */
class FakeTimelineClient {
  readonly requests: FetchAgentTimelineOptions[] = [];
  epoch = "epoch-1";
  onRequest?: (client: FakeTimelineClient) => void;

  constructor(readonly entries: Entry[]) {}

  async fetchAgentTimeline(
    agentId: string,
    options: FetchAgentTimelineOptions = {},
  ): Promise<FetchAgentTimelinePayload> {
    this.requests.push(options);
    this.onRequest?.(this);
    const staleCursor = options.cursor !== undefined && options.cursor.epoch !== this.epoch;
    const limit = options.limit ?? 200;
    const eligible =
      options.direction === "before" && !staleCursor
        ? this.entries.filter((candidate) => candidate.seqStart < (options.cursor?.seq ?? 0))
        : this.entries;
    const selected = limit === 0 ? eligible : eligible.slice(Math.max(0, eligible.length - limit));
    const first = selected[0];
    return {
      requestId: "request",
      agentId,
      agent: null,
      direction: options.direction ?? "tail",
      projection: "projected",
      epoch: this.epoch,
      reset: staleCursor,
      staleCursor,
      gap: false,
      window: { minSeq: 1, maxSeq: this.entries.length, nextSeq: this.entries.length + 1 },
      startCursor: first ? { epoch: this.epoch, seq: first.seqStart } : null,
      endCursor: null,
      hasOlder: selected.length < eligible.length,
      hasNewer: false,
      entries: selected,
      error: null,
    };
  }
}

const ENTRIES = Array.from({ length: 7 }, (_, index) => entry(index + 1));

describe("fetchProjectedTimelineItems", () => {
  it("returns the same items as one unbounded fetch, oldest first", async () => {
    const client = new FakeTimelineClient(ENTRIES);

    const items = await fetchProjectedTimelineItems({ client, agentId: "agent", pageSize: 3 });

    expect(items).toEqual(ENTRIES.map((candidate) => candidate.item));
    expect(client.requests.map(({ direction, limit }) => ({ direction, limit }))).toEqual([
      { direction: "tail", limit: 3 },
      { direction: "before", limit: 3 },
      { direction: "before", limit: 3 },
    ]);
  });

  it("restarts from the tail when the timeline resets between pages", async () => {
    const client = new FakeTimelineClient(ENTRIES);
    client.onRequest = (current) => {
      if (current.requests.length === 2) current.epoch = "epoch-2";
    };

    const items = await fetchProjectedTimelineItems({ client, agentId: "agent", pageSize: 3 });

    expect(items).toEqual(ENTRIES.map((candidate) => candidate.item));
    expect(client.requests[2]?.direction).toBe("tail");
  });
});
