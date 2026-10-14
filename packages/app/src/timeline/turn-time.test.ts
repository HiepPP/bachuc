import assert from "node:assert/strict";
import { describe, it } from "vitest";
import {
  buildComposerTurnMetricParts,
  buildTurnMetricParts,
  deriveComposerTurnMetrics,
  deriveStreamTurnTiming,
  formatComposerTurnMetricsLabel,
  formatTurnMetricsLabel,
} from "./turn-time";
import type { StreamItem } from "@/types/stream";

describe("turn metric tones", () => {
  const tones = (ttftMs: number, outputTokensPerSecond: number) =>
    buildTurnMetricParts({ ttftMs, outputTokensPerSecond }).map((part) => part.tone);

  it("rates a lower TTFT and a higher output rate as better", () => {
    assert.deepEqual(tones(4_999, 80), ["good", "good"]);
    assert.deepEqual(tones(5_000, 79), ["fair", "fair"]);
    assert.deepEqual(tones(14_999, 40), ["fair", "fair"]);
    assert.deepEqual(tones(15_000, 39), ["poor", "poor"]);
  });

  it("colors the counting TTFT by the time already waited", () => {
    assert.deepEqual(buildComposerTurnMetricParts({ kind: "waiting", elapsedMs: 16_000 }), [
      { text: "TTFT 16s…", tone: "poor" },
    ]);
  });
});

function user(id: string, timestamp: Date): StreamItem {
  return {
    kind: "user_message",
    id,
    text: id,
    timestamp,
  };
}

function assistant(
  id: string,
  timestamp: Date,
): Extract<StreamItem, { kind: "assistant_message" }> {
  return {
    kind: "assistant_message",
    id,
    text: id,
    timestamp,
  };
}

describe("deriveStreamTurnTiming", () => {
  it("starts elapsed time from the submitted prompt", () => {
    const submittedAt = new Date("2026-05-15T00:00:00.000Z");

    const timing = deriveStreamTurnTiming({
      isTurnActive: true,
      activeTurnStartedAt: submittedAt,
      tail: [],
      head: [user("submitted", submittedAt)],
    });

    assert.equal(timing.runningStartedAt, submittedAt);
  });

  it("uses the last user message as the running turn start", () => {
    const firstUserAt = new Date("2026-05-15T00:00:00.000Z");
    const secondUserAt = new Date("2026-05-15T00:01:00.000Z");

    const timing = deriveStreamTurnTiming({
      isTurnActive: true,
      activeTurnStartedAt: secondUserAt,
      tail: [
        user("u1", firstUserAt),
        assistant("a1", new Date("2026-05-15T00:00:05.000Z")),
        user("u2", secondUserAt),
      ],
      head: [assistant("a2", new Date("2026-05-15T00:01:04.000Z"))],
    });

    assert.equal(timing.runningStartedAt, secondUserAt);
    assert.equal(timing.byAssistantId.has("a2"), false);
  });

  it("derives completed turn timing from user and assistant item timestamps", () => {
    const userAt = new Date("2026-05-15T00:00:00.000Z");
    const assistantAt = new Date("2026-05-15T00:00:07.000Z");

    const timing = deriveStreamTurnTiming({
      isTurnActive: false,
      activeTurnStartedAt: null,
      tail: [
        user("u1", userAt),
        assistant("a1", assistantAt),
        user("u2", new Date("2026-05-15T00:01:00.000Z")),
      ],
      head: [],
    });

    assert.deepEqual(timing.byAssistantId.get("a1"), {
      completedAt: assistantAt,
      durationMs: 7000,
    });
  });

  it("maps multiple assistant chunks in one turn to the same timing", () => {
    const userAt = new Date("2026-05-15T00:00:00.000Z");
    const firstAssistantAt = new Date("2026-05-15T00:00:03.000Z");
    const lastAssistantAt = new Date("2026-05-15T00:00:07.000Z");

    const timing = deriveStreamTurnTiming({
      isTurnActive: false,
      activeTurnStartedAt: null,
      tail: [
        user("u1", userAt),
        assistant("a1", firstAssistantAt),
        assistant("a2", lastAssistantAt),
      ],
      head: [],
    });

    const expected = {
      completedAt: lastAssistantAt,
      durationMs: 7000,
    };
    assert.deepEqual(timing.byAssistantId.get("a1"), expected);
    assert.deepEqual(timing.byAssistantId.get("a2"), expected);
  });

  it("carries turn metrics from any assistant message to the whole turn", () => {
    const userAt = new Date("2026-05-15T00:00:00.000Z");
    const lastAssistantAt = new Date("2026-05-15T00:00:07.000Z");
    const metrics = { ttftMs: 1200, outputTokens: 300, outputTokensPerSecond: 50 };

    const timing = deriveStreamTurnTiming({
      isTurnActive: false,
      activeTurnStartedAt: null,
      tail: [
        user("u1", userAt),
        assistant("a1", new Date("2026-05-15T00:00:03.000Z")),
        { ...assistant("a2", lastAssistantAt), turnMetrics: metrics },
        user("u2", new Date("2026-05-15T00:01:00.000Z")),
        assistant("a3", new Date("2026-05-15T00:01:02.000Z")),
      ],
      head: [],
    });

    const expected = { completedAt: lastAssistantAt, durationMs: 7000, metrics };
    assert.deepEqual(timing.byAssistantId.get("a1"), expected);
    assert.deepEqual(timing.byAssistantId.get("a2"), expected);
    assert.equal(timing.byAssistantId.get("a3")?.metrics, undefined);
  });

  it("preserves the completion timestamp when a canonical turn has no visible prompt", () => {
    const firstTurnAt = new Date("2026-05-15T00:00:00.000Z");
    const hiddenPromptTurnAt = new Date("2026-05-15T00:01:07.000Z");
    const timing = deriveStreamTurnTiming({
      isTurnActive: false,
      activeTurnStartedAt: null,
      tail: [
        { ...user("u1", firstTurnAt), turnId: "turn-1" },
        {
          ...assistant("a1", new Date("2026-05-15T00:00:07.000Z")),
          turnId: "turn-1",
        },
        {
          ...assistant("hidden-prompt-a1", new Date("2026-05-15T00:01:03.000Z")),
          turnId: "turn-2",
        },
        { ...assistant("hidden-prompt-a2", hiddenPromptTurnAt), turnId: "turn-2" },
      ],
      head: [],
    });

    assert.deepEqual(timing.byAssistantId.get("hidden-prompt-a2"), {
      completedAt: hiddenPromptTurnAt,
      durationMs: null,
    });
  });
});

describe("formatTurnMetricsLabel", () => {
  it("formats TTFT and output rate", () => {
    assert.equal(
      formatTurnMetricsLabel({ ttftMs: 1234, outputTokens: 900, outputTokensPerSecond: 45.6 }),
      "TTFT 1.2s · 46 tok/s",
    );
    assert.equal(
      formatTurnMetricsLabel({ ttftMs: 850, outputTokensPerSecond: 7.25 }),
      "TTFT 850ms · 7.3 tok/s",
    );
  });

  it("shows only TTFT when the provider reports no per-turn tokens", () => {
    assert.equal(formatTurnMetricsLabel({ ttftMs: 2000 }), "TTFT 2.0s");
  });

  it("is empty without metrics", () => {
    assert.equal(formatTurnMetricsLabel(undefined), "");
    assert.equal(formatTurnMetricsLabel({}), "");
  });
});

describe("deriveComposerTurnMetrics", () => {
  const startedAt = new Date("2026-10-14T00:00:00.000Z");
  const at = (ms: number) => new Date(startedAt.getTime() + ms);
  const live = (items: StreamItem[], nowMs: number) =>
    formatComposerTurnMetricsLabel(
      deriveComposerTurnMetrics({
        items,
        isTurnActive: true,
        turnStartedAt: startedAt,
        turnId: null,
        now: startedAt.getTime() + nowMs,
      }),
    );

  it("counts the wait until the first model output", () => {
    assert.equal(live([user("prompt", at(0))], 3_400), "TTFT 3s…");
  });

  it("fixes TTFT at the first tool call and estimates the rate from streamed text", () => {
    const items: StreamItem[] = [
      user("prompt", at(0)),
      {
        kind: "tool_call",
        id: "tool",
        timestamp: at(1_500),
        payload: {
          source: "agent",
          data: {
            provider: "claude",
            callId: "call-1",
            name: "Read",
            status: "running",
            error: null,
            detail: { type: "unknown", input: null, output: null },
          },
        },
      },
      { ...assistant("reply", at(2_000)), text: "x".repeat(800) },
    ];
    // 800 chars ≈ 200 tokens over 4s since the first output.
    assert.equal(live(items, 5_500), "TTFT 1.5s · ~50 tok/s");
    // Under one second of output there is no stable rate yet.
    assert.equal(live(items, 2_000), "TTFT 1.5s");
  });

  it("keeps the earliest seen first output while merged text moves its timestamp", () => {
    const derive = (text: string, timestampMs: number, nowMs: number, seen: number | null) =>
      deriveComposerTurnMetrics({
        items: [user("prompt", at(0)), { ...assistant("reply", at(timestampMs)), text }],
        isTurnActive: true,
        turnStartedAt: startedAt,
        turnId: null,
        now: startedAt.getTime() + nowMs,
        seenFirstOutputAt: seen,
      });
    const firstChunk = derive("x", 1_200, 1_300, null);
    assert.equal(firstChunk?.kind, "streaming");
    const seen = firstChunk?.kind === "streaming" ? firstChunk.firstOutputAt : null;
    const later = derive("x".repeat(400), 4_000, 5_200, seen);
    // 400 chars ≈ 100 tokens over 4s since the first chunk.
    assert.equal(formatComposerTurnMetricsLabel(later), "TTFT 1.2s · ~25 tok/s");
  });

  it("shows the daemon metrics of the last completed turn only", () => {
    const completed = {
      ...assistant("reply", at(2_000)),
      turnMetrics: { ttftMs: 1_200, outputTokensPerSecond: 45 },
    };
    const idle = (items: StreamItem[]) =>
      formatComposerTurnMetricsLabel(
        deriveComposerTurnMetrics({
          items,
          isTurnActive: false,
          turnStartedAt: null,
          turnId: null,
          now: 0,
        }),
      );
    assert.equal(idle([user("prompt", at(0)), completed]), "TTFT 1.2s · 45 tok/s");
    assert.equal(
      idle([
        user("prompt", at(0)),
        completed,
        user("next", at(9_000)),
        assistant("plain", at(9_500)),
      ]),
      "",
    );
  });
});
