import { afterEach, describe, expect, it, vi } from "vitest";
import { registerTimelinePassageRevealer, requestTimelinePassage } from "./timeline-reveal";

const cleanups: Array<() => void> = [];

afterEach(() => {
  for (const cleanup of cleanups.splice(0)) cleanup();
  vi.useRealTimers();
});

function register(agentId: string, revealer = vi.fn()) {
  cleanups.push(registerTimelinePassageRevealer("host", agentId, revealer));
  return revealer;
}

describe("timeline passage reveal", () => {
  it("reveals in the latest mounted stream of that agent only", () => {
    const first = register("agent-1");
    const latest = register("agent-1");
    const other = register("agent-2");

    requestTimelinePassage("host", "agent-1", { messageId: "m1", text: "quote" });

    expect(latest).toHaveBeenCalledWith({ messageId: "m1", text: "quote" });
    expect(first).not.toHaveBeenCalled();
    expect(other).not.toHaveBeenCalled();
  });

  it("hands a request made before the stream mounts to that stream once", () => {
    requestTimelinePassage("host", "agent-3", { messageId: "m1" });
    const revealer = register("agent-3");
    const next = register("agent-3");

    expect(revealer).toHaveBeenCalledTimes(1);
    expect(next).not.toHaveBeenCalled();
  });

  it("drops a held request that waited too long", () => {
    vi.useFakeTimers();
    requestTimelinePassage("host", "agent-4", { messageId: "m1" });
    vi.advanceTimersByTime(10_001);

    expect(register("agent-4")).not.toHaveBeenCalled();
  });

  it("falls back to the earlier stream after the latest unmounts", () => {
    const first = register("agent-5");
    const unregister = registerTimelinePassageRevealer("host", "agent-5", vi.fn());
    unregister();

    requestTimelinePassage("host", "agent-5", { messageId: "m1" });

    expect(first).toHaveBeenCalledTimes(1);
  });
});
