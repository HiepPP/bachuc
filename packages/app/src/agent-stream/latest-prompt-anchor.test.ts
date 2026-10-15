import { afterEach, describe, expect, it, vi } from "vitest";
import type { StreamItem } from "@/types/stream";
import {
  beginLatestPromptAnchor,
  cancelLatestPromptAnchor,
  findLatestPromptId,
  getLatestPromptAnchorVersion,
  hasPendingLatestPromptAnchor,
  requestLatestPromptAnchor,
  settleLatestPromptAnchor,
  subscribeLatestPromptAnchor,
} from "./latest-prompt-anchor";

const item = (id: string, kind: string) => ({ id, kind }) as StreamItem;

describe("latest prompt anchor", () => {
  afterEach(() => {
    settleLatestPromptAnchor("agent-a");
    settleLatestPromptAnchor("agent-b");
    vi.useRealTimers();
  });

  it("begins once, only for the requested agent, and stays pending until settled", () => {
    requestLatestPromptAnchor("agent-a", 1000);
    expect(beginLatestPromptAnchor("agent-b", 1000)).toBeNull();
    const requestId = beginLatestPromptAnchor("agent-a", 1000);
    expect(requestId).not.toBeNull();
    expect(beginLatestPromptAnchor("agent-a", 1000)).toBeNull();
    expect(hasPendingLatestPromptAnchor("agent-a", 1000)).toBe(true);
    expect(hasPendingLatestPromptAnchor("agent-b", 1000)).toBe(false);
    settleLatestPromptAnchor("agent-a", requestId ?? undefined);
    expect(hasPendingLatestPromptAnchor("agent-a", 1000)).toBe(false);
  });

  it("stays pending past 5 s while it has not landed", () => {
    requestLatestPromptAnchor("agent-a", 1000);
    beginLatestPromptAnchor("agent-a", 1000);
    expect(hasPendingLatestPromptAnchor("agent-a", 6001)).toBe(true);
    expect(hasPendingLatestPromptAnchor("agent-a", 20_000)).toBe(true);
  });

  it("ends when the user scrolls during the jump", () => {
    requestLatestPromptAnchor("agent-a", 1000);
    const requestId = beginLatestPromptAnchor("agent-a", 1000);
    // The settle controller reports a cancel from user intent through the same call.
    settleLatestPromptAnchor("agent-a", requestId ?? undefined);
    expect(hasPendingLatestPromptAnchor("agent-a", 1000)).toBe(false);
  });

  it("ends at the 30 s cap and notifies subscribers", () => {
    vi.useFakeTimers();
    let calls = 0;
    const unsubscribe = subscribeLatestPromptAnchor(() => {
      calls += 1;
    });
    requestLatestPromptAnchor("agent-a");
    expect(calls).toBe(1);
    vi.advanceTimersByTime(29_999);
    expect(hasPendingLatestPromptAnchor("agent-a")).toBe(true);
    vi.advanceTimersByTime(1);
    expect(hasPendingLatestPromptAnchor("agent-a")).toBe(false);
    expect(calls).toBe(2);
    unsubscribe();
  });

  it("is replaced by a new request, and the old jump does not end the new one", () => {
    requestLatestPromptAnchor("agent-a", 1000);
    const oldRequestId = beginLatestPromptAnchor("agent-a", 1000);
    requestLatestPromptAnchor("agent-a", 2000);
    expect(beginLatestPromptAnchor("agent-a", 2000)).not.toBeNull();
    settleLatestPromptAnchor("agent-a", oldRequestId ?? undefined);
    expect(hasPendingLatestPromptAnchor("agent-a", 2000)).toBe(true);

    requestLatestPromptAnchor("agent-b", 3000);
    expect(hasPendingLatestPromptAnchor("agent-a", 3000)).toBe(false);
    expect(hasPendingLatestPromptAnchor("agent-b", 3000)).toBe(true);
  });

  it("ends on cancel, notifying only when a request was pending", () => {
    let calls = 0;
    const unsubscribe = subscribeLatestPromptAnchor(() => {
      calls += 1;
    });
    requestLatestPromptAnchor("agent-a", 1000);
    beginLatestPromptAnchor("agent-a", 1000);
    cancelLatestPromptAnchor();
    expect(hasPendingLatestPromptAnchor("agent-a", 1000)).toBe(false);
    expect(beginLatestPromptAnchor("agent-a", 1000)).toBeNull();
    cancelLatestPromptAnchor();
    expect(calls).toBe(2);
    unsubscribe();
  });

  it("notifies subscribers of a new request", () => {
    let calls = 0;
    const unsubscribe = subscribeLatestPromptAnchor(() => {
      calls += 1;
    });
    const before = getLatestPromptAnchorVersion();
    requestLatestPromptAnchor("agent-a", 1000);
    unsubscribe();
    requestLatestPromptAnchor("agent-a", 1000);
    expect(calls).toBe(1);
    expect(getLatestPromptAnchorVersion()).toBe(before + 2);
  });

  it("finds the last user message across segments, oldest segment first", () => {
    expect(
      findLatestPromptId([
        [item("u1", "user_message"), item("a1", "assistant_message")],
        [item("u2", "user_message"), item("a2", "assistant_message")],
        [item("a3", "assistant_message")],
      ]),
    ).toBe("u2");
    expect(findLatestPromptId([[item("a1", "assistant_message")], []])).toBeNull();
  });
});
