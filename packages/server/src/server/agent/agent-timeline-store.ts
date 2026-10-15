import { randomUUID } from "node:crypto";
import {
  TimelineProjection,
  projectTimelineRows,
  selectProjectedTimelinePage,
  type ProjectedTimelinePageSelection,
  type ProjectedTimelineRow,
  type TimelineProjectionEntry,
} from "./timeline-projection.js";
import type { TurnMetrics } from "@getpaseo/protocol/messages";
import { timelineItemIdentity } from "@getpaseo/protocol/timeline-identity";
import type { AgentTimelineItem } from "./agent-sdk-types.js";
import type {
  AgentTimelineFetchDirection,
  AgentTimelineFetchOptions,
  AgentTimelineFetchResult,
  AgentTimelineRow,
} from "./agent-timeline-store-types.js";

export interface SeedAgentTimelineOptions {
  items?: readonly AgentTimelineItem[];
  rows?: readonly AgentTimelineRow[];
  epoch?: string;
  nextSeq?: number;
  timestamp?: string;
}

interface AgentTimelineState {
  epoch: string;
  projection: TimelineProjection;
  minSeq: number;
  nextSeq: number;
  /**
   * 0 until a trim. After one, items whose last source seq is below this seq live only in the
   * timeline cache; memory holds every item with a source seq at or above it.
   */
  trimmedBelowSeq: number;
  // In memory only: metrics are lost on daemon restart or timeline re-initialization.
  turnMetrics: Map<string, TurnMetrics>;
}

/** Reads one agent's stored source rows, for pages past the in-memory tail. */
export interface TimelineHistoryReader {
  /** Rows with seq in [fromSeq, toSeq], in seq order. */
  readRows(fromSeq: number, toSeq: number): AgentTimelineRow[];
  /** Rows below `beforeSeq` whose item identity is listed, in seq order. */
  readIdentityRows(identities: readonly string[], beforeSeq: number): AgentTimelineRow[];
  latestPromptSeq(): number | null;
}

interface HistoryWindow {
  entries: TimelineProjectionEntry[];
  fromSeq: number;
  toSeq: number;
}

interface PageRequest {
  direction: AgentTimelineFetchDirection;
  cursorSeq: number | undefined;
  limit: number;
  bounds: { minSeq: number; maxSeq: number };
}

/** Source rows each timeline keeps in memory once the cache holds the older ones. */
export const TIMELINE_TAIL_ROWS = 1_000;
const DEFAULT_TIMELINE_FETCH_LIMIT = 200;
const LATEST_PROMPT_TAIL_CAP = 200;
/** First window of cached rows read for a page past the tail; it grows 4x until it suffices. */
const HISTORY_WINDOW_ROWS = 256;
/** A latest prompt further back than this many rows counts as out of reach. */
const LATEST_PROMPT_SCAN_ROWS = 10_000;
/** Rows read per step while a window edge widens to a whole assistant or reasoning run. */
const RUN_SCAN_ROWS = 64;

function isChunkRow(row: AgentTimelineRow): boolean {
  return row.item.type === "assistant_message" || row.item.type === "reasoning";
}

/** Adjacent rows the projection may merge into one item; a window edge never splits them. */
function sameRun(left: AgentTimelineRow, right: AgentTimelineRow): boolean {
  return (
    isChunkRow(left) &&
    left.item.type === right.item.type &&
    left.turnId === right.turnId &&
    right.seq === left.seq + 1
  );
}

function displayAnchor(row: AgentTimelineRow): number {
  return "seqStart" in row ? (row as ProjectedTimelineRow).seqStart : row.seq;
}

function identitiesOf(rows: readonly AgentTimelineRow[]): string[] {
  const identities = new Set<string>();
  for (const row of rows) {
    const identity = timelineItemIdentity(row.item);
    if (identity !== null) identities.add(identity);
  }
  return [...identities];
}

/** Source seqs up to `lastCachedSeq` that belong to these items. */
function cachedSeqsOf(rows: readonly ProjectedTimelineRow[], lastCachedSeq: number): Set<number> {
  const seqs = new Set<number>();
  for (const row of rows) {
    for (const range of row.sourceSeqRanges) {
      for (let seq = range.startSeq; seq <= Math.min(range.endSeq, lastCachedSeq); seq++) {
        seqs.add(seq);
      }
    }
  }
  return seqs;
}

function asRow(entry: TimelineProjectionEntry): ProjectedTimelineRow {
  return { ...entry, seq: entry.seqEnd };
}

/** A trimmed timeline whose cache stopped serving: pages within the tail still work. */
function unavailableHistory(agentId: string): TimelineHistoryReader {
  const fail = (): never => {
    throw new Error(
      `Timeline history of agent '${agentId}' before its in-memory tail is unavailable`,
    );
  };
  return { readRows: fail, readIdentityRows: fail, latestPromptSeq: fail };
}

/** Tail limit that reaches the latest user message, or the base limit when it is out of reach. */
function latestPromptTailLimit(rows: readonly AgentTimelineRow[], baseLimit: number): number {
  if (baseLimit === 0) return 0;
  const projected = projectTimelineRows({ rows, mode: "projected" });
  const promptIndex = projected.findLastIndex((entry) => entry.item.type === "user_message");
  if (promptIndex === -1) return baseLimit;
  const promptLimit = projected.length - promptIndex;
  if (promptLimit > LATEST_PROMPT_TAIL_CAP) return baseLimit;
  return Math.max(baseLimit, promptLimit);
}

function cloneRow<T extends AgentTimelineRow>(row: T): T {
  return { ...row };
}

export class InMemoryAgentTimelineStore {
  private readonly states = new Map<string, AgentTimelineState>();
  private readonly tailRows: number;

  constructor(options?: { tailRows?: number }) {
    this.tailRows = options?.tailRows ?? TIMELINE_TAIL_ROWS;
  }

  has(agentId: string): boolean {
    return this.states.has(agentId);
  }

  initialize(agentId: string, options?: SeedAgentTimelineOptions): void {
    const timestamp = options?.timestamp ?? new Date().toISOString();
    const rows = options?.rows?.length
      ? options.rows.map(cloneRow)
      : this.buildRowsFromItems(options?.items ?? [], options?.nextSeq ?? 1, timestamp);
    const nextSeq = rows.reduce((next, row) => Math.max(next, row.seq + 1), options?.nextSeq ?? 1);
    const projection = new TimelineProjection();
    for (const row of rows) projection.append(row);
    this.states.set(agentId, {
      epoch: options?.epoch ?? randomUUID(),
      projection,
      minSeq: projection.getRows()[0]?.seqStart ?? 0,
      nextSeq,
      trimmedBelowSeq: 0,
      turnMetrics: new Map(),
    });
  }

  /**
   * Drops items whose source rows all sit below the tail and up to `flushedThroughSeq`, which
   * the timeline cache already stores. An item with a newer source row stays whole.
   */
  trimFlushed(agentId: string, flushedThroughSeq: number): void {
    const state = this.requireState(agentId);
    const cutoff = Math.min(flushedThroughSeq + 1, state.nextSeq - this.tailRows);
    if (cutoff <= state.trimmedBelowSeq) return;
    const rows = state.projection.getRows();
    const kept = rows.filter((row) => row.seqEnd >= cutoff);
    if (kept.length === rows.length) return;
    const projection = new TimelineProjection();
    for (const row of kept) projection.append(row);
    state.projection = projection;
    state.trimmedBelowSeq = cutoff;
  }

  delete(agentId: string): void {
    this.states.delete(agentId);
  }

  getItems(agentId: string): AgentTimelineItem[] {
    return this.requireState(agentId)
      .projection.getRows()
      .map((row) => row.item);
  }

  getRows(agentId: string): ProjectedTimelineRow[] {
    return this.requireState(agentId).projection.getRows().map(cloneRow);
  }

  getSubmittedUserMessage(agentId: string, clientMessageId: string): AgentTimelineRow | null {
    const row = this.requireState(agentId)
      .projection.getRows()
      .find(
        (candidate) =>
          candidate.item.type === "user_message" &&
          candidate.item.clientMessageId === clientMessageId,
      );
    return row ? cloneRow(row) : null;
  }

  enrichSubmittedUserMessage(
    agentId: string,
    clientMessageId: string,
    providerMessageId: string,
  ): AgentTimelineRow | null {
    return this.requireState(agentId).projection.enrichSubmittedUserMessage(
      clientMessageId,
      providerMessageId,
    );
  }

  getEpoch(agentId: string): string {
    return this.requireState(agentId).epoch;
  }

  /** `history` serves pages past the in-memory tail; it is read only after a trim. */
  fetch(
    agentId: string,
    options?: AgentTimelineFetchOptions,
    history?: TimelineHistoryReader,
  ): AgentTimelineFetchResult {
    const state = this.requireState(agentId);
    const direction = options?.direction ?? "tail";
    const cursor = options?.cursor;
    const rows = state.projection.getRows();
    const window = { minSeq: state.minSeq, maxSeq: state.nextSeq - 1, nextSeq: state.nextSeq };
    const staleCursor = cursor !== undefined && cursor.epoch !== state.epoch;
    const gap =
      !staleCursor &&
      direction === "after" &&
      cursor !== undefined &&
      rows.length > 0 &&
      cursor.seq < state.minSeq - 1;
    const reset = staleCursor || gap;
    const trimmedHistory =
      state.trimmedBelowSeq > 0 ? (history ?? unavailableHistory(agentId)) : undefined;
    const request: PageRequest = {
      direction: reset ? "tail" : direction,
      cursorSeq: cursor?.seq,
      limit: this.pageLimit(state, rows, options, trimmedHistory),
      bounds: window,
    };
    const page = trimmedHistory
      ? this.selectAcrossEdge(state, rows, request, trimmedHistory)
      : selectProjectedTimelinePage({ rows, ...request });
    return {
      epoch: state.epoch,
      direction,
      reset,
      staleCursor,
      gap,
      window,
      hasOlder: page.hasOlder,
      hasNewer: page.hasNewer,
      startSeq: page.startSeq,
      endSeq: page.endSeq,
      rows: page.entries.map((entry) => Object.assign({ seq: entry.seqEnd }, entry)),
    };
  }

  append(
    agentId: string,
    item: AgentTimelineItem,
    options?: { timestamp?: string; providerMessageId?: string; turnId?: string },
  ): AgentTimelineRow {
    const state = this.requireState(agentId);
    const row: AgentTimelineRow = {
      seq: state.nextSeq,
      timestamp: options?.timestamp ?? new Date().toISOString(),
      item,
      ...(options?.turnId ? { turnId: options.turnId } : {}),
      ...(options?.providerMessageId ? { providerMessageId: options.providerMessageId } : {}),
    };
    state.nextSeq += 1;
    if (state.minSeq === 0) state.minSeq = row.seq;
    state.projection.append(row);
    return cloneRow(row);
  }

  getLastItem(agentId: string): AgentTimelineItem | null {
    const state = this.requireState(agentId);
    return state.projection.getRows().find((row) => row.seqEnd === state.nextSeq - 1)?.item ?? null;
  }

  setTurnMetrics(agentId: string, turnId: string, metrics: TurnMetrics): void {
    this.requireState(agentId).turnMetrics.set(turnId, { ...metrics });
  }

  getTurnMetrics(agentId: string, turnId: string): TurnMetrics | undefined {
    const metrics = this.states.get(agentId)?.turnMetrics.get(turnId);
    return metrics ? { ...metrics } : undefined;
  }

  getLastAssistantMessage(agentId: string): string | null {
    const row = this.requireState(agentId)
      .projection.getRows()
      .findLast((candidate) => candidate.item.type === "assistant_message");
    return row?.item.type === "assistant_message" ? row.item.text : null;
  }

  private pageLimit(
    state: AgentTimelineState,
    rows: readonly ProjectedTimelineRow[],
    options: AgentTimelineFetchOptions | undefined,
    trimmedHistory: TimelineHistoryReader | undefined,
  ): number {
    const baseLimit = options?.limit ?? DEFAULT_TIMELINE_FETCH_LIMIT;
    const direction = options?.direction ?? "tail";
    if (options?.includeLatestPrompt !== true || direction !== "tail") return baseLimit;
    return trimmedHistory
      ? this.latestPromptLimitAcrossEdge(state, rows, baseLimit, trimmedHistory)
      : latestPromptTailLimit(rows, baseLimit);
  }

  /** latestPromptTailLimit for a trimmed timeline, whose latest prompt may sit in the cache. */
  private latestPromptLimitAcrossEdge(
    state: AgentTimelineState,
    rows: readonly ProjectedTimelineRow[],
    baseLimit: number,
    history: TimelineHistoryReader,
  ): number {
    if (baseLimit === 0) return 0;
    // A user message is one source row, so a prompt in memory is newer than any cached one.
    if (rows.some((row) => row.item.type === "user_message")) {
      return latestPromptTailLimit(rows, baseLimit);
    }
    const promptSeq = history.latestPromptSeq();
    if (promptSeq === null || state.trimmedBelowSeq - promptSeq > LATEST_PROMPT_SCAN_ROWS) {
      return baseLimit;
    }
    const window = this.readAcrossEdge(state, rows, history, promptSeq, state.trimmedBelowSeq - 1);
    const promptLimit = window.entries.filter((entry) => entry.seqStart >= promptSeq).length;
    if (promptLimit > LATEST_PROMPT_TAIL_CAP) return baseLimit;
    return Math.max(baseLimit, promptLimit);
  }

  /**
   * Selects a page of a trimmed timeline. Memory serves it when every selected item and every
   * item between them is in memory; otherwise a window of cached rows joins the memory items.
   */
  private selectAcrossEdge(
    state: AgentTimelineState,
    rows: readonly ProjectedTimelineRow[],
    request: PageRequest,
    history: TimelineHistoryReader,
  ): ProjectedTimelinePageSelection {
    const edge = state.trimmedBelowSeq;
    const { bounds, cursorSeq, limit } = request;
    const memoryPage = selectProjectedTimelinePage({ rows, ...request });
    if (request.direction === "after") {
      const startSeq = Math.max(bounds.minSeq, (cursorSeq ?? bounds.minSeq - 1) + 1);
      // Dropped items end below the edge, so memory holds every item with a row from startSeq.
      if (startSeq >= edge) return memoryPage;
      return this.selectAfterAcrossEdge(state, rows, request, history, startSeq);
    }
    // Every item that starts at or after the edge is in memory, so such a page has no holes.
    const fromMemory =
      limit > 0 && memoryPage.entries.length >= limit && (memoryPage.startSeq ?? -1) >= edge;
    if (fromMemory) return { ...memoryPage, hasOlder: true };
    const endSeq =
      request.direction === "tail"
        ? bounds.maxSeq
        : Math.min(bounds.maxSeq, (cursorSeq ?? bounds.maxSeq + 1) - 1);
    if (endSeq < bounds.minSeq) return memoryPage;
    let span = limit === 0 ? Number.POSITIVE_INFINITY : Math.max(limit * 2, HISTORY_WINDOW_ROWS);
    for (;;) {
      const toSeq = Math.min(endSeq, edge - 1);
      const window = this.readAcrossEdge(state, rows, history, toSeq - span + 1, toSeq);
      // Items starting below the window may own rows inside it; they belong to an older page.
      const entries = window.entries.filter((entry) => entry.seqStart >= window.fromSeq).map(asRow);
      const page = selectProjectedTimelinePage({ rows: entries, ...request });
      const atStart = window.fromSeq <= bounds.minSeq;
      if (atStart) return page;
      if (limit > 0 && page.entries.length >= limit) return { ...page, hasOlder: true };
      span *= 4;
    }
  }

  private selectAfterAcrossEdge(
    state: AgentTimelineState,
    rows: readonly ProjectedTimelineRow[],
    request: PageRequest,
    history: TimelineHistoryReader,
    startSeq: number,
  ): ProjectedTimelinePageSelection {
    const { bounds, limit } = request;
    const lastCachedSeq = state.trimmedBelowSeq - 1;
    let span = limit === 0 ? Number.POSITIVE_INFINITY : Math.max(limit * 2, HISTORY_WINDOW_ROWS);
    for (;;) {
      const toSeq = Math.min(lastCachedSeq, startSeq + span - 1);
      const window = this.readAcrossEdge(state, rows, history, startSeq, toSeq);
      const reachesMemory = window.toSeq >= lastCachedSeq;
      // Below the edge, only rows up to the window end are known, so the page ends there.
      const page = selectProjectedTimelinePage({
        rows: window.entries.map(asRow),
        ...request,
        bounds: reachesMemory ? bounds : { minSeq: bounds.minSeq, maxSeq: window.toSeq },
      });
      if (reachesMemory || (limit > 0 && page.entries.length >= limit)) {
        return { ...page, hasNewer: page.endSeq !== null && page.endSeq < bounds.maxSeq };
      }
      span *= 4;
    }
  }

  /**
   * Projects cached rows in [fromSeq, toSeq] together with the memory items. The window widens
   * so no assistant or reasoning run is cut, and pulls every row of each tool call or plugin item
   * in it, so each item that starts inside the window is whole.
   */
  private readAcrossEdge(
    state: AgentTimelineState,
    memoryRows: readonly ProjectedTimelineRow[],
    history: TimelineHistoryReader,
    fromSeq: number,
    toSeq: number,
  ): HistoryWindow {
    const lastCachedSeq = state.trimmedBelowSeq - 1;
    let from = Math.max(state.minSeq, fromSeq);
    let to = Math.min(toSeq, lastCachedSeq);
    const rows = to >= from ? history.readRows(from, to) : [];
    while (from > state.minSeq && rows[0] && isChunkRow(rows[0])) {
      const before = history.readRows(Math.max(state.minSeq, from - RUN_SCAN_ROWS), from - 1);
      let index = before.length - 1;
      while (index >= 0 && sameRun(before[index], rows[0])) {
        rows.unshift(before[index]);
        index -= 1;
      }
      from = rows[0].seq;
      if (index >= 0 || before.length === 0) break;
    }
    while (to < lastCachedSeq && rows.length > 0 && isChunkRow(rows[rows.length - 1])) {
      const after = history.readRows(to + 1, Math.min(lastCachedSeq, to + RUN_SCAN_ROWS));
      let index = 0;
      while (index < after.length && sameRun(rows[rows.length - 1], after[index])) {
        rows.push(after[index]);
        index += 1;
      }
      to = rows[rows.length - 1].seq;
      if (index < after.length || after.length === 0) break;
    }

    const windowSeqs = new Set(rows.map((row) => row.seq));
    const identityRows = history
      .readIdentityRows(identitiesOf(rows), state.trimmedBelowSeq)
      .filter((row) => !windowSeqs.has(row.seq));
    // Memory items are whole; their cached source rows must not project a second copy.
    const inMemory = cachedSeqsOf(memoryRows, lastCachedSeq);
    const combined: AgentTimelineRow[] = [...rows, ...identityRows]
      .filter((row) => !inMemory.has(row.seq))
      .concat(memoryRows)
      .sort((left, right) => displayAnchor(left) - displayAnchor(right));
    return {
      entries: projectTimelineRows({ rows: combined, mode: "projected" }),
      fromSeq: from,
      toSeq: Math.max(to, from - 1),
    };
  }

  private requireState(agentId: string): AgentTimelineState {
    const state = this.states.get(agentId);
    if (!state) {
      throw new Error(`Unknown agent '${agentId}'`);
    }
    return state;
  }

  private buildRowsFromItems(
    items: readonly AgentTimelineItem[],
    startSeq: number,
    timestamp: string,
  ): AgentTimelineRow[] {
    let nextSeq = startSeq;
    return items.map((item) => {
      const row: AgentTimelineRow = {
        seq: nextSeq,
        timestamp,
        item,
      };
      nextSeq += 1;
      return row;
    });
  }
}
