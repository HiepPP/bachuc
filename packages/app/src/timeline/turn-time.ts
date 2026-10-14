import type { TurnMetrics } from "@getpaseo/protocol/messages";
import type { StreamItem } from "@/types/stream";
import { startsNewTurn } from "@/agent-stream/turn-membership";

export interface TurnTiming {
  completedAt: Date;
  durationMs: number | null;
  metrics?: TurnMetrics;
}

export interface StreamTurnTiming {
  byAssistantId: Map<string, TurnTiming>;
  runningStartedAt: Date | null;
}

export function deriveStreamTurnTiming(params: {
  isTurnActive: boolean;
  activeTurnStartedAt: Date | null;
  tail: StreamItem[];
  head: StreamItem[];
}): StreamTurnTiming {
  const byAssistantId = new Map<string, TurnTiming>();
  let currentUserAt: Date | null = null;
  let currentLastItemAt: Date | null = null;
  let currentAssistantIds: string[] = [];
  let currentMetrics: TurnMetrics | undefined;
  let previousItem: StreamItem | null = null;

  const flushCompletedTurn = () => {
    if (!currentLastItemAt || currentAssistantIds.length === 0) {
      return;
    }
    const timing: TurnTiming = {
      completedAt: currentLastItemAt,
      durationMs: currentUserAt
        ? Math.max(0, currentLastItemAt.getTime() - currentUserAt.getTime())
        : null,
      ...(currentMetrics ? { metrics: currentMetrics } : {}),
    };
    for (const id of currentAssistantIds) {
      byAssistantId.set(id, timing);
    }
  };

  const visitItem = (item: StreamItem) => {
    if (startsNewTurn(item, previousItem)) {
      flushCompletedTurn();
      currentUserAt = item.kind === "user_message" ? item.timestamp : null;
      currentLastItemAt = null;
      currentAssistantIds = [];
      currentMetrics = undefined;
    }
    currentLastItemAt = item.timestamp;
    if (item.kind === "assistant_message") {
      currentAssistantIds.push(item.id);
      currentMetrics = item.turnMetrics ?? currentMetrics;
    }
    previousItem = item;
  };

  for (const item of params.tail) {
    visitItem(item);
  }
  for (const item of params.head) {
    visitItem(item);
  }

  const runningStartedAt = params.isTurnActive ? params.activeTurnStartedAt : null;
  if (!params.isTurnActive) {
    flushCompletedTurn();
  }

  return {
    byAssistantId,
    runningStartedAt,
  };
}

function formatTtft(ms: number): string {
  return ms < 1000 ? `${Math.round(ms)}ms` : `${(ms / 1000).toFixed(1)}s`;
}

function formatTokensPerSecond(rate: number): string {
  return rate < 10 ? rate.toFixed(1) : String(Math.round(rate));
}

export type TurnMetricTone = "good" | "fair" | "poor";

export interface TurnMetricPart {
  text: string;
  tone: TurnMetricTone;
}

// Lower TTFT is better; higher output rate is better.
const TTFT_GOOD_BELOW_MS = 5000;
const TTFT_FAIR_BELOW_MS = 15000;
const TPS_GOOD_FROM = 80;
const TPS_FAIR_FROM = 40;

export function rateTtft(ms: number): TurnMetricTone {
  if (ms < TTFT_GOOD_BELOW_MS) return "good";
  return ms < TTFT_FAIR_BELOW_MS ? "fair" : "poor";
}

export function rateTokensPerSecond(rate: number): TurnMetricTone {
  if (rate >= TPS_GOOD_FROM) return "good";
  return rate >= TPS_FAIR_FROM ? "fair" : "poor";
}

/**
 * The parts of "TTFT 1.2s · 45 tok/s", each with its tone; empty when the turn has no
 * metrics. `estimated` marks the rate with "~" while a turn is still streaming.
 */
export function buildTurnMetricParts(
  metrics: TurnMetrics | undefined,
  options?: { estimated?: boolean },
): TurnMetricPart[] {
  if (!metrics) {
    return [];
  }
  const parts: TurnMetricPart[] = [];
  if (metrics.ttftMs !== undefined) {
    parts.push({ text: `TTFT ${formatTtft(metrics.ttftMs)}`, tone: rateTtft(metrics.ttftMs) });
  }
  if (metrics.outputTokensPerSecond !== undefined) {
    const prefix = options?.estimated ? "~" : "";
    parts.push({
      text: `${prefix}${formatTokensPerSecond(metrics.outputTokensPerSecond)} tok/s`,
      tone: rateTokensPerSecond(metrics.outputTokensPerSecond),
    });
  }
  return parts;
}

export const TURN_METRIC_SEPARATOR = " · ";

export function formatTurnMetricsLabel(
  metrics: TurnMetrics | undefined,
  options?: { estimated?: boolean },
): string {
  return buildTurnMetricParts(metrics, options)
    .map((part) => part.text)
    .join(TURN_METRIC_SEPARATOR);
}

// Live rate estimate only; the daemon replaces it with the provider's count at turn end.
const ESTIMATED_CHARS_PER_TOKEN = 4;
const MIN_LIVE_RATE_WINDOW_MS = 1000;

export type ComposerTurnMetrics =
  | { kind: "waiting"; elapsedMs: number }
  | { kind: "streaming"; metrics: TurnMetrics; firstOutputAt: number }
  | { kind: "completed"; metrics: TurnMetrics };

function isModelOutput(item: StreamItem): boolean {
  if (item.kind === "tool_call") return true;
  return (item.kind === "assistant_message" || item.kind === "thought") && item.text !== "";
}

function activeTurnItems(items: readonly StreamItem[], turnId: string | null): StreamItem[] {
  if (turnId !== null) {
    return items.filter((item) => item.turnId === turnId);
  }
  let start = items.length;
  while (start > 0 && items[start - 1]?.kind !== "user_message") start -= 1;
  return items.slice(start);
}

/**
 * Metrics for the composer: the running turn live, else the last turn's daemon metrics.
 * A finished turn without metrics shows nothing rather than an older turn's numbers.
 *
 * A merged assistant or thought item carries its latest chunk's timestamp, so the first-output
 * time drifts as text streams. Pass back the returned `firstOutputAt` as `seenFirstOutputAt`
 * to keep the earliest time observed for the turn.
 */
export function deriveComposerTurnMetrics(params: {
  items: readonly StreamItem[];
  isTurnActive: boolean;
  turnStartedAt: Date | null;
  turnId: string | null;
  now: number;
  seenFirstOutputAt?: number | null;
}): ComposerTurnMetrics | null {
  const { items, isTurnActive, turnStartedAt, turnId, now, seenFirstOutputAt } = params;
  if (isTurnActive) {
    if (!turnStartedAt) return null;
    const startedAt = turnStartedAt.getTime();
    const turnItems = activeTurnItems(items, turnId);
    const first = turnItems.find(isModelOutput);
    if (!first) {
      return { kind: "waiting", elapsedMs: Math.max(0, now - startedAt) };
    }
    const firstAt = Math.min(first.timestamp.getTime(), seenFirstOutputAt ?? Infinity);
    const metrics: TurnMetrics = { ttftMs: Math.max(0, firstAt - startedAt) };
    const windowMs = now - firstAt;
    if (windowMs >= MIN_LIVE_RATE_WINDOW_MS) {
      let chars = 0;
      for (const item of turnItems) {
        if (item.kind === "assistant_message" || item.kind === "thought") chars += item.text.length;
      }
      metrics.outputTokensPerSecond = chars / ESTIMATED_CHARS_PER_TOKEN / (windowMs / 1000);
    }
    return { kind: "streaming", metrics, firstOutputAt: firstAt };
  }
  const metrics = findLastTurnMetrics([items]);
  return metrics ? { kind: "completed", metrics } : null;
}

/**
 * Daemon metrics of the last turn, scanning `segments` (in order, e.g. tail then head) from
 * the end. Stops at the last user message, so a later turn without metrics yields null.
 */
export function findLastTurnMetrics(
  segments: readonly (readonly StreamItem[] | undefined)[],
): TurnMetrics | null {
  for (let segment = segments.length - 1; segment >= 0; segment -= 1) {
    const items = segments[segment] ?? [];
    for (let index = items.length - 1; index >= 0; index -= 1) {
      const item = items[index];
      if (!item || item.kind === "user_message") return null;
      if (item.kind === "assistant_message" && item.turnMetrics) return item.turnMetrics;
    }
  }
  return null;
}

/** While waiting, the counting TTFT takes the tone it would have if the first token came now. */
export function buildComposerTurnMetricParts(value: ComposerTurnMetrics | null): TurnMetricPart[] {
  if (!value) return [];
  if (value.kind === "waiting") {
    return [
      { text: `TTFT ${Math.floor(value.elapsedMs / 1000)}s…`, tone: rateTtft(value.elapsedMs) },
    ];
  }
  return buildTurnMetricParts(value.metrics, { estimated: value.kind === "streaming" });
}

export function formatComposerTurnMetricsLabel(value: ComposerTurnMetrics | null): string {
  return buildComposerTurnMetricParts(value)
    .map((part) => part.text)
    .join(TURN_METRIC_SEPARATOR);
}
