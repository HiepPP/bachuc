import { afterEach, describe, expect, it } from "vitest";
import type { CachedTimeline } from "@/runtime/replica-cache";
import { selectAgentTimelineState, useSessionStore } from "@/stores/session-store";
import {
  createAgentPrefetcher,
  isViewedAgentDesired,
  type AgentPrefetchPorts,
} from "./agent-prefetch";
import { planTimelineTailFetch } from "./timeline-sync-plan";
import {
  createTimelineReplica,
  createViewedTimelineOwner,
  type TimelineReplicaStorage,
} from "./viewed-timeline-sync";

interface FetchCall {
  serverId: string;
  agentId: string;
  request: Record<string, unknown>;
  resolve(): void;
  reject(error: Error): void;
}

function setup(overrides: Partial<AgentPrefetchPorts> = {}) {
  const fetches: FetchCall[] = [];
  const steps: string[] = [];
  const errors: unknown[] = [];
  const desired = new Set<string>();
  const featureHosts = new Set<string>();
  let now = 0;
  const prefetcher = createAgentPrefetcher({
    isAgentDesired: (serverId, agentId) => desired.has(`${serverId}:${agentId}`),
    supportsLatestPromptTail: (serverId) => featureHosts.has(serverId),
    prepareAgentRoute: async (serverId, agentId) => {
      steps.push(`route:${serverId}:${agentId}`);
    },
    prepareAgentTimeline: async (serverId, agentId) => {
      steps.push(`replica:${serverId}:${agentId}`);
    },
    fetchAgentTimeline: (serverId, agentId, request) =>
      new Promise<void>((resolve, reject) => {
        steps.push(`fetch:${serverId}:${agentId}`);
        fetches.push({ serverId, agentId, request: { ...request }, resolve, reject });
      }),
    now: () => now,
    reportError: (error) => errors.push(error),
    ...overrides,
  });
  return {
    prefetcher,
    fetches,
    steps,
    errors,
    desired,
    featureHosts,
    advance: (ms: number) => {
      now += ms;
    },
  };
}

const settle = () => new Promise((resolve) => setTimeout(resolve, 0));

describe("agent prefetch", () => {
  it("prepares the route and replica before one tail fetch", async () => {
    const { prefetcher, fetches, steps } = setup();
    prefetcher.prefetchAgent({ serverId: "h", agentId: "a" });
    await settle();
    expect(steps).toEqual(["route:h:a", "replica:h:a", "fetch:h:a"]);
    expect(fetches[0]?.request).toEqual(planTimelineTailFetch());
  });

  it("dedupes calls while a prefetch is running or queued", async () => {
    const { prefetcher, fetches } = setup();
    prefetcher.prefetchAgent({ serverId: "h", agentId: "a" });
    prefetcher.prefetchAgent({ serverId: "h", agentId: "b" });
    await settle();
    prefetcher.prefetchAgent({ serverId: "h", agentId: "a" });
    prefetcher.prefetchAgent({ serverId: "h", agentId: "b" });
    fetches[0]?.resolve();
    await settle();
    fetches[1]?.resolve();
    await settle();
    expect(fetches.map((call) => call.agentId)).toEqual(["a", "b"]);
  });

  it("skips agents the viewed timeline sync already holds", async () => {
    const { prefetcher, fetches, desired } = setup();
    desired.add("h:a");
    prefetcher.prefetchAgent({ serverId: "h", agentId: "a" });
    prefetcher.prefetchAgent({ serverId: "h", agentId: "b" });
    prefetcher.prefetchAgent({ serverId: "h", agentId: "c" });
    await settle();
    // c was queued before the user opened it.
    desired.add("h:c");
    fetches[0]?.resolve();
    await settle();
    expect(fetches.map((call) => call.agentId)).toEqual(["b"]);
  });

  it("does not prefetch the same agent again within 30 seconds", async () => {
    const { prefetcher, fetches, advance } = setup();
    prefetcher.prefetchAgent({ serverId: "h", agentId: "a" });
    await settle();
    fetches[0]?.resolve();
    await settle();
    advance(29_999);
    prefetcher.prefetchAgent({ serverId: "h", agentId: "a" });
    await settle();
    expect(fetches).toHaveLength(1);
    advance(1);
    prefetcher.prefetchAgent({ serverId: "h", agentId: "a" });
    await settle();
    expect(fetches).toHaveLength(2);
  });

  it("runs one prefetch per host and keeps only the newest three queued", async () => {
    const { prefetcher, fetches } = setup();
    for (const agentId of ["a", "b", "c", "d", "e"]) {
      prefetcher.prefetchAgent({ serverId: "h", agentId });
    }
    prefetcher.prefetchAgent({ serverId: "other", agentId: "x" });
    await settle();
    expect(fetches.map((call) => `${call.serverId}:${call.agentId}`)).toEqual(["h:a", "other:x"]);

    const hostCalls = () => fetches.filter((call) => call.serverId === "h");
    for (let index = 0; index < 4; index += 1) {
      expect(hostCalls()).toHaveLength(index + 1);
      hostCalls()[index]?.resolve();
      await settle();
    }
    expect(hostCalls().map((call) => call.agentId)).toEqual(["a", "c", "d", "e"]);
  });

  it("moves a repeated queued agent to the newest slot", async () => {
    const { prefetcher, fetches } = setup();
    for (const agentId of ["a", "b", "c", "d", "b", "e"]) {
      prefetcher.prefetchAgent({ serverId: "h", agentId });
    }
    for (let index = 0; index < 4; index += 1) {
      await settle();
      fetches[index]?.resolve();
    }
    await settle();
    expect(fetches.map((call) => call.agentId)).toEqual(["a", "d", "b", "e"]);
  });

  it("asks for the latest prompt only with the host feature and the latest-prompt anchor", async () => {
    const { prefetcher, fetches, featureHosts } = setup();
    featureHosts.add("new");
    const inputs = [
      { serverId: "new", agentId: "a", anchor: "latest-prompt" as const },
      { serverId: "new", agentId: "b", anchor: "bottom" as const },
      { serverId: "new", agentId: "c" },
      { serverId: "old", agentId: "d", anchor: "latest-prompt" as const },
    ];
    for (const input of inputs) {
      prefetcher.prefetchAgent(input);
      await settle();
      fetches.at(-1)?.resolve();
      await settle();
    }
    expect(fetches.map((call) => [call.agentId, call.request.includeLatestPrompt])).toEqual([
      ["a", true],
      ["b", undefined],
      ["c", undefined],
      ["d", undefined],
    ]);
  });

  it("swallows errors, allows a retry, and moves on to the next queued agent", async () => {
    const { prefetcher, fetches, errors } = setup();
    prefetcher.prefetchAgent({ serverId: "h", agentId: "a" });
    prefetcher.prefetchAgent({ serverId: "h", agentId: "b" });
    await settle();
    fetches[0]?.reject(new Error("offline"));
    await settle();
    expect(errors).toEqual([new Error("offline")]);
    expect(fetches.map((call) => call.agentId)).toEqual(["a", "b"]);
    fetches[1]?.resolve();
    await settle();
    prefetcher.prefetchAgent({ serverId: "h", agentId: "a" });
    await settle();
    expect(fetches.map((call) => call.agentId)).toEqual(["a", "b", "a"]);
  });

  it("reports a failing route preparation without fetching", async () => {
    const { prefetcher, fetches, errors } = setup({
      prepareAgentRoute: async () => {
        throw new Error("unknown host");
      },
    });
    prefetcher.prefetchAgent({ serverId: "h", agentId: "a" });
    await settle();
    expect(fetches).toEqual([]);
    expect(errors).toEqual([new Error("unknown host")]);
  });
});

function createOwner(serverId: string, storage: TimelineReplicaStorage) {
  const replica = createTimelineReplica({
    serverId,
    storage,
    prepareAgent: async () => undefined,
  });
  const owner = createViewedTimelineOwner({
    serverId,
    replica,
    replaceDemandedAgentIds: () => undefined,
    drainQueuedAgentMessage: () => undefined,
    ports: {
      observe: () => ({ ready: Promise.resolve(), release: async () => undefined }),
      readCursor: () => undefined,
      fetchPage: async () => ({ hasNewer: false, endCursor: null }),
      fetchLatestTail: async () => ({ hasNewer: false, endCursor: null }),
      reportError: () => undefined,
      schedule: () => () => undefined,
    },
  });
  return { replica, owner };
}

describe("host desired check", () => {
  const SERVER_ID = "agent-prefetch-desired-host";

  afterEach(() => useSessionStore.getState().clearSession(SERVER_ID));

  it("treats an opened agent as desired while its catch-up is still pending", () => {
    expect(isViewedAgentDesired(SERVER_ID, "a")).toBe(false);
    useSessionStore.getState().initializeSession(SERVER_ID, null);
    const { owner } = createOwner(SERVER_ID, {
      readTimeline: async () => undefined,
      commitTimeline: () => undefined,
    });
    useSessionStore.getState().setViewedTimelineSync(SERVER_ID, owner);
    expect(isViewedAgentDesired(SERVER_ID, "a")).toBe(false);
    owner.replaceVisibleAgentIds("pane", ["a"]);
    expect(owner.getAgentTimelineStatus("a")).toBe("pending");
    expect(isViewedAgentDesired(SERVER_ID, "a")).toBe(true);
    expect(isViewedAgentDesired(SERVER_ID, "b")).toBe(false);
    owner.dispose();
  });
});

describe("prefetched page and a later replica prepare", () => {
  const SERVER_ID = "agent-prefetch-host";
  const AGENT_ID = "agent-1";

  afterEach(() => useSessionStore.getState().clearSession(SERVER_ID));

  function cachedRow(seq: number, text: string): CachedTimeline {
    return {
      agentId: AGENT_ID,
      items: [
        {
          kind: "assistant_message",
          id: `cached-${seq}`,
          text,
          timestamp: new Date("2026-10-15T10:00:00.000Z"),
          timelineCursor: { epoch: "epoch-1", seq },
        },
      ],
      range: { epoch: "epoch-1", startSeq: 1, endSeq: seq },
      hasOlder: false,
    };
  }

  it("keeps the newer prefetched page when an older cache row is prepared afterwards", async () => {
    useSessionStore.getState().initializeSession(SERVER_ID, null);
    // The cache commit after the prefetch is not read back: storage still serves the old row.
    const storage = {
      readTimeline: async () => cachedRow(2, "old cached"),
      commitTimeline: () => undefined,
    };
    const { replica, owner } = createOwner(SERVER_ID, storage);

    // Prefetch: prepare the replica, then apply the tail page as host-runtime does.
    await replica.prepare(AGENT_ID);
    owner.applyTimelineResponse({
      requestId: "prefetch",
      agentId: AGENT_ID,
      agent: null,
      direction: "tail",
      projection: "projected",
      reset: false,
      epoch: "epoch-1",
      window: { minSeq: 1, maxSeq: 5, nextSeq: 6 },
      startCursor: { epoch: "epoch-1", seq: 5 },
      endCursor: { epoch: "epoch-1", seq: 5 },
      entries: [
        {
          provider: "mock",
          item: { type: "assistant_message", text: "prefetched" },
          timestamp: "2026-10-15T10:01:00.000Z",
          seqStart: 5,
          seqEnd: 5,
          sourceSeqRanges: [{ startSeq: 5, endSeq: 5 }],
          collapsed: [],
        },
      ],
      error: null,
      hasNewer: false,
      hasOlder: true,
      staleCursor: false,
      gap: false,
    });

    // The open: route prep and the viewed sync prepare the replica again, from a fresh replica too.
    await replica.prepare(AGENT_ID);
    await createTimelineReplica({
      serverId: SERVER_ID,
      storage,
      prepareAgent: async () => undefined,
    }).prepare(AGENT_ID);

    const timeline = selectAgentTimelineState(
      useSessionStore.getState().sessions[SERVER_ID],
      AGENT_ID,
    );
    expect(timeline).toMatchObject({
      status: "synced",
      range: { epoch: "epoch-1", endSeq: 5 },
      items: [{ kind: "assistant_message", text: "prefetched" }],
    });
    owner.dispose();
  });
});
