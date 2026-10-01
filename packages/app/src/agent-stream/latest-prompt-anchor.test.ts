import { describe, expect, it } from "vitest";
import type { StreamItem } from "@/types/stream";
import {
  extendLatestPromptAnchor,
  findLatestPromptId,
  getLatestPromptAnchorVersion,
  hasPendingLatestPromptAnchor,
  requestLatestPromptAnchor,
  subscribeLatestPromptAnchor,
  takeLatestPromptAnchor,
} from "./latest-prompt-anchor";

const item = (id: string, kind: string) => ({ id, kind }) as StreamItem;

describe("latest prompt anchor", () => {
  it("is taken once, only by the requested agent", () => {
    requestLatestPromptAnchor("agent-a", 1000);
    expect(takeLatestPromptAnchor("agent-b", 1000)).toBe(false);
    expect(takeLatestPromptAnchor("agent-a", 1000)).toBe(true);
    expect(takeLatestPromptAnchor("agent-a", 1000)).toBe(false);
  });

  it("expires", () => {
    requestLatestPromptAnchor("agent-a", 1000);
    expect(takeLatestPromptAnchor("agent-a", 6001)).toBe(false);
  });

  it("stays pending until expiry, and an extension restarts the expiry", () => {
    requestLatestPromptAnchor("agent-a", 1000);
    expect(hasPendingLatestPromptAnchor("agent-a", 6000)).toBe(true);
    expect(hasPendingLatestPromptAnchor("agent-b", 6000)).toBe(false);
    expect(hasPendingLatestPromptAnchor("agent-a", 6001)).toBe(false);
    extendLatestPromptAnchor("agent-a", 7000);
    expect(hasPendingLatestPromptAnchor("agent-a", 12000)).toBe(true);
    expect(takeLatestPromptAnchor("agent-a", 12000)).toBe(true);
    extendLatestPromptAnchor("agent-a", 12000);
    expect(hasPendingLatestPromptAnchor("agent-a", 12000)).toBe(false);
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
    takeLatestPromptAnchor("agent-a", 1000);
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
