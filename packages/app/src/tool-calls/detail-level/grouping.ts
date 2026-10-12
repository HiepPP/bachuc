import type { ToolCallDetail } from "@getpaseo/protocol/agent-types";
import type { StreamItem, ToolCallItem } from "@/types/stream";

export interface ToolCallDescriptor {
  detail: ToolCallDetail;
  name: string;
  status: "executing" | "running" | "completed" | "failed" | "canceled";
  error: unknown;
  metadata?: Record<string, unknown>;
}

export interface ToolCallRun {
  id: string;
  calls: readonly ToolCallItem[];
  latest: ToolCallItem;
  isSealed: boolean;
}

export interface GroupedHistory<TGroup> {
  tail: StreamItem[];
  groupsByHostId: Map<string, TGroup>;
  pendingCalls: readonly ToolCallItem[];
}

export interface GroupedToolCalls<TGroup> {
  tail: StreamItem[];
  head: StreamItem[];
  groupsByHostId: ToolCallGroupLookup<TGroup>;
  historyGroupUpdatesByHostId: ToolCallGroupLookup<TGroup>;
}

export interface ToolCallGroupLookup<TGroup> {
  readonly size: number;
  get(id: string): TGroup | undefined;
  has(id: string): boolean;
}

/** One agent turn: its notes, thoughts, tool calls, and the plugin rows that replaced a tool call. */
export interface TurnRun {
  id: string;
  items: readonly StreamItem[];
  /** The last tool call, which shapes the host row; a replaced call stands in for its plugin row. */
  latest: ToolCallItem;
  /**
   * The user message before the run, or the boundary that split its turn. A tool call takes
   * the time of its latest update, so the first item can start later than the turn did.
   */
  startedAt: Date;
  isSealed: boolean;
}

export interface GroupedTurnHistory<TGroup> {
  tail: StreamItem[];
  /** `tail` without the rows that the trailing run takes back while the turn goes on. */
  openTail: StreamItem[];
  groupsByHostId: Map<string, TGroup>;
  pendingItems: readonly StreamItem[];
  /** Where `pendingItems` started; null when no user message or split boundary is before them. */
  pendingStartedAt: Date | null;
  isPendingHosted: boolean;
}

const EMPTY_GROUPS = new Map<string, never>();

export function describeToolCall(item: ToolCallItem): ToolCallDescriptor {
  if (item.payload.source === "agent") {
    const { data } = item.payload;
    return {
      detail: data.detail,
      name: data.name,
      status: data.status,
      error: data.error,
      metadata: data.metadata,
    };
  }

  const { data } = item.payload;
  return {
    detail: {
      type: "unknown",
      input: data.arguments ?? null,
      output: data.result ?? null,
    },
    name: data.toolName,
    status: data.status,
    error: data.error,
  };
}

export function isGroupableToolCall(item: StreamItem): item is ToolCallItem {
  if (item.kind !== "tool_call") {
    return false;
  }
  const descriptor = describeToolCall(item);
  return descriptor.detail.type !== "plan" && descriptor.name.trim().toLowerCase() !== "speak";
}

function createRun(calls: readonly ToolCallItem[], isSealed: boolean): ToolCallRun {
  const first = calls[0];
  const latest = calls.at(-1);
  if (!first || !latest) {
    throw new Error("Cannot group an empty tool call run");
  }
  return { id: first.id, calls, latest, isSealed };
}

function createHost(run: ToolCallRun): ToolCallItem {
  if (run.calls.length === 1) {
    return run.latest;
  }
  return { ...run.latest, id: run.id };
}

function isRunning(call: ToolCallItem): boolean {
  const status = describeToolCall(call).status;
  return status === "running" || status === "executing";
}

function appendRun<TGroup>(input: {
  calls: readonly ToolCallItem[];
  isSealed: boolean;
  output: StreamItem[];
  groups: Map<string, TGroup>;
  buildGroup: (run: ToolCallRun) => TGroup;
}): void {
  if (input.calls.length === 0) {
    return;
  }
  const run = createRun(input.calls, input.isSealed);
  const host = createHost(run);
  input.output.push(host);
  input.groups.set(host.id, input.buildGroup(run));
}

export function prepareGroupedHistory<TGroup>(input: {
  tail: StreamItem[];
  buildGroup: (run: ToolCallRun) => TGroup;
}): GroupedHistory<TGroup> {
  const output: StreamItem[] = [];
  const groups = new Map<string, TGroup>();
  let pending: ToolCallItem[] = [];

  for (const item of input.tail) {
    if (isGroupableToolCall(item)) {
      pending.push(item);
      continue;
    }
    appendRun({
      calls: pending,
      isSealed: true,
      output,
      groups,
      buildGroup: input.buildGroup,
    });
    pending = [];
    output.push(item);
  }

  appendRun({
    calls: pending,
    isSealed: true,
    output,
    groups,
    buildGroup: input.buildGroup,
  });

  return {
    tail: groups.size > 0 ? output : input.tail,
    groupsByHostId: groups,
    pendingCalls: pending,
  };
}

export function groupLiveToolCalls<TGroup>(input: {
  history: GroupedHistory<TGroup>;
  head: StreamItem[];
  isTurnActive: boolean;
  buildGroup: (run: ToolCallRun) => TGroup;
}): GroupedToolCalls<TGroup> {
  const head: StreamItem[] = [];
  const liveGroups = new Map<string, TGroup>();
  let pending = [...input.history.pendingCalls];
  let hostPlacement: "history" | "head" | null = pending.length > 0 ? "history" : null;
  let pendingIncludesHead = false;

  const flush = (isSealed: boolean) => {
    if (pending.length === 0) {
      return;
    }
    const run = createRun(pending, isSealed);
    if (hostPlacement === "head") {
      head.push(createHost(run));
    }
    if (hostPlacement === "head" || pendingIncludesHead || !isSealed) {
      liveGroups.set(run.id, input.buildGroup(run));
    }
    pending = [];
    hostPlacement = null;
    pendingIncludesHead = false;
  };

  for (const item of input.head) {
    if (isGroupableToolCall(item)) {
      if (pending.length === 0) {
        hostPlacement = "head";
      }
      pending.push(item);
      pendingIncludesHead = true;
      continue;
    }
    flush(true);
    head.push(item);
  }
  // Tool calls live in retained tail rather than the streaming head. The agent
  // lifecycle snapshot can still be idle while a newly received tool call is
  // already running, so its direct timeline status is the authoritative start
  // signal. The lifecycle state continues to keep completed calls live between
  // sequential tool updates.
  const trailingRunIsActive = input.isTurnActive || pending.some(isRunning);
  flush(!trailingRunIsActive);

  if (liveGroups.size === 0) {
    return {
      tail: input.history.tail,
      head: input.head,
      groupsByHostId: input.history.groupsByHostId,
      historyGroupUpdatesByHostId: EMPTY_GROUPS,
    };
  }
  return mergeLiveGroups({
    tail: input.history.tail,
    head,
    historyGroups: input.history.groupsByHostId,
    liveGroups,
  });
}

function mergeLiveGroups<TGroup>(input: {
  tail: StreamItem[];
  head: StreamItem[];
  historyGroups: Map<string, TGroup>;
  liveGroups: Map<string, TGroup>;
}): GroupedToolCalls<TGroup> {
  if (input.historyGroups.size === 0) {
    return {
      tail: input.tail,
      head: input.head,
      groupsByHostId: input.liveGroups,
      historyGroupUpdatesByHostId: EMPTY_GROUPS,
    };
  }
  const groupsByHostId = new Map(input.historyGroups);
  let historyGroupUpdatesByHostId: Map<string, TGroup> | null = null;
  for (const [id, group] of input.liveGroups) {
    groupsByHostId.set(id, group);
    if (input.historyGroups.has(id)) {
      historyGroupUpdatesByHostId ??= new Map();
      historyGroupUpdatesByHostId.set(id, group);
    }
  }
  return {
    tail: input.tail,
    head: input.head,
    groupsByHostId,
    historyGroupUpdatesByHostId: historyGroupUpdatesByHostId ?? EMPTY_GROUPS,
  };
}

/** The tool call behind a turn run item: the call itself, or the call a plugin row replaced. */
export function turnWorkCall(item: StreamItem): ToolCallItem | undefined {
  if (isGroupableToolCall(item)) {
    return item;
  }
  return item.kind === "plugin" ? item.sourceToolCall : undefined;
}

function isTurnRunItem(item: StreamItem): boolean {
  return (
    item.kind === "assistant_message" || item.kind === "thought" || turnWorkCall(item) !== undefined
  );
}

function isRunningWork(item: StreamItem): boolean {
  const call = turnWorkCall(item);
  return call !== undefined && isRunning(call);
}

/**
 * Where the turn's answer starts: its last assistant message, with every Markdown block of
 * it, when only thoughts follow. Returns `items.length` when the turn has no answer.
 */
function findAnswerStart(items: readonly StreamItem[]): number {
  let start = items.length;
  let messageId: string | null = null;
  for (let index = items.length - 1; index >= 0; index -= 1) {
    const item = items[index]!;
    if (item.kind === "thought") {
      continue;
    }
    if (item.kind !== "assistant_message") {
      break;
    }
    const itemMessageId = item.blockGroupId ?? item.id;
    if (messageId !== null && itemMessageId !== messageId) {
      break;
    }
    messageId = itemMessageId;
    start = index;
  }
  return start;
}

/**
 * Whether the turn ends at each boundary: no note or tool work follows it before the next
 * user message, and that message came or the turn is idle. Then the answer leaves the run.
 */
function findTurnEnds(items: readonly StreamItem[], isTurnActive: boolean): boolean[] {
  const ends = Array.from({ length: items.length }, () => false);
  let isEnded = !isTurnActive;
  for (let index = items.length - 1; index >= 0; index -= 1) {
    const item = items[index]!;
    if (item.kind === "user_message") {
      isEnded = true;
    }
    ends[index] = isEnded;
    if (item.kind === "assistant_message" || turnWorkCall(item) !== undefined) {
      isEnded = false;
    }
  }
  return ends;
}

/** The run after a boundary starts there, unless the boundary ended a turn with no user message. */
function startAfter(boundary: StreamItem, endsTurn: boolean): Date | null {
  return boundary.kind === "user_message" || !endsTurn ? boundary.timestamp : null;
}

/** A turn run needs a tool call; notes and thoughts alone keep their own rows. */
function createTurnRun(input: {
  items: readonly StreamItem[];
  startedAt: Date | null;
  isSealed: boolean;
}): TurnRun | null {
  const { items, isSealed } = input;
  let latest: ToolCallItem | undefined;
  for (const item of items) {
    latest = turnWorkCall(item) ?? latest;
  }
  const first = items[0];
  if (!first || !latest) {
    return null;
  }
  return { id: first.id, items, latest, startedAt: input.startedAt ?? first.timestamp, isSealed };
}

function createTurnHost(run: TurnRun): ToolCallItem {
  if (run.items.length === 1 && run.items[0] === run.latest) {
    return run.latest;
  }
  return { ...run.latest, id: run.id };
}

/** Places a sealed run and its answer. Returns whether the run got a host row. */
function appendTurnRun<TGroup>(input: {
  items: readonly StreamItem[];
  startedAt: Date | null;
  endsTurn: boolean;
  output: StreamItem[];
  groups: Map<string, TGroup>;
  buildGroup: (run: TurnRun) => TGroup;
}): boolean {
  const answerStart = input.endsTurn ? findAnswerStart(input.items) : input.items.length;
  const run = createTurnRun({
    items: input.items.slice(0, answerStart),
    startedAt: input.startedAt,
    isSealed: true,
  });
  if (!run) {
    input.output.push(...input.items);
    return false;
  }
  const host = createTurnHost(run);
  input.output.push(host, ...input.items.slice(answerStart));
  input.groups.set(host.id, input.buildGroup(run));
  return true;
}

/**
 * Turn mode: notes, thoughts, and replaced tool calls join the run, so one turn is one run.
 * A boundary where the turn goes on splits the run and keeps its last note inside.
 */
export function prepareTurnHistory<TGroup>(input: {
  tail: StreamItem[];
  isTurnActive: boolean;
  buildGroup: (run: TurnRun) => TGroup;
}): GroupedTurnHistory<TGroup> {
  const output: StreamItem[] = [];
  const groups = new Map<string, TGroup>();
  const turnEnds = findTurnEnds(input.tail, input.isTurnActive);
  let pending: StreamItem[] = [];
  let startedAt: Date | null = null;

  for (const [index, item] of input.tail.entries()) {
    if (isTurnRunItem(item)) {
      pending.push(item);
      continue;
    }
    const endsTurn = turnEnds[index]!;
    appendTurnRun({
      items: pending,
      startedAt,
      endsTurn,
      output,
      groups,
      buildGroup: input.buildGroup,
    });
    pending = [];
    startedAt = startAfter(item, endsTurn);
    output.push(item);
  }

  // History places the trailing run as if the turn ended; the live head reopens it.
  const pendingStart = output.length;
  const isPendingHosted = appendTurnRun({
    items: pending,
    startedAt,
    endsTurn: true,
    output,
    groups,
    buildGroup: input.buildGroup,
  });
  const tail = groups.size > 0 ? output : input.tail;
  const openLength = isPendingHosted ? pendingStart + 1 : pendingStart;
  return {
    tail,
    openTail: openLength < tail.length ? tail.slice(0, openLength) : tail,
    groupsByHostId: groups,
    pendingItems: pending,
    pendingStartedAt: startedAt,
    isPendingHosted,
  };
}

export function groupLiveTurns<TGroup>(input: {
  history: GroupedTurnHistory<TGroup>;
  head: StreamItem[];
  isTurnActive: boolean;
  buildGroup: (run: TurnRun) => TGroup;
}): GroupedToolCalls<TGroup> {
  const { history } = input;
  let tail = history.tail;
  const head: StreamItem[] = [];
  const liveGroups = new Map<string, TGroup>();
  let pending = [...history.pendingItems];
  let historyCount = pending.length;
  let startedAt = history.pendingStartedAt;
  const turnEnds = findTurnEnds(input.head, input.isTurnActive);

  const flush = ({ isSealed, endsTurn }: { isSealed: boolean; endsTurn: boolean }) => {
    const answerStart = endsTurn ? findAnswerStart(pending) : pending.length;
    const run = createTurnRun({ items: pending.slice(0, answerStart), startedAt, isSealed });
    if (!run) {
      head.push(...pending.slice(historyCount));
    } else {
      const isHostInHistory = historyCount > 0 && history.isPendingHosted;
      // An answer that starts in history keeps the rows history placed after the host.
      const keepsHistoryAnswer = isHostInHistory && answerStart < historyCount;
      const reopensHistory =
        historyCount > 0 && !keepsHistoryAnswer && history.openTail !== history.tail;
      if (reopensHistory) {
        tail = history.openTail;
      }
      if (!isHostInHistory) {
        head.push(createTurnHost(run));
      }
      head.push(...pending.slice(keepsHistoryAnswer ? historyCount : answerStart));
      const includesHead = pending.length > historyCount;
      if (!isHostInHistory || reopensHistory || includesHead || !isSealed) {
        liveGroups.set(run.id, input.buildGroup(run));
      }
    }
    pending = [];
    historyCount = 0;
  };

  for (const [index, item] of input.head.entries()) {
    if (isTurnRunItem(item)) {
      pending.push(item);
      continue;
    }
    const endsTurn = turnEnds[index]!;
    flush({ isSealed: true, endsTurn });
    startedAt = startAfter(item, endsTurn);
    head.push(item);
  }
  // A call that still shows as running keeps the run open, but an idle turn has its answer.
  const trailingRunIsActive = input.isTurnActive || pending.some(isRunningWork);
  flush({ isSealed: !trailingRunIsActive, endsTurn: !input.isTurnActive });

  if (liveGroups.size === 0) {
    return {
      tail: history.tail,
      head: input.head,
      groupsByHostId: history.groupsByHostId,
      historyGroupUpdatesByHostId: EMPTY_GROUPS,
    };
  }
  return mergeLiveGroups({ tail, head, historyGroups: history.groupsByHostId, liveGroups });
}
