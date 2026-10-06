// @vitest-environment jsdom

import { renderHook } from "@testing-library/react";
import { createRef } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { StreamItem } from "@/types/stream";
import type { StreamViewportHandle } from "../strategy";
import type { TimelinePassageRevealProps } from "./types";

const registry = vi.hoisted(() => ({ registrations: 0 }));

vi.mock("react-i18next", () => ({ useTranslation: () => ({ t: (key: string) => key }) }));
vi.mock("@/plugins/timeline-reveal", async (importActual) => {
  const actual = await importActual<typeof import("@/plugins/timeline-reveal")>();
  return {
    ...actual,
    registerTimelinePassageRevealer: (
      ...args: Parameters<typeof actual.registerTimelinePassageRevealer>
    ) => {
      registry.registrations += 1;
      return actual.registerTimelinePassageRevealer(...args);
    },
  };
});

const { requestTimelinePassage } = await import("@/plugins/timeline-reveal");
const { usePassageReveal } = await import("./use-passage-reveal");

function userMessage(id: string): StreamItem {
  return { kind: "user_message", id, text: id, timestamp: new Date(0) };
}

function props(overrides: Partial<TimelinePassageRevealProps>): TimelinePassageRevealProps {
  return {
    serverId: "host",
    agentId: "agent-1",
    items: [],
    historyReady: true,
    viewportRef: createRef<StreamViewportHandle | null>(),
    revealLoadedMessage: () => false,
    visibleMessageIds: new Set(),
    toast: { show: vi.fn(), error: vi.fn() } as unknown as TimelinePassageRevealProps["toast"],
    ...overrides,
  };
}

afterEach(() => {
  registry.registrations = 0;
});

describe("usePassageReveal", () => {
  it("holds a request until the stream's history is ready, then reveals it", () => {
    const run = vi.fn();
    const initial = props({ agentId: "cold", historyReady: false });
    const { rerender, unmount } = renderHook((current) => usePassageReveal(current, run), {
      initialProps: initial,
    });

    requestTimelinePassage("host", "cold", { messageId: "m1", text: "quote" });
    expect(run).not.toHaveBeenCalled();
    expect(initial.toast?.show).not.toHaveBeenCalled();

    rerender({ ...initial, historyReady: true, items: [userMessage("m1")] });
    expect(run).toHaveBeenCalledTimes(1);
    expect(run.mock.calls[0]?.[0]).toEqual({ messageId: "m1", text: "quote" });
    unmount();
  });

  it("shows a toast and reveals nothing for a message that is not loaded", () => {
    const run = vi.fn();
    const current = props({ agentId: "missing", items: [userMessage("m1")] });
    const { unmount } = renderHook(() => usePassageReveal(current, run));

    requestTimelinePassage("host", "missing", { messageId: "unknown" });

    expect(run).not.toHaveBeenCalled();
    expect(current.toast?.show).toHaveBeenCalledWith("agentStream.passageNotLoaded");
    unmount();
  });

  it("registers once per stream however often the timeline re-renders", () => {
    const initial = props({ agentId: "busy" });
    const { rerender, unmount } = renderHook((current) => usePassageReveal(current, vi.fn()), {
      initialProps: initial,
    });
    for (let index = 0; index < 5; index += 1) {
      rerender({ ...initial, items: [userMessage(`m${index}`)] });
    }

    expect(registry.registrations).toBe(1);
    unmount();
  });
});
