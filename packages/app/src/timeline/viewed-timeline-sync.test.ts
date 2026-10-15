import { afterEach, describe, expect, test, vi } from "vitest";
import { DaemonClient, type DaemonTransport } from "@getpaseo/client/internal/daemon-client";
import type {
  AgentStreamEventPayload,
  SessionInboundMessage,
  SessionOutboundMessage,
} from "@getpaseo/protocol/messages";
import type { CachedTimeline } from "@/runtime/replica-cache";
import { selectAgentTimelineState, useSessionStore } from "@/stores/session-store";
import type { AgentStreamReducerEvent } from "./session-stream-reducers";
import type { ProjectedTimelineForwardFetchPlan } from "./timeline-sync-plan";
import {
  consumeForcedTimelineTailReplacement,
  createTimelineReplica,
  createViewedTimelineOwner,
  createViewedTimelineSync,
  RECENT_STREAM_LIMIT,
  type TimelineResponsePayload,
  type ViewedTimelineStatus,
  type ViewedTimelineSyncPorts,
} from "./viewed-timeline-sync";

interface Deferred<T> {
  promise: Promise<T>;
  resolve(value: T): void;
  reject(error: Error): void;
}

function deferred<T>(): Deferred<T> {
  let resolve!: (value: T) => void;
  let reject!: (error: Error) => void;
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, resolve, reject };
}

interface MembershipRequest {
  agentIds: string[];
  succeed(): void;
  fail(message: string): void;
}

interface TimelineCurrentReport {
  agentId: string;
  status: ViewedTimelineStatus;
}

interface TimelineFetch {
  agentId: string;
  request: ProjectedTimelineForwardFetchPlan;
  respond(input: { hasNewer: boolean; seq?: number }): void;
  fail(message: string): void;
}

test("split panes catch up together before hidden open chats start fetching", async () => {
  const world = new TimelineWorld();
  world.openedEarlier(["a-hidden"]);
  world.sync.replaceVisibleAgentIds("panes", ["y-visible", "z-visible"]);
  world.sync.setConnected(true);
  (await world.nextMembership()).succeed();
  const left = await world.nextFetch("y-visible");
  const right = await world.nextFetch("z-visible");
  world.expectNoPendingFetch();
  left.respond({ hasNewer: false });
  await vi.waitFor(() => expect(world.sync.getAgentTimelineStatus("y-visible")).toBe("ready"));
  world.expectNoPendingFetch();
  right.respond({ hasNewer: false });
  (await world.nextFetch("a-hidden")).respond({ hasNewer: false });
  world.sync.dispose();
});

test("backgrounding releases hidden catch-ups waiting on a visible chat", async () => {
  const world = new TimelineWorld();
  world.openedEarlier(["a-hidden"]);
  world.sync.replaceVisibleAgentIds("pane", ["z-visible"]);
  world.sync.setConnected(true);
  (await world.nextMembership()).succeed();
  const visible = await world.nextFetch("z-visible");
  world.expectNoPendingFetch();
  world.sync.setActive(false);
  (await world.nextFetch("a-hidden")).respond({ hasNewer: false });
  visible.respond({ hasNewer: false });
  world.sync.dispose();
});

test("visible catch-up settles before hidden open chats start fetching", async () => {
  const world = new TimelineWorld();
  world.openedEarlier(["a-hidden"]);
  world.sync.replaceVisibleAgentIds("pane", ["z-visible"]);
  world.sync.setConnected(true);
  (await world.nextMembership()).succeed();
  const visible = await world.nextFetch("z-visible");
  world.expectNoPendingFetch();
  visible.respond({ hasNewer: false });
  const hidden = await world.nextFetch("a-hidden");
  expect(world.sync.getAgentTimelineStatus("z-visible")).toBe("ready");
  hidden.respond({ hasNewer: false });
  world.sync.dispose();
});

test("a failed visible catch-up releases hidden chats without bypassing its retry delay", async () => {
  const world = new TimelineWorld();
  world.openedEarlier(["a-hidden"]);
  world.sync.replaceVisibleAgentIds("pane", ["z-visible"]);
  world.sync.setConnected(true);
  (await world.nextMembership()).succeed();
  const visible = await world.nextFetch("z-visible");
  world.expectNoPendingFetch();
  visible.fail("temporarily unavailable");
  const hidden = await world.nextFetch("a-hidden");
  hidden.respond({ hasNewer: false });
  await world.nextError();
  world.expectNoPendingFetch();
  (await world.nextRetry())();
  (await world.nextFetch("z-visible")).respond({ hasNewer: false });
  world.sync.dispose();
});

test("switching chats promotes the newly visible catch-up while another visible request is pending", async () => {
  const world = new TimelineWorld();
  world.openedEarlier(["a-hidden", "b-hidden"]);
  world.sync.replaceVisibleAgentIds("pane", ["z-visible"]);
  world.sync.setConnected(true);
  (await world.nextMembership()).succeed();
  const previous = await world.nextFetch("z-visible");
  world.expectNoPendingFetch();
  world.sync.replaceVisibleAgentIds("pane", ["b-hidden"]);
  const current = await world.nextFetch("b-hidden");
  world.expectNoPendingFetch();
  current.respond({ hasNewer: false });
  (await world.nextFetch("a-hidden")).respond({ hasNewer: false });
  previous.respond({ hasNewer: false });
  world.sync.dispose();
});

test("reconnect fetches the visible chat first and at most two hidden chats at a time", async () => {
  const world = new TimelineWorld();
  const hidden = Array.from({ length: 10 }, (_, index) => `hidden-${index}`);
  world.openedEarlier(hidden);
  world.show("pane", ["z-visible"]);
  world.sync.setConnected(true);
  (await world.nextMembership()).succeed();
  const visible = await world.nextFetch("z-visible");
  world.expectNoPendingFetch();
  visible.respond({ hasNewer: false });

  const finished: string[] = [];
  while (finished.length < hidden.length) {
    const expected = Math.min(2, hidden.length - finished.length);
    await vi.waitFor(() => expect(world.pendingFetchAgentIds()).toHaveLength(expected));
    const [agentId] = world.pendingFetchAgentIds();
    (await world.nextFetch(agentId)).respond({ hasNewer: false });
    finished.push(agentId);
  }

  expect([...finished].sort()).toEqual(hidden);
  await vi.waitFor(() =>
    expect(hidden.every((id) => world.sync.getAgentTimelineStatus(id) === "ready")).toBe(true),
  );
  world.expectNoPendingFetch();
  world.sync.dispose();
});

test("a hidden chat held back by the cap starts at once when revealed", async () => {
  const world = new TimelineWorld();
  world.openedEarlier(["a-hidden", "b-hidden", "c-hidden"]);
  world.sync.setConnected(true);
  (await world.nextMembership()).succeed();
  await vi.waitFor(() => expect(world.pendingFetchAgentIds()).toEqual(["a-hidden", "b-hidden"]));

  world.show("pane", ["c-hidden"]);

  const revealed = await world.nextFetch("c-hidden");
  expect(world.pendingFetchAgentIds()).toEqual(["a-hidden", "b-hidden"]);
  revealed.respond({ hasNewer: false });
  (await world.nextFetch("a-hidden")).respond({ hasNewer: false });
  (await world.nextFetch("b-hidden")).respond({ hasNewer: false });
  world.expectNoPendingFetch();
  world.sync.dispose();
});

test("gap recovery of a hidden chat bypasses the hidden catch-up cap", async () => {
  const world = new TimelineWorld();
  world.openedEarlier(["a-hidden", "b-hidden", "c-hidden"]);
  world.sync.setConnected(true);
  (await world.nextMembership()).succeed();
  await vi.waitFor(() => expect(world.pendingFetchAgentIds()).toEqual(["a-hidden", "b-hidden"]));

  world.sync.recoverGap("c-hidden", { epoch: "epoch-c-hidden", endSeq: 10 });

  const gap = await world.nextFetch("c-hidden");
  expect(gap.request).toEqual({
    direction: "after",
    cursor: { epoch: "epoch-c-hidden", seq: 10 },
    limit: 40,
    projection: "projected",
  });
  expect(world.pendingFetchAgentIds()).toEqual(["a-hidden", "b-hidden"]);
  gap.respond({ hasNewer: false });
  (await world.nextFetch("a-hidden")).respond({ hasNewer: false });
  (await world.nextFetch("b-hidden")).respond({ hasNewer: false });
  world.expectNoPendingFetch();
  world.sync.dispose();
});

test("a manual retry of a hidden chat bypasses the hidden catch-up cap", async () => {
  const world = new TimelineWorld();
  world.openedEarlier(["a-hidden", "b-hidden", "c-hidden"]);
  world.sync.setConnected(true);
  (await world.nextMembership()).succeed();
  (await world.nextFetch("a-hidden")).fail("temporarily unavailable");
  await world.nextError();
  await vi.waitFor(() => expect(world.pendingFetchAgentIds()).toEqual(["b-hidden", "c-hidden"]));

  world.sync.retryVisibleAgentTimeline("a-hidden");

  const retried = await world.nextFetch("a-hidden");
  expect(world.pendingFetchAgentIds()).toEqual(["b-hidden", "c-hidden"]);
  retried.respond({ hasNewer: false });
  await vi.waitFor(() => expect(world.sync.getAgentTimelineStatus("a-hidden")).toBe("ready"));
  (await world.nextFetch("b-hidden")).respond({ hasNewer: false });
  (await world.nextFetch("c-hidden")).respond({ hasNewer: false });
  world.sync.dispose();
});

class TimelineWorld {
  constructor(private readonly observeMembership?: ViewedTimelineSyncPorts["observe"]) {}
  readonly errors: string[] = [];
  readonly releasedMemberships: string[][] = [];
  readonly cursors = new Map<string, { epoch: string; endSeq: number }>();
  readonly cacheRequests: string[] = [];
  readonly forcedTimelineTailReplacements = new Set<string>();
  readonly reportedCurrent: TimelineCurrentReport[] = [];
  cacheGate: Deferred<void> | null = null;
  /** Threads that hold stream items, as the owner's store would report them. */
  readonly heldStreams = new Set<string>();
  readonly evictions: string[] = [];
  readonly sync = createViewedTimelineSync({
    listStreamAgentIds: () => [...this.heldStreams],
    evictStream: (agentId) => {
      this.heldStreams.delete(agentId);
      this.evictions.push(agentId);
      return true;
    },
    replaceDemandedAgentIds: () => undefined,
    onCatchUpEnded: (agentId) => {
      this.reportedCurrent.push({
        agentId,
        status: this.sync.getAgentTimelineStatus(agentId),
      });
    },
    prepare: async (agentId) => {
      this.cacheRequests.push(agentId);
      this.cacheRequestWaiters.shift()?.(agentId);
      await this.cacheGate?.promise;
    },
    observe: (agentIds) => {
      if (this.observeMembership) return this.observeMembership(agentIds);
      const result = deferred<void>();
      this.memberships.push({
        agentIds,
        succeed: () => result.resolve(),
        fail: (message) => result.reject(new Error(message)),
      });
      this.releaseMembershipWaiter();
      return {
        ready: result.promise,
        release: async () => {
          this.releasedMemberships.push(agentIds);
        },
      };
    },
    readCursor: (agentId) => this.cursors.get(agentId),
    fetchPage: async (agentId, request) => {
      const result = deferred<{
        hasNewer: boolean;
        endCursor: { epoch: string; seq: number } | null;
      }>();
      this.fetches.push({
        agentId,
        request,
        respond: ({ hasNewer, seq = 1 }) =>
          result.resolve({
            hasNewer,
            endCursor: { epoch: `epoch-${agentId}`, seq },
          }),
        fail: (message) => result.reject(new Error(message)),
      });
      this.releaseFetchWaiters();
      return result.promise;
    },
    fetchLatestTail: async (agentId) => {
      this.forcedTimelineTailReplacements.add(agentId);
      try {
        return await this.fetchTimeline(agentId, {
          direction: "tail",
          limit: 40,
          projection: "projected",
        });
      } finally {
        this.forcedTimelineTailReplacements.delete(agentId);
      }
    },
    reportError: (error) => {
      this.errors.push(error instanceof Error ? error.message : String(error));
      const waiter = this.errorWaiters.shift();
      if (waiter) waiter(this.errors.at(-1) ?? "");
    },
    schedule: (task, delayMs) => {
      const scheduled = { task, delayMs };
      this.scheduled.push(scheduled);
      const waiterIndex = this.retryWaiters.findIndex((waiter) => waiter.delayMs === delayMs);
      if (waiterIndex >= 0) {
        const [waiter] = this.retryWaiters.splice(waiterIndex, 1);
        this.scheduled.splice(this.scheduled.indexOf(scheduled), 1);
        waiter.resolve(task);
      }
      return () => {
        const index = this.scheduled.indexOf(scheduled);
        if (index >= 0) this.scheduled.splice(index, 1);
      };
    },
  });

  show(sourceId: string, agentIds: string[]): void {
    this.sync.replaceVisibleAgentIds(sourceId, agentIds);
  }

  /** Shows each thread in turn in one pane and lets its cache load and items land. */
  async openInTurn(sourceId: string, agentIds: string[]): Promise<void> {
    for (const agentId of agentIds) {
      this.show(sourceId, [agentId]);
      await new Promise((resolve) => setTimeout(resolve, 0));
      this.heldStreams.add(agentId);
    }
  }

  /** Chats the user opened earlier in this session and has since navigated away from. */
  openedEarlier(agentIds: string[]): void {
    this.sync.replaceVisibleAgentIds("earlier", agentIds);
    this.sync.replaceVisibleAgentIds("earlier", []);
  }

  private readonly memberships: MembershipRequest[] = [];
  private readonly membershipWaiters: Array<(request: MembershipRequest) => void> = [];
  private readonly fetches: TimelineFetch[] = [];
  private readonly fetchWaiters: Array<{
    agentId: string;
    resolve(fetch: TimelineFetch): void;
  }> = [];
  private readonly errorWaiters: Array<(message: string) => void> = [];
  private readonly cacheRequestWaiters: Array<(agentId: string) => void> = [];
  private readonly scheduled: Array<{ task: () => void; delayMs: number }> = [];
  private readonly retryWaiters: Array<{
    delayMs: number;
    resolve(retry: () => void): void;
  }> = [];

  get pendingFetchCount(): number {
    return this.fetches.length;
  }

  pendingFetchAgentIds(): string[] {
    return this.fetches.map((fetch) => fetch.agentId).sort();
  }

  applyTimelineResponse(payload: TimelineResponsePayload): TimelineResponsePayload {
    return consumeForcedTimelineTailReplacement(payload, this.forcedTimelineTailReplacements);
  }

  nextCacheRequest(): Promise<string> {
    const request = this.cacheRequests.at(-1);
    if (request) return Promise.resolve(request);
    return new Promise((resolve) => this.cacheRequestWaiters.push(resolve));
  }

  private fetchTimeline(
    agentId: string,
    request: ProjectedTimelineForwardFetchPlan,
  ): Promise<{ hasNewer: boolean; endCursor: { epoch: string; seq: number } | null }> {
    const result = deferred<{
      hasNewer: boolean;
      endCursor: { epoch: string; seq: number } | null;
    }>();
    this.fetches.push({
      agentId,
      request,
      respond: ({ hasNewer, seq = 1 }) =>
        result.resolve({ hasNewer, endCursor: { epoch: `epoch-${agentId}`, seq } }),
      fail: (message) => result.reject(new Error(message)),
    });
    this.releaseFetchWaiters();
    return result.promise;
  }

  nextMembership(): Promise<MembershipRequest> {
    const request = this.memberships.shift();
    if (request) return Promise.resolve(request);
    return new Promise((resolve) => this.membershipWaiters.push(resolve));
  }

  nextFetch(agentId: string): Promise<TimelineFetch> {
    const index = this.fetches.findIndex((fetch) => fetch.agentId === agentId);
    if (index >= 0) return Promise.resolve(this.fetches.splice(index, 1)[0]);
    return new Promise((resolve) => this.fetchWaiters.push({ agentId, resolve }));
  }

  expectNoPendingMembership(): void {
    expect(this.memberships).toEqual([]);
  }

  expectNoPendingFetch(): void {
    expect(this.fetches).toEqual([]);
  }

  nextError(): Promise<string> {
    const message = this.errors.at(-1);
    if (message) return Promise.resolve(message);
    return new Promise((resolve) => this.errorWaiters.push(resolve));
  }

  nextRetry(delayMs = 1_000): Promise<() => void> {
    const index = this.scheduled.findIndex((entry) => entry.delayMs === delayMs);
    if (index >= 0) return Promise.resolve(this.scheduled.splice(index, 1)[0].task);
    return new Promise((resolve) => this.retryWaiters.push({ delayMs, resolve }));
  }

  elapse(elapsedMs: number): void {
    const due = this.scheduled.filter((entry) => entry.delayMs <= elapsedMs);
    this.scheduled.splice(
      0,
      this.scheduled.length,
      ...this.scheduled.filter((entry) => entry.delayMs > elapsedMs),
    );
    for (const entry of due) entry.task();
  }

  private releaseMembershipWaiter(): void {
    const waiter = this.membershipWaiters.shift();
    if (!waiter) return;
    const request = this.memberships.shift();
    if (request) waiter(request);
  }

  private releaseFetchWaiters(): void {
    for (let waiterIndex = this.fetchWaiters.length - 1; waiterIndex >= 0; waiterIndex -= 1) {
      const waiter = this.fetchWaiters[waiterIndex];
      const index = this.fetches.findIndex((fetch) => fetch.agentId === waiter.agentId);
      if (index < 0) continue;
      this.fetchWaiters.splice(waiterIndex, 1);
      waiter.resolve(this.fetches.splice(index, 1)[0]);
    }
  }
}

test("an agent is desired from its first show until its tab closes, even while its catch-up is pending", () => {
  const world = new TimelineWorld();
  expect(world.sync.isAgentDesired("agent-a")).toBe(false);
  world.show("pane", ["agent-a"]);
  expect(world.sync.getAgentTimelineStatus("agent-a")).toBe("pending");
  expect(world.sync.isAgentDesired("agent-a")).toBe(true);
  world.sync.replaceVisibleAgentIds("pane", []);
  expect(world.sync.isAgentDesired("agent-a")).toBe(true);
  world.sync.replaceOpenTabAgentIds([]);
  expect(world.sync.isAgentDesired("agent-a")).toBe(false);
  world.sync.dispose();
});

test("uses a tail fetch when an agent becomes visible", async () => {
  const world = new TimelineWorld();
  world.sync.setConnected(true);
  world.sync.replaceVisibleAgentIds("workspace", ["agent-a"]);
  const membership = await world.nextMembership();
  membership.succeed();

  const fetch = await world.nextFetch("agent-a");
  expect(fetch.request).toEqual({ direction: "tail", limit: 40, projection: "projected" });
  fetch.respond({ hasNewer: false });
});

test("loads the cache before choosing the authoritative network request", async () => {
  const world = new TimelineWorld();
  world.cacheGate = deferred<void>();
  world.sync.setConnected(true);
  world.sync.replaceVisibleAgentIds("workspace", ["agent-a"]);
  const membership = await world.nextMembership();
  membership.succeed();
  const cacheRequest = await world.nextCacheRequest();

  expect(cacheRequest).toBe("agent-a");
  expect(world.pendingFetchCount).toBe(0);

  world.cursors.set("agent-a", { epoch: "cached-epoch", endSeq: 17 });
  world.cacheGate.resolve();
  const fetch = await world.nextFetch("agent-a");

  expect(fetch.request).toEqual({
    direction: "after",
    cursor: { epoch: "cached-epoch", seq: 17 },
    limit: 40,
    projection: "projected",
  });
  fetch.respond({ hasNewer: false });
});

test("catches up after the restored cursor when an agent becomes visible", async () => {
  const world = new TimelineWorld();
  world.cursors.set("agent-a", { epoch: "epoch-agent-a", endSeq: 42 });
  world.sync.setConnected(true);
  world.sync.replaceVisibleAgentIds("workspace", ["agent-a"]);
  const membership = await world.nextMembership();
  membership.succeed();

  const fetch = await world.nextFetch("agent-a");
  expect(fetch.request).toEqual({
    direction: "after",
    cursor: { epoch: "epoch-agent-a", seq: 42 },
    limit: 40,
    projection: "projected",
  });
  fetch.respond({ hasNewer: false });
});

test("falls back to the latest tail when a restored cursor has more than one catch-up page", async () => {
  const world = new TimelineWorld();
  world.cursors.set("agent-a", { epoch: "epoch-agent-a", endSeq: 42 });
  world.sync.setConnected(true);
  world.sync.replaceVisibleAgentIds("workspace", ["agent-a"]);
  const membership = await world.nextMembership();
  membership.succeed();

  const probe = await world.nextFetch("agent-a");
  expect(probe.request.direction).toBe("after");
  probe.respond({ hasNewer: true, seq: 82 });

  const fallback = await world.nextFetch("agent-a");
  expect(fallback.request).toEqual({ direction: "tail", limit: 40, projection: "projected" });
  expect(
    world.applyTimelineResponse({
      requestId: "fallback-tail",
      agentId: "agent-a",
      agent: null,
      direction: "tail",
      projection: "projected",
      epoch: "epoch-agent-a",
      reset: false,
      staleCursor: false,
      gap: false,
      window: { minSeq: 43, maxSeq: 82, nextSeq: 83 },
      startCursor: { epoch: "epoch-agent-a", seq: 43 },
      endCursor: { epoch: "epoch-agent-a", seq: 82 },
      hasOlder: true,
      hasNewer: false,
      entries: [],
      error: null,
    }).reset,
  ).toBe(true);
  fallback.respond({ hasNewer: false });
});

test("a gap absorbed by a running tail is recovered after the tail completes", async () => {
  const world = new TimelineWorld();
  world.sync.setConnected(true);
  world.sync.replaceVisibleAgentIds("workspace", ["agent-a"]);
  const membership = await world.nextMembership();
  membership.succeed();
  const tail = await world.nextFetch("agent-a");

  world.sync.recoverGap("agent-a", { epoch: "epoch-agent-a", endSeq: 9 });

  world.expectNoPendingFetch();
  tail.respond({ hasNewer: false });

  const recovery = await world.nextFetch("agent-a");
  expect(recovery.request).toEqual({
    direction: "after",
    cursor: { epoch: "epoch-agent-a", seq: 9 },
    limit: 40,
    projection: "projected",
  });
  recovery.respond({ hasNewer: false });
  await vi.waitFor(() => expect(world.sync.getAgentTimelineStatus("agent-a")).toBe("ready"));
});

test("reports a chat current only once its parked follow-up page completes", async () => {
  const world = new TimelineWorld();
  world.sync.setConnected(true);
  world.sync.replaceVisibleAgentIds("workspace", ["agent-a"]);
  (await world.nextMembership()).succeed();
  const tail = await world.nextFetch("agent-a");

  world.sync.recoverGap("agent-a", { epoch: "epoch-agent-a", endSeq: 9 });
  tail.respond({ hasNewer: false });

  const recovery = await world.nextFetch("agent-a");
  expect(world.reportedCurrent).toEqual([]);
  expect(world.sync.getAgentTimelineStatus("agent-a")).toBe("pending");

  recovery.respond({ hasNewer: false });
  await vi.waitFor(() => expect(world.sync.getAgentTimelineStatus("agent-a")).toBe("ready"));
  expect(world.reportedCurrent).toEqual([{ agentId: "agent-a", status: "ready" }]);
  world.sync.dispose();
});

test("reports a chat current once the latest-tail fallback replaces an overflowing catch-up", async () => {
  const world = new TimelineWorld();
  world.cursors.set("agent-a", { epoch: "epoch-agent-a", endSeq: 42 });
  world.sync.setConnected(true);
  world.sync.replaceVisibleAgentIds("workspace", ["agent-a"]);
  (await world.nextMembership()).succeed();

  const probe = await world.nextFetch("agent-a");
  probe.respond({ hasNewer: true, seq: 82 });

  const fallback = await world.nextFetch("agent-a");
  expect(world.reportedCurrent).toEqual([]);

  fallback.respond({ hasNewer: false });
  await vi.waitFor(() => expect(world.sync.getAgentTimelineStatus("agent-a")).toBe("ready"));
  expect(world.reportedCurrent).toEqual([{ agentId: "agent-a", status: "ready" }]);
  world.sync.dispose();
});

test("stops owing a chat a catch-up when its tab closes mid-fetch", async () => {
  const world = new TimelineWorld();
  world.sync.setConnected(true);
  world.sync.replaceVisibleAgentIds("pane", ["agent-a"]);
  (await world.nextMembership()).succeed();
  await world.nextFetch("agent-a");
  expect(world.reportedCurrent).toEqual([]);

  world.sync.replaceVisibleAgentIds("pane", []);
  world.sync.replaceOpenTabAgentIds([]);

  expect(world.reportedCurrent).toEqual([{ agentId: "agent-a", status: "pending" }]);
  world.sync.dispose();
});

test("unchanged visible-set publication does not cancel paged catch-up", async () => {
  const world = new TimelineWorld();
  world.sync.setConnected(true);
  world.sync.replaceVisibleAgentIds("workspace", ["agent-a"]);
  expect(world.sync.getAgentTimelineStatus("agent-a")).toBe("pending");
  const membership = await world.nextMembership();
  membership.succeed();
  const firstPage = await world.nextFetch("agent-a");

  world.sync.replaceVisibleAgentIds("workspace", ["agent-a", "agent-a"]);
  firstPage.respond({ hasNewer: true, seq: 5 });
  const secondPage = await world.nextFetch("agent-a");
  expect(world.sync.getAgentTimelineStatus("agent-a")).toBe("pending");
  secondPage.respond({ hasNewer: false });

  await vi.waitFor(() => {
    expect(world.sync.getAgentTimelineStatus("agent-a")).toBe("ready");
  });

  expect(secondPage.request).toEqual({
    direction: "after",
    cursor: { epoch: "epoch-agent-a", seq: 5 },
    limit: 40,
    projection: "projected",
  });
  world.expectNoPendingMembership();
});

test("all acknowledged agents begin catch-up independently", async () => {
  const world = new TimelineWorld();
  world.sync.setConnected(true);
  world.sync.replaceVisibleAgentIds("workspace", ["agent-b", "agent-a"]);
  const membership = await world.nextMembership();
  membership.succeed();

  const agentB = await world.nextFetch("agent-b");
  agentB.respond({ hasNewer: false });
  const agentA = await world.nextFetch("agent-a");
  agentA.respond({ hasNewer: false });

  expect(membership.agentIds).toEqual(["agent-a", "agent-b"]);
});

test("an eviction starts and acknowledges B before A returns its late subscription ID", async () => {
  const requests: SessionInboundMessage[] = [];
  let receive!: (data: unknown, isBinary: boolean) => void;
  let open!: () => void;
  const reply = (message: SessionOutboundMessage) =>
    receive(JSON.stringify({ type: "session", message }), false);
  const transport: DaemonTransport = {
    send(data) {
      if (typeof data !== "string") throw new Error("No binary request expected");
      const frame = JSON.parse(data);
      if (frame.type === "ping") receive(JSON.stringify({ type: "pong" }), false);
      if (frame.type !== "session") return;
      requests.push(frame.message);
      if (frame.message.type === "subscription.release.request")
        reply({
          type: "subscription.release.response",
          payload: {
            requestId: frame.message.requestId,
            subscriptionId: frame.message.subscriptionId,
          },
        });
    },
    close() {},
    onMessage(handler) {
      receive = handler;
      return () => {};
    },
    onOpen(handler) {
      open = handler;
      return () => {};
    },
    onClose() {
      return () => {};
    },
    onError() {
      return () => {};
    },
  };
  const client = new DaemonClient({
    url: "ws://timeline-fixture",
    clientId: "timeline-replacement",
    transportFactory: () => transport,
    reconnect: { enabled: false },
  });
  const world = new TimelineWorld((agentIds) => client.observeTimeline(agentIds));
  const membershipRequests = () =>
    requests.filter((request) => request.type === "agent.timeline.set_subscription.request");
  try {
    const connected = client.connect();
    open();
    reply({
      type: "status",
      payload: {
        status: "server_info",
        serverId: "timeline-fixture",
        hostname: null,
        version: null,
        features: { ownedSubscriptions: true },
      },
    });
    await connected;
    world.sync.replaceVisibleAgentIds("workspace", [
      "agent-a",
      "agent-b",
      "agent-c",
      "agent-d",
      "agent-e",
    ]);
    world.sync.setConnected(true);
    await vi.waitFor(() => expect(membershipRequests()).toHaveLength(1));
    const a = membershipRequests()[0]!;
    // Closing the five tabs releases them; opening a sixth chat is what commits the swap.
    world.sync.replaceOpenTabAgentIds(["agent-f"]);
    world.sync.replaceVisibleAgentIds("workspace", ["agent-f"]);
    await vi.waitFor(() => expect(membershipRequests()).toHaveLength(2));
    const b = membershipRequests()[1]!;
    reply({
      type: "agent.timeline.set_subscription.response",
      payload: { requestId: b.requestId, agentIds: b.agentIds, subscriptionId: "server-b" },
    });
    const catchUps = await Promise.all(b.agentIds.map((agentId) => world.nextFetch(agentId)));
    for (const catchUp of catchUps) catchUp.respond({ hasNewer: false });
    await vi.waitFor(() => expect(world.sync.getAgentTimelineStatus("agent-f")).toBe("ready"));
    reply({
      type: "agent.timeline.set_subscription.response",
      payload: { requestId: a.requestId, agentIds: a.agentIds, subscriptionId: "server-a-late" },
    });
    await vi.waitFor(() =>
      expect(requests.filter((request) => request.type === "subscription.release.request")).toEqual(
        [expect.objectContaining({ subscriptionId: "server-a-late" })],
      ),
    );
    expect(a.agentIds).toEqual(["agent-a", "agent-b", "agent-c", "agent-d", "agent-e"]);
    expect(b.agentIds).toEqual(["agent-f"]);
    world.expectNoPendingFetch();
    expect(world.errors).toEqual([]);
    world.sync.dispose();
    await vi.waitFor(() =>
      expect(
        requests
          .filter((request) => request.type === "subscription.release.request")
          .map((request) => request.subscriptionId),
      ).toEqual(["server-a-late", "server-b"]),
    );
  } finally {
    world.sync.dispose();
    await client.close();
  }
});

test("a restored workspace layout subscribes nothing until the user opens a chat", async () => {
  const world = new TimelineWorld();
  // Launch. Workspace layout rehydrates from disk carrying a tab for every chat the user
  // has ever opened on this host; none of them is open in this session yet.
  world.sync.replaceOpenTabAgentIds(["agent-a", "agent-b", "agent-c", "agent-d", "agent-e"]);
  world.sync.setConnected(true);
  world.expectNoPendingMembership();
  world.expectNoPendingFetch();

  // The workspace the app restores into shows one chat. Only that chat is fetched, so only
  // that agent is resumed on the daemon.
  world.sync.replaceVisibleAgentIds("workspace", ["agent-a"]);
  const membership = await world.nextMembership();
  expect(membership.agentIds).toEqual(["agent-a"]);
  membership.succeed();
  (await world.nextFetch("agent-a")).respond({ hasNewer: false });
  world.expectNoPendingFetch();
  world.sync.dispose();
});

test("open chats remain subscribed beyond five views and without mounted workspace panes", async () => {
  const world = new TimelineWorld();
  const agents = ["agent-a", "agent-b", "agent-c", "agent-d", "agent-e", "agent-f"];
  world.openedEarlier(agents);
  world.sync.replaceVisibleAgentIds("workspace", ["agent-a"]);
  world.sync.setConnected(true);
  const membership = await world.nextMembership();
  expect(membership.agentIds).toEqual(agents);
  membership.succeed();
  for (const agentId of agents) {
    (await world.nextFetch(agentId)).respond({ hasNewer: false });
  }
  await vi.waitFor(() => expect(world.sync.getAgentTimelineStatus("agent-a")).toBe("ready"));

  world.sync.replaceVisibleAgentIds("workspace", []);
  world.expectNoPendingMembership();
  world.sync.replaceVisibleAgentIds("workspace", ["agent-a"]);
  world.expectNoPendingFetch();
  expect(world.sync.getAgentTimelineStatus("agent-a")).toBe("ready");

  world.sync.replaceOpenTabAgentIds(agents.slice(0, 5));
  const closed = await world.nextMembership();
  expect(closed.agentIds).toEqual(agents.slice(0, 5));
  closed.succeed();
  world.sync.dispose();
});

test("disconnect cancels paging and reconnect restores membership before fresh catch-up", async () => {
  const world = new TimelineWorld();
  world.sync.setConnected(true);
  world.sync.replaceVisibleAgentIds("workspace", ["agent-a"]);
  const firstMembership = await world.nextMembership();
  firstMembership.succeed();
  const stalePage = await world.nextFetch("agent-a");

  world.sync.setConnected(false);
  expect(world.sync.getAgentTimelineStatus("agent-a")).toBe("pending");
  stalePage.respond({ hasNewer: true, seq: 8 });
  world.sync.setConnected(true);
  const restoredMembership = await world.nextMembership();
  restoredMembership.succeed();
  const restoredPage = await world.nextFetch("agent-a");
  restoredPage.respond({ hasNewer: false });

  await vi.waitFor(() => expect(world.sync.getAgentTimelineStatus("agent-a")).toBe("ready"));

  expect(restoredMembership.agentIds).toEqual(["agent-a"]);
  world.expectNoPendingFetch();
});

test("navigation while disconnected keeps both opened chats for reconnect", async () => {
  const world = new TimelineWorld();
  world.sync.replaceVisibleAgentIds("workspace", ["agent-a"]);
  world.sync.replaceVisibleAgentIds("workspace", ["agent-b"]);

  world.sync.setConnected(true);
  const membership = await world.nextMembership();
  expect(membership.agentIds).toEqual(["agent-a", "agent-b"]);
  membership.succeed();
  // A dropped socket is not the user closing a chat, so reconnect owes both of them a
  // catch-up. The visible one goes first.
  (await world.nextFetch("agent-b")).respond({ hasNewer: false });
  (await world.nextFetch("agent-a")).respond({ hasNewer: false });
  world.expectNoPendingFetch();
});

test("overlapping sources deduplicate membership and retain open hidden chats", async () => {
  const world = new TimelineWorld();
  world.show("left-route", ["agent-a"]);
  world.show("right-route", ["agent-a", "agent-b"]);
  world.sync.setConnected(true);
  const combined = await world.nextMembership();
  combined.succeed();
  const [agentA, agentB] = await Promise.all([
    world.nextFetch("agent-a"),
    world.nextFetch("agent-b"),
  ]);
  agentA.respond({ hasNewer: false });
  agentB.respond({ hasNewer: false });

  world.show("left-route", []);
  world.expectNoPendingMembership();
  world.show("right-route", ["agent-b"]);

  expect(combined.agentIds).toEqual(["agent-a", "agent-b"]);
  world.expectNoPendingMembership();
  world.expectNoPendingFetch();
});

test("a failed catch-up reports once and retries through the explicit retry policy", async () => {
  const world = new TimelineWorld();
  world.sync.setConnected(true);
  world.sync.replaceVisibleAgentIds("workspace", ["agent-a"]);
  const membership = await world.nextMembership();
  membership.succeed();
  const failed = await world.nextFetch("agent-a");
  failed.fail("timeline unavailable");
  const [error, retryCatchUp] = await Promise.all([world.nextError(), world.nextRetry()]);
  expect(world.sync.getAgentTimelineStatus("agent-a")).toBe("error");

  retryCatchUp();
  const retry = await world.nextFetch("agent-a");
  expect(world.sync.getAgentTimelineStatus("agent-a")).toBe("error");
  retry.respond({ hasNewer: false });
  await vi.waitFor(() => expect(world.sync.getAgentTimelineStatus("agent-a")).toBe("ready"));

  expect({ error, retryDirection: retry.request.direction }).toEqual({
    error: "timeline unavailable",
    retryDirection: "tail",
  });
  world.expectNoPendingMembership();
});

test("a failed catch-up retries with exponential backoff", async () => {
  const world = new TimelineWorld();
  world.sync.setConnected(true);
  world.sync.replaceVisibleAgentIds("workspace", ["agent-a"]);
  const membership = await world.nextMembership();
  membership.succeed();

  const first = await world.nextFetch("agent-a");
  first.fail("timeline unavailable");
  const [firstError, retryAfterFirstFailure] = await Promise.all([
    world.nextError(),
    world.nextRetry(),
  ]);
  expect(firstError).toBe("timeline unavailable");
  retryAfterFirstFailure();

  const second = await world.nextFetch("agent-a");
  second.fail("timeline unavailable");
  await world.nextError();

  const retryAfterSecondFailure = await world.nextRetry(2_000);
  expect(world.sync.getAgentTimelineStatus("agent-a")).toBe("error");

  retryAfterSecondFailure();
  const third = await world.nextFetch("agent-a");
  third.respond({ hasNewer: false });
  await vi.waitFor(() => expect(world.sync.getAgentTimelineStatus("agent-a")).toBe("ready"));
});

test("manual retries can immediately re-attempt a failed catch-up", async () => {
  const world = new TimelineWorld();
  world.sync.setConnected(true);
  world.sync.replaceVisibleAgentIds("workspace", ["agent-a"]);
  const membership = await world.nextMembership();
  membership.succeed();

  const failed = await world.nextFetch("agent-a");
  failed.fail("timeline unavailable");
  await world.nextRetry();
  expect(world.sync.getAgentTimelineStatus("agent-a")).toBe("error");

  world.sync.retryVisibleAgentTimeline("agent-a");
  expect(world.sync.getAgentTimelineStatus("agent-a")).toBe("retrying");

  const retry = await world.nextFetch("agent-a");
  retry.respond({ hasNewer: false });
  await vi.waitFor(() => expect(world.sync.getAgentTimelineStatus("agent-a")).toBe("ready"));
});

test("redeclaring unchanged visibility does not bypass catch-up backoff", async () => {
  const world = new TimelineWorld();
  world.sync.setConnected(true);
  world.sync.replaceVisibleAgentIds("workspace", ["agent-a"]);
  const membership = await world.nextMembership();
  membership.succeed();

  const failed = await world.nextFetch("agent-a");
  failed.fail("timeline unavailable");
  await world.nextRetry();

  world.sync.replaceVisibleAgentIds("workspace", ["agent-a"]);

  world.expectNoPendingMembership();
  world.expectNoPendingFetch();
  expect(world.sync.getAgentTimelineStatus("agent-a")).toBe("error");
});

test("a manual retry that fails returns to the error state", async () => {
  const world = new TimelineWorld();
  world.sync.setConnected(true);
  world.sync.replaceVisibleAgentIds("workspace", ["agent-a"]);
  const membership = await world.nextMembership();
  membership.succeed();

  const failed = await world.nextFetch("agent-a");
  failed.fail("timeline unavailable");
  await world.nextRetry();

  world.sync.retryVisibleAgentTimeline("agent-a");
  expect(world.sync.getAgentTimelineStatus("agent-a")).toBe("retrying");

  const retry = await world.nextFetch("agent-a");
  retry.fail("timeline still unavailable");
  await vi.waitFor(() => expect(world.sync.getAgentTimelineStatus("agent-a")).toBe("error"));
});

test("gap recovery supersedes completed catch-up and pages through the current tail", async () => {
  const world = new TimelineWorld();
  world.sync.setConnected(true);
  world.sync.replaceVisibleAgentIds("workspace", ["agent-a"]);
  const membership = await world.nextMembership();
  membership.succeed();
  const initial = await world.nextFetch("agent-a");
  initial.respond({ hasNewer: false });
  await vi.waitFor(() => expect(world.sync.getAgentTimelineStatus("agent-a")).toBe("ready"));

  world.sync.recoverGap("agent-a", { epoch: "epoch-agent-a", endSeq: 10 });
  const gapPage = await world.nextFetch("agent-a");
  gapPage.respond({ hasNewer: true, seq: 15 });
  const finalPage = await world.nextFetch("agent-a");
  finalPage.respond({ hasNewer: false });

  expect([gapPage.request, finalPage.request]).toEqual([
    {
      direction: "after",
      cursor: { epoch: "epoch-agent-a", seq: 10 },
      limit: 40,
      projection: "projected",
    },
    {
      direction: "after",
      cursor: { epoch: "epoch-agent-a", seq: 15 },
      limit: 40,
      projection: "projected",
    },
  ]);
});

test("repeated recovery for the same running gap reuses the in-flight fetch", async () => {
  const world = new TimelineWorld();
  world.sync.setConnected(true);
  world.sync.replaceVisibleAgentIds("workspace", ["agent-a"]);
  const membership = await world.nextMembership();
  membership.succeed();
  const initial = await world.nextFetch("agent-a");
  initial.respond({ hasNewer: false });
  await vi.waitFor(() => expect(world.sync.getAgentTimelineStatus("agent-a")).toBe("ready"));

  const cursor = { epoch: "epoch-agent-a", endSeq: 10 };
  world.sync.recoverGap("agent-a", cursor);
  const gapPage = await world.nextFetch("agent-a");
  world.sync.recoverGap("agent-a", cursor);

  world.expectNoPendingFetch();
  gapPage.respond({ hasNewer: false });
});

test("membership failure autonomously retries without another visibility declaration", async () => {
  const world = new TimelineWorld();
  world.sync.setConnected(true);
  world.sync.replaceVisibleAgentIds("workspace", ["agent-a"]);
  const failed = await world.nextMembership();
  failed.fail("subscription unavailable");
  const [error, retryMembership] = await Promise.all([world.nextError(), world.nextRetry()]);
  expect(world.sync.getAgentTimelineStatus("agent-a")).toBe("error");

  retryMembership();
  const retry = await world.nextMembership();
  retry.succeed();
  const catchUp = await world.nextFetch("agent-a");
  expect(world.sync.getAgentTimelineStatus("agent-a")).toBe("error");
  catchUp.respond({ hasNewer: false });
  await vi.waitFor(() => expect(world.sync.getAgentTimelineStatus("agent-a")).toBe("ready"));

  expect({ error, failed: failed.agentIds, retry: retry.agentIds }).toEqual({
    error: "subscription unavailable",
    failed: ["agent-a"],
    retry: ["agent-a"],
  });
});

test("membership failures retry with exponential backoff", async () => {
  const world = new TimelineWorld();
  world.sync.setConnected(true);
  world.sync.replaceVisibleAgentIds("workspace", ["agent-a"]);

  const first = await world.nextMembership();
  first.fail("subscription unavailable");
  const [firstError, retryAfterFirstFailure] = await Promise.all([
    world.nextError(),
    world.nextRetry(),
  ]);
  expect(firstError).toBe("subscription unavailable");

  retryAfterFirstFailure();
  const second = await world.nextMembership();
  second.fail("subscription unavailable again");

  const retryAfterSecondFailure = await world.nextRetry(2_000);
  expect(world.sync.getAgentTimelineStatus("agent-a")).toBe("error");

  retryAfterSecondFailure();
  const retry = await world.nextMembership();
  retry.succeed();
  const catchUp = await world.nextFetch("agent-a");
  catchUp.respond({ hasNewer: false });
  await vi.waitFor(() => expect(world.sync.getAgentTimelineStatus("agent-a")).toBe("ready"));
});

test("redeclaring unchanged visibility does not bypass membership backoff", async () => {
  const world = new TimelineWorld();
  world.sync.setConnected(true);
  world.sync.replaceVisibleAgentIds("workspace", ["agent-a"]);

  const failed = await world.nextMembership();
  failed.fail("subscription unavailable");
  await world.nextRetry();

  world.sync.replaceVisibleAgentIds("workspace", ["agent-a"]);

  world.expectNoPendingMembership();
  world.expectNoPendingFetch();
  expect(world.sync.getAgentTimelineStatus("agent-a")).toBe("error");
});

test("foreground recovery skips to the latest tail after one missed page", async () => {
  const world = new TimelineWorld();
  world.sync.setConnected(true);
  world.show("workspace", ["agent-a"]);
  (await world.nextMembership()).succeed();
  (await world.nextFetch("agent-a")).respond({ hasNewer: false });
  await vi.waitFor(() => expect(world.sync.getAgentTimelineStatus("agent-a")).toBe("ready"));
  world.cursors.set("agent-a", { epoch: "epoch-agent-a", endSeq: 42 });

  world.sync.setActive(false);
  world.sync.setActive(true);
  const probe = await world.nextFetch("agent-a");
  expect(probe.request.direction).toBe("after");
  probe.respond({ hasNewer: true, seq: 82 });
  const latest = await world.nextFetch("agent-a");
  expect(latest.request.direction).toBe("tail");
  expect(world.sync.getAgentTimelineStatus("agent-a")).toBe("pending");
  latest.respond({ hasNewer: false });
  await vi.waitFor(() => expect(world.sync.getAgentTimelineStatus("agent-a")).toBe("ready"));
});

test("backgrounding preserves subscriptions and returning verifies the visible chat", async () => {
  const world = new TimelineWorld();
  world.sync.setConnected(true);
  world.show("workspace", ["agent-a"]);
  const agentAMembership = await world.nextMembership();
  agentAMembership.succeed();
  const agentACatchUp = await world.nextFetch("agent-a");
  agentACatchUp.respond({ hasNewer: false });

  world.show("workspace", ["agent-b"]);
  const agentBMembership = await world.nextMembership();
  agentBMembership.succeed();
  const agentBCatchUp = await world.nextFetch("agent-b");
  agentBCatchUp.respond({ hasNewer: false });
  await vi.waitFor(() => expect(world.sync.getAgentTimelineStatus("agent-b")).toBe("ready"));

  world.sync.setActive(false);
  world.expectNoPendingMembership();
  world.elapse(60_000);
  world.sync.setActive(true);

  world.expectNoPendingMembership();
  expect(world.sync.getAgentTimelineStatus("agent-b")).toBe("pending");
  const resume = await world.nextFetch("agent-b");
  resume.respond({ hasNewer: false });
  await vi.waitFor(() => expect(world.sync.getAgentTimelineStatus("agent-b")).toBe("ready"));
  world.expectNoPendingFetch();
});

test("stale membership retry cannot overwrite a newer effective set", async () => {
  const world = new TimelineWorld();
  world.sync.setConnected(true);
  world.show("workspace", ["agent-a"]);
  const failed = await world.nextMembership();
  failed.fail("subscription unavailable");
  const staleRetry = await world.nextRetry();

  world.show("workspace", ["agent-b"]);
  const current = await world.nextMembership();
  staleRetry();
  current.succeed();
  const agentB = await world.nextFetch("agent-b");
  agentB.respond({ hasNewer: false });
  const agentA = await world.nextFetch("agent-a");
  agentA.respond({ hasNewer: false });

  expect(current.agentIds).toEqual(["agent-a", "agent-b"]);
  world.expectNoPendingMembership();
});

test("membership retry cannot run while disconnected", async () => {
  const world = new TimelineWorld();
  world.sync.setConnected(true);
  world.sync.replaceVisibleAgentIds("workspace", ["agent-a"]);
  const failed = await world.nextMembership();
  failed.fail("subscription unavailable");
  const disconnectedRetry = await world.nextRetry();

  world.sync.setConnected(false);
  disconnectedRetry();
  world.expectNoPendingMembership();
  world.sync.setConnected(true);
  const restored = await world.nextMembership();
  restored.succeed();
  const catchUp = await world.nextFetch("agent-a");
  catchUp.respond({ hasNewer: false });

  expect(restored.agentIds).toEqual(["agent-a"]);
});

test("returning to an open hidden agent stays live after inactivity", async () => {
  const world = new TimelineWorld();
  world.sync.setConnected(true);
  world.show("workspace", ["agent-a"]);
  const initialMembership = await world.nextMembership();
  initialMembership.succeed();
  const agentACatchUp = await world.nextFetch("agent-a");
  agentACatchUp.respond({ hasNewer: false });

  world.show("workspace", ["agent-b"]);
  const expandedMembership = await world.nextMembership();
  expandedMembership.succeed();
  const agentBCatchUp = await world.nextFetch("agent-b");
  agentBCatchUp.respond({ hasNewer: false });
  await vi.waitFor(() => expect(world.sync.getAgentTimelineStatus("agent-b")).toBe("ready"));

  world.elapse(60_000);
  world.show("workspace", ["agent-a"]);

  world.expectNoPendingMembership();
  world.expectNoPendingFetch();
});

test("visible agents stay subscribed even before open-tab membership arrives", async () => {
  const world = new TimelineWorld();
  const visible = ["agent-a", "agent-b", "agent-c", "agent-d", "agent-e", "agent-f"];
  world.sync.setConnected(true);
  world.sync.replaceVisibleAgentIds("workspace", visible);
  const membership = await world.nextMembership();
  membership.succeed();
  const catchUps = await Promise.all(visible.map((agentId) => world.nextFetch(agentId)));
  for (const catchUp of catchUps) catchUp.respond({ hasNewer: false });

  expect(membership.agentIds).toEqual(visible);

  // Hiding a pane is not closing its chat, so the sixth stays subscribed.
  world.sync.replaceVisibleAgentIds("workspace", visible.slice(0, 5));
  world.expectNoPendingMembership();

  // Closing its tab is what releases it.
  world.sync.replaceOpenTabAgentIds(visible.slice(0, 5));
  const closed = await world.nextMembership();
  closed.succeed();
  expect(closed.agentIds).toEqual(visible.slice(0, 5));
});

test("reconnect restores open demand and reopening a closed chat fetches again", async () => {
  const world = new TimelineWorld();
  world.sync.setConnected(true);
  world.sync.replaceVisibleAgentIds("workspace", ["agent-a"]);
  const agentAMembership = await world.nextMembership();
  agentAMembership.succeed();
  (await world.nextFetch("agent-a")).respond({ hasNewer: false });

  // Close agent-a's tab, then open agent-b.
  world.sync.replaceOpenTabAgentIds(["agent-b"]);
  world.sync.replaceVisibleAgentIds("workspace", ["agent-b"]);
  const agentBMembership = await world.nextMembership();
  agentBMembership.succeed();
  (await world.nextFetch("agent-b")).respond({ hasNewer: false });
  await vi.waitFor(() => expect(world.sync.getAgentTimelineStatus("agent-b")).toBe("ready"));

  world.sync.setConnected(false);
  world.expectNoPendingMembership();
  world.sync.setConnected(true);
  const restored = await world.nextMembership();
  restored.succeed();
  (await world.nextFetch("agent-b")).respond({ hasNewer: false });
  expect(restored.agentIds).toEqual(["agent-b"]);

  world.sync.replaceVisibleAgentIds("workspace", ["agent-a"]);
  const reopened = await world.nextMembership();
  reopened.succeed();
  (await world.nextFetch("agent-a")).respond({ hasNewer: false });
  expect(reopened.agentIds).toEqual(["agent-a", "agent-b"]);
});

test("disposing a view releases its pending observation before bootstrap completes", async () => {
  const world = new TimelineWorld();
  world.sync.replaceVisibleAgentIds("workspace", ["agent-a"]);
  world.sync.setConnected(true);
  const request = await world.nextMembership();
  world.sync.dispose();
  expect(world.releasedMemberships).toEqual([["agent-a"]]);
  request.succeed();
  await Promise.resolve();
  world.expectNoPendingFetch();
});

const threadIds = (count: number, prefix = "thread") =>
  Array.from({ length: count }, (_, index) => `${prefix}-${String(index + 1).padStart(2, "0")}`);

test("opening 12 threads in turn keeps stream items for the 8 most recent", async () => {
  const world = new TimelineWorld();
  const threads = threadIds(12);
  await world.openInTurn("pane", threads);

  expect(RECENT_STREAM_LIMIT).toBe(8);
  expect([...world.heldStreams].sort()).toEqual(threads.slice(4));
  expect(world.evictions).toEqual(threads.slice(0, 4));
  world.sync.dispose();
});

test("a thread visible in a split pane is never evicted, even when it is the oldest", async () => {
  const world = new TimelineWorld();
  await world.openInTurn("left", ["pinned"]);
  await world.openInTurn("right", threadIds(12));

  expect(world.evictions).not.toContain("pinned");
  expect(world.heldStreams.has("pinned")).toBe(true);
  expect(world.heldStreams.size).toBe(8);
  world.sync.dispose();
});

test("each host keeps its own cap of 8 threads", async () => {
  const first = new TimelineWorld();
  const second = new TimelineWorld();
  const threads = threadIds(12);
  for (const agentId of threads) {
    await first.openInTurn("pane", [agentId]);
    await second.openInTurn("pane", [agentId]);
  }

  expect([...first.heldStreams].sort()).toEqual(threads.slice(4));
  expect([...second.heldStreams].sort()).toEqual(threads.slice(4));
  first.sync.dispose();
  second.sync.dispose();
});

test("an evicted hidden thread waits for its next view before it catches up", async () => {
  const world = new TimelineWorld();
  world.sync.setConnected(true);
  world.show("pane", ["thread-01"]);
  (await world.nextMembership()).succeed();
  (await world.nextFetch("thread-01")).respond({ hasNewer: false });
  await vi.waitFor(() => expect(world.sync.getAgentTimelineStatus("thread-01")).toBe("ready"));
  world.heldStreams.add("thread-01");

  for (const agentId of threadIds(8).slice(1).concat("thread-09")) {
    world.show("pane", [agentId]);
    (await world.nextMembership()).succeed();
    (await world.nextFetch(agentId)).respond({ hasNewer: false });
    await vi.waitFor(() => expect(world.sync.getAgentTimelineStatus(agentId)).toBe("ready"));
    world.heldStreams.add(agentId);
  }
  world.show("pane", ["thread-10"]);
  (await world.nextMembership()).succeed();
  expect(world.evictions).toEqual(["thread-01", "thread-02"]);

  world.sync.recoverGap("thread-01", { epoch: "epoch-thread-01", endSeq: 1 });
  (await world.nextFetch("thread-10")).respond({ hasNewer: false });
  await vi.waitFor(() => expect(world.sync.getAgentTimelineStatus("thread-10")).toBe("ready"));
  world.expectNoPendingFetch();
  expect(world.sync.getAgentTimelineStatus("thread-01")).toBe("pending");

  const cacheReads = world.cacheRequests.filter((agentId) => agentId === "thread-01").length;
  world.cursors.set("thread-01", { epoch: "epoch-thread-01", endSeq: 1 });
  world.show("pane", ["thread-01"]);
  const reopened = await world.nextFetch("thread-01");
  expect(world.cacheRequests.filter((agentId) => agentId === "thread-01")).toHaveLength(
    cacheReads + 1,
  );
  expect(reopened.request).toEqual({
    direction: "after",
    cursor: { epoch: "epoch-thread-01", seq: 1 },
    limit: 40,
    projection: "projected",
  });
  reopened.respond({ hasNewer: false });
  await vi.waitFor(() => expect(world.sync.getAgentTimelineStatus("thread-01")).toBe("ready"));
  world.expectNoPendingFetch();
  world.sync.dispose();
});

describe("reopening an evicted thread through the owner", () => {
  const SERVER_ID = "recent-stream-cap-host";
  const EPOCH = "epoch-1";

  afterEach(() => useSessionStore.getState().clearSession(SERVER_ID));

  function row(agentId: string, seq: number): string {
    return `${agentId} row ${seq}`;
  }

  function page(
    agentId: string,
    rows: string[],
    direction: "tail" | "after",
    afterSeq: number,
  ): TimelineResponsePayload {
    const entries = rows.slice(afterSeq).map((text, index) => {
      const seq = afterSeq + index + 1;
      return {
        provider: "codex" as const,
        item: { type: "user_message" as const, text, messageId: `${agentId}-${seq}` },
        timestamp: new Date(Date.UTC(2026, 9, 15, 0, 0, seq)).toISOString(),
        seqStart: seq,
        seqEnd: seq,
        sourceSeqRanges: [{ startSeq: seq, endSeq: seq }],
        collapsed: [],
      };
    });
    return {
      requestId: `${agentId}-${direction}-${afterSeq}`,
      agentId,
      agent: null,
      direction,
      projection: "projected",
      reset: false,
      epoch: EPOCH,
      window: { minSeq: 1, maxSeq: rows.length, nextSeq: rows.length + 1 },
      startCursor: entries.length > 0 ? { epoch: EPOCH, seq: afterSeq + 1 } : null,
      endCursor: entries.length > 0 ? { epoch: EPOCH, seq: rows.length } : null,
      entries,
      error: null,
      hasNewer: false,
      hasOlder: false,
      staleCursor: false,
      gap: false,
    };
  }

  function liveRow(agentId: string, seq: number): AgentStreamReducerEvent {
    return {
      event: {
        type: "timeline",
        provider: "codex",
        item: { type: "user_message", text: row(agentId, seq), messageId: `${agentId}-${seq}` },
      } as AgentStreamEventPayload,
      seq,
      epoch: EPOCH,
      timestamp: new Date(Date.UTC(2026, 9, 15, 0, 0, seq)),
    };
  }

  function renderedRows(agentId: string): string[] {
    const session = useSessionStore.getState().sessions[SERVER_ID];
    return [
      ...(session?.agentStreamTail.get(agentId) ?? []),
      ...(session?.agentStreamHead.get(agentId) ?? []),
    ].map((item) => (item.kind === "user_message" ? item.text : item.kind));
  }

  const timelineStatus = (agentId: string) =>
    selectAgentTimelineState(useSessionStore.getState().sessions[SERVER_ID], agentId).status;

  test("paints from the cache, issues one fetch, then applies live rows in order", async () => {
    useSessionStore.getState().initializeSession(SERVER_ID, null);
    const daemonRows = new Map<string, string[]>();
    const stored = new Map<string, CachedTimeline>();
    const fetches: Array<{ agentId: string; request: ProjectedTimelineForwardFetchPlan }> = [];
    const errors: unknown[] = [];
    let gate: { agentId: string; release: Deferred<void> } | null = null;
    const replica = createTimelineReplica({
      serverId: SERVER_ID,
      storage: {
        readTimeline: async (_serverId, agentId) => stored.get(agentId),
        commitTimeline: (_serverId, agentId, timeline) => stored.set(agentId, timeline),
      },
      prepareAgent: async () => undefined,
    });
    const owner = createViewedTimelineOwner({
      serverId: SERVER_ID,
      replica,
      replaceDemandedAgentIds: () => undefined,
      drainQueuedAgentMessage: () => undefined,
      ports: {
        observe: () => ({ ready: Promise.resolve(), release: async () => undefined }),
        readCursor: () => undefined,
        fetchPage: async (agentId, request) => {
          fetches.push({ agentId, request });
          if (gate?.agentId === agentId) await gate.release.promise;
          const rows = daemonRows.get(agentId) ?? [];
          const afterSeq = request.direction === "after" ? request.cursor.seq : 0;
          owner.applyTimelineResponse(page(agentId, rows, request.direction, afterSeq));
          return { hasNewer: false, endCursor: { epoch: EPOCH, seq: rows.length } };
        },
        fetchLatestTail: async () => {
          throw new Error("unexpected latest-tail fallback");
        },
        reportError: (error) => errors.push(error),
        schedule: () => () => undefined,
      },
    });
    owner.setConnected(true);

    const threads = threadIds(9);
    for (const agentId of threads) {
      daemonRows.set(
        agentId,
        [1, 2, 3].map((seq) => row(agentId, seq)),
      );
      owner.replaceVisibleAgentIds("pane", [agentId]);
      await vi.waitFor(() => expect(timelineStatus(agentId)).toBe("synced"));
    }
    const first = threads[0];
    expect(timelineStatus(first)).toBe("cold");
    expect(stored.get(first)?.range).toEqual({ epoch: EPOCH, startSeq: 1, endSeq: 3 });

    // Rows the daemon streams while the thread is evicted wait for the reopen catch-up.
    daemonRows.get(first)?.push(row(first, 4), row(first, 5));
    owner.enqueueStreamEvent(first, liveRow(first, 4));
    owner.enqueueStreamEvent(first, liveRow(first, 5));
    owner.flushStreamAgent(first);
    expect(renderedRows(first)).toEqual([]);

    fetches.length = 0;
    gate = { agentId: first, release: deferred<void>() };
    owner.replaceVisibleAgentIds("pane", [first]);
    await vi.waitFor(() => expect(timelineStatus(first)).toBe("painted"));
    expect(renderedRows(first)).toEqual([1, 2, 3].map((seq) => row(first, seq)));

    gate.release.resolve();
    await vi.waitFor(() => expect(timelineStatus(first)).toBe("synced"));
    await vi.waitFor(() => expect(owner.getAgentTimelineStatus(first)).toBe("ready"));
    expect(fetches).toEqual([
      {
        agentId: first,
        request: {
          direction: "after",
          cursor: { epoch: EPOCH, seq: 3 },
          limit: 40,
          projection: "projected",
        },
      },
    ]);
    expect(renderedRows(first)).toEqual([1, 2, 3, 4, 5].map((seq) => row(first, seq)));

    owner.enqueueStreamEvent(first, liveRow(first, 6));
    owner.flushStreamAgent(first);
    expect(renderedRows(first)).toEqual([1, 2, 3, 4, 5, 6].map((seq) => row(first, seq)));

    const session = useSessionStore.getState().sessions[SERVER_ID];
    const held = new Set(session?.agentStreamTail.keys() ?? []);
    expect(held.size).toBe(8);
    expect(held.has(threads[1])).toBe(false);
    expect(errors).toEqual([]);
    owner.dispose();
  });
});
