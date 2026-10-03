import { createReadStream } from "node:fs";
import fs from "node:fs/promises";
import path from "node:path";
import { createInterface } from "node:readline";

import type {
  AgentStreamEvent,
  AgentTimelineItem,
  ToolCallTimelineItem,
} from "../../agent-sdk-types.js";
import { normalizeProviderReplayTimestamp } from "../../provider-history-timestamps.js";
import { mapCodexToolCallEnvelope } from "./tool-call-mapper.js";

const SESSION_ID_PATTERN = /^[0-9a-f]{8}-(?:[0-9a-f]{4}-){3}[0-9a-f]{12}$/i;
const MAX_SESSION_TREE_DEPTH = 8;

interface CodexOfflineHistoryInput {
  sessionId: string;
  codexHome: string;
  cwd?: string | null;
}

interface RolloutRecord extends Record<string, unknown> {
  payload?: Record<string, unknown>;
  timestamp?: unknown;
  type?: unknown;
}

interface HistoryTurn {
  turnId?: string;
  events: AgentStreamEvent[];
  itemEventIndex: Map<string, number>;
  hasUserMessage: boolean;
  completed: boolean;
}

interface PendingResponseTool {
  callId: string;
  name: string;
  input: unknown;
  turn: HistoryTurn;
  timestamp?: string;
}

interface HistoryReadState {
  turns: HistoryTurn[];
  currentTurn?: HistoryTurn;
  pendingResponseTools: Map<string, PendingResponseTool>;
  metadataSeen: boolean;
  cwdFromMetadata: string | null;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isWithin(parent: string, candidate: string): boolean {
  const relative = path.relative(parent, candidate);
  return (
    relative === "" ||
    (!relative.startsWith(`..${path.sep}`) && relative !== ".." && !path.isAbsolute(relative))
  );
}

function rolloutNameMatches(name: string, sessionId: string): boolean {
  return name.startsWith("rollout-") && name.endsWith(`-${sessionId}.jsonl`);
}

function isNotFound(error: unknown): boolean {
  return isRecord(error) && error.code === "ENOENT";
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

async function collectRolloutMatches(
  directory: string,
  sessionId: string,
  matches: string[],
  depth: number,
): Promise<void> {
  if (depth > MAX_SESSION_TREE_DEPTH) return;

  let entries;
  try {
    entries = await fs.readdir(directory, { withFileTypes: true });
  } catch (error) {
    throw new Error(
      `Cannot search Codex transcript directory ${directory}: ${errorMessage(error)}`,
      { cause: error },
    );
  }

  for (const entry of entries) {
    const entryPath = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      await collectRolloutMatches(entryPath, sessionId, matches, depth + 1);
    } else if (entry.isFile() && rolloutNameMatches(entry.name, sessionId)) {
      matches.push(entryPath);
    }
  }
}

async function findCodexRollout(codexHome: string, sessionId: string): Promise<string> {
  let realHome: string;
  try {
    realHome = await fs.realpath(codexHome);
  } catch (error) {
    throw new Error(`Cannot access Codex home for session ${sessionId}: ${errorMessage(error)}`, {
      cause: error,
    });
  }

  const matches: string[] = [];
  for (const directoryName of ["sessions", "archived_sessions"]) {
    const directoryPath = path.join(realHome, directoryName);
    let realDirectory: string;
    try {
      realDirectory = await fs.realpath(directoryPath);
    } catch (error) {
      if (isNotFound(error)) continue;
      throw new Error(
        `Cannot search Codex ${directoryName} for session ${sessionId}: ${errorMessage(error)}`,
        { cause: error },
      );
    }
    if (!isWithin(realHome, realDirectory)) {
      throw new Error(`Codex ${directoryName} path escapes the Codex home`);
    }

    await collectRolloutMatches(realDirectory, sessionId, matches, 0);
  }

  if (matches.length === 0) {
    throw new Error(`No Codex transcript found for session ${sessionId}`);
  }
  if (matches.length > 1) {
    throw new Error(`Multiple Codex transcripts found for session ${sessionId}`);
  }
  return matches[0]!;
}

function sessionMetadataMatches(record: RolloutRecord, sessionId: string): boolean {
  if (record.type !== "session_meta" || !isRecord(record.payload)) return false;
  const identifiers = [record.payload.id, record.payload.session_id].filter(
    (value): value is string => typeof value === "string",
  );
  return identifiers.length > 0 && identifiers.every((value) => value === sessionId);
}

function turnFrom(turns: HistoryTurn[], turnId?: string, startsTurn = false): HistoryTurn {
  const current = turns.at(-1);
  if (current && (!turnId || current.turnId === turnId) && !startsTurn) {
    return current;
  }
  const turn: HistoryTurn = {
    ...(turnId ? { turnId } : {}),
    events: [],
    itemEventIndex: new Map(),
    hasUserMessage: false,
    completed: false,
  };
  turns.push(turn);
  return turn;
}

function timelineItemIdentity(item: AgentTimelineItem): string | null {
  if (item.type === "tool_call") return `tool:${item.callId}`;
  if (item.type === "user_message" || item.type === "assistant_message") {
    return item.messageId ? `${item.type}:id:${item.messageId}` : `${item.type}:text:${item.text}`;
  }
  return null;
}

function matchingMessageIndex(turn: HistoryTurn, item: AgentTimelineItem): number | undefined {
  if (item.type !== "user_message" && item.type !== "assistant_message") return undefined;
  const exactIndex = turn.itemEventIndex.get(timelineItemIdentity(item)!);
  if (exactIndex !== undefined) return exactIndex;

  const contentKey = `${item.type}:text:${item.text}`;
  const contentIndex = turn.itemEventIndex.get(contentKey);
  if (contentIndex === undefined) return undefined;
  const priorEvent = turn.events[contentIndex];
  if (!priorEvent || priorEvent.type !== "timeline") return undefined;
  const priorItem = priorEvent.item;
  if (priorItem.type !== item.type) return undefined;
  if (priorItem.type !== "user_message" && priorItem.type !== "assistant_message") return undefined;
  return priorItem.messageId && item.messageId && priorItem.messageId !== item.messageId
    ? undefined
    : contentIndex;
}

function preserveMessageIdentifiers(
  item: AgentTimelineItem,
  previous: AgentTimelineItem | undefined,
): AgentTimelineItem {
  if (item.type === "assistant_message" && previous?.type === "assistant_message") {
    return {
      ...item,
      ...(!item.messageId && previous.messageId ? { messageId: previous.messageId } : {}),
    };
  }
  if (item.type === "user_message" && previous?.type === "user_message") {
    return {
      ...item,
      ...(!item.messageId && previous.messageId ? { messageId: previous.messageId } : {}),
      ...(!item.clientMessageId && previous.clientMessageId
        ? { clientMessageId: previous.clientMessageId }
        : {}),
    };
  }
  return item;
}

function addTimelineItem(
  turn: HistoryTurn,
  item: AgentTimelineItem,
  timestamp: string | undefined,
  replaceExisting: boolean,
): void {
  const identity = timelineItemIdentity(item);
  const priorIndex =
    matchingMessageIndex(turn, item) ?? (identity ? turn.itemEventIndex.get(identity) : undefined);
  const previousEvent = priorIndex === undefined ? undefined : turn.events[priorIndex];
  const previousItem = previousEvent?.type === "timeline" ? previousEvent.item : undefined;
  const timelineItem = preserveMessageIdentifiers(item, previousItem);
  const event: AgentStreamEvent = {
    type: "timeline",
    provider: "codex",
    item: timelineItem,
    ...(turn.turnId ? { turnId: turn.turnId } : {}),
    ...(timestamp ? { timestamp } : {}),
  };

  const isMessage = item.type === "user_message" || item.type === "assistant_message";
  if ((replaceExisting || isMessage) && priorIndex !== undefined) {
    turn.events[priorIndex] = event;
    const updatedIdentity = timelineItemIdentity(timelineItem);
    if (updatedIdentity) turn.itemEventIndex.set(updatedIdentity, priorIndex);
    if (
      isMessage &&
      (timelineItem.type === "user_message" || timelineItem.type === "assistant_message")
    ) {
      turn.itemEventIndex.set(`${timelineItem.type}:text:${timelineItem.text}`, priorIndex);
    }
    return;
  }

  const eventIdentity = timelineItemIdentity(timelineItem);
  if (eventIdentity) turn.itemEventIndex.set(eventIdentity, turn.events.length);
  if (
    isMessage &&
    (timelineItem.type === "user_message" || timelineItem.type === "assistant_message")
  ) {
    turn.itemEventIndex.set(`${timelineItem.type}:text:${timelineItem.text}`, turn.events.length);
  }
  turn.events.push(event);
}

function readTimestamp(record: RolloutRecord): string | undefined {
  return normalizeProviderReplayTimestamp(record.timestamp) ?? undefined;
}

function readTurnId(payload: Record<string, unknown>): string | undefined {
  return typeof payload.turn_id === "string" && payload.turn_id.length > 0
    ? payload.turn_id
    : undefined;
}

function isVisibleCodexThreadItem(type: unknown): boolean {
  return (
    typeof type === "string" &&
    [
      "userMessage",
      "UserMessage",
      "agentMessage",
      "AgentMessage",
      "reasoning",
      "Reasoning",
      "commandExecution",
      "CommandExecution",
      "fileChange",
      "FileChange",
      "mcpToolCall",
      "McpToolCall",
      "webSearch",
      "WebSearch",
      "collabAgentToolCall",
      "CollabAgentToolCall",
      "subAgentActivity",
      "SubAgentActivity",
      "plan",
      "Plan",
    ].includes(type)
  );
}

async function mapThreadItem(item: unknown, cwd: string | null): Promise<AgentTimelineItem | null> {
  if (!isRecord(item) || !isVisibleCodexThreadItem(item.type)) return null;
  const { threadItemToTimeline } = await import("../codex-app-server-agent.js");
  return threadItemToTimeline(item, { cwd });
}

function runningToolCall(item: ToolCallTimelineItem): ToolCallTimelineItem {
  return {
    type: "tool_call",
    callId: item.callId,
    name: item.name,
    status: "running",
    error: null,
    detail: item.detail,
    ...(item.metadata ? { metadata: item.metadata } : {}),
  };
}

function rollbackTurns(turns: HistoryTurn[], count: unknown): void {
  if (typeof count !== "number" || !Number.isSafeInteger(count) || count <= 0) return;
  turns.splice(Math.max(0, turns.length - count), count);
}

function parseToolInput(value: unknown): unknown {
  if (typeof value !== "string") return value;
  try {
    return JSON.parse(value);
  } catch {
    return value;
  }
}

function responseCallId(payload: Record<string, unknown>): string | undefined {
  const value = payload.call_id ?? payload.callId;
  return typeof value === "string" && value.length > 0 ? value : undefined;
}

function recordResponseTool(
  payload: Record<string, unknown>,
  turns: HistoryTurn[],
  pending: Map<string, PendingResponseTool>,
  timestamp: string | undefined,
  cwd: string | null,
): void {
  const type = payload.type;
  const callId = responseCallId(payload);
  if (!callId) return;

  if (type === "function_call" || type === "custom_tool_call") {
    const name = typeof payload.name === "string" ? payload.name : "unknown_tool";
    const call: PendingResponseTool = {
      callId,
      name,
      input: parseToolInput(payload.arguments ?? payload.input ?? null),
      turn: turnFrom(turns),
      ...(timestamp ? { timestamp } : {}),
    };
    pending.set(callId, call);
    const item = mapCodexToolCallEnvelope({ callId, name, input: call.input, cwd });
    if (item) addTimelineItem(call.turn, runningToolCall(item), timestamp, true);
    return;
  }

  if (type !== "function_call_output" && type !== "custom_tool_call_output") return;

  const call = pending.get(callId) ?? {
    callId,
    name: "unknown_tool",
    input: null,
    turn: turnFrom(turns),
    ...(timestamp ? { timestamp } : {}),
  };
  pending.delete(callId);
  const item = mapCodexToolCallEnvelope({
    callId,
    name: call.name,
    input: call.input,
    output: payload.output ?? payload.content ?? null,
    error: payload.error,
    cwd,
  });
  if (item) addTimelineItem(call.turn, item, call.timestamp ?? timestamp, true);
}

async function mapEventTimelineItem(
  type: unknown,
  payload: Record<string, unknown>,
  cwd: string | null,
): Promise<AgentTimelineItem | null> {
  switch (type) {
    case "user_message":
      return mapThreadItem(
        {
          type: "userMessage",
          content: [
            { type: "text", text: typeof payload.message === "string" ? payload.message : "" },
          ],
          ...(typeof payload.id === "string" ? { id: payload.id } : {}),
          ...(typeof payload.client_id === "string" ? { clientId: payload.client_id } : {}),
        },
        cwd,
      );
    case "agent_message":
      return mapThreadItem(
        {
          type: "agentMessage",
          text: typeof payload.message === "string" ? payload.message : "",
          ...(typeof payload.id === "string" ? { id: payload.id } : {}),
        },
        cwd,
      );
    case "agent_reasoning":
      return mapThreadItem(
        {
          type: "reasoning",
          summary: [typeof payload.text === "string" ? payload.text : ""],
        },
        cwd,
      );
    case "agent_reasoning_raw_content":
      return mapThreadItem(
        {
          type: "reasoning",
          content: [typeof payload.text === "string" ? payload.text : ""],
        },
        cwd,
      );
    default:
      return null;
  }
}

function responseContentText(content: unknown): string {
  if (typeof content === "string") return content;
  if (!Array.isArray(content)) return "";
  return content
    .map((block) => (isRecord(block) && typeof block.text === "string" ? block.text : ""))
    .join("");
}

function responseItemToThreadItem(
  payload: Record<string, unknown>,
): Record<string, unknown> | null {
  if (payload.type === "message" && (payload.role === "user" || payload.role === "assistant")) {
    const id = typeof payload.id === "string" ? payload.id : payload.item_id;
    if (payload.role === "user") {
      return {
        type: "userMessage",
        content: [{ type: "text", text: responseContentText(payload.content) }],
        ...(typeof id === "string" ? { id } : {}),
      };
    }
    return {
      type: "agentMessage",
      text: responseContentText(payload.content),
      ...(typeof id === "string" ? { id } : {}),
    };
  }
  if (payload.type === "reasoning") {
    const readParts = (value: unknown): string[] =>
      Array.isArray(value)
        ? value.flatMap((part) => {
            if (typeof part === "string") return [part];
            return isRecord(part) && typeof part.text === "string" ? [part.text] : [];
          })
        : [];
    return {
      type: "reasoning",
      summary: readParts(payload.summary),
      content: readParts(payload.content),
    };
  }
  return null;
}

async function processResponseItem(
  payload: Record<string, unknown>,
  timestamp: string | undefined,
  state: HistoryReadState,
  cwd: string | null,
): Promise<void> {
  const threadItem = responseItemToThreadItem(payload);
  const item = await mapThreadItem(threadItem, cwd);
  if (!item) return;

  const current = state.turns.at(-1);
  const hasMatchingMessage = current && matchingMessageIndex(current, item) !== undefined;
  const startNewTurn =
    item.type === "user_message" &&
    !hasMatchingMessage &&
    Boolean(current && (current.completed || current.hasUserMessage));
  const turn = turnFrom(state.turns, readTurnId(payload), startNewTurn);
  if (item.type === "user_message") turn.hasUserMessage = true;
  addTimelineItem(turn, item, timestamp, true);
}

function pruneRolledBackTools(
  turns: HistoryTurn[],
  pendingResponseTools: Map<string, PendingResponseTool>,
): void {
  for (const [callId, call] of pendingResponseTools) {
    if (!turns.includes(call.turn)) pendingResponseTools.delete(callId);
  }
}

async function processEventMessage(
  turns: HistoryTurn[],
  payload: Record<string, unknown>,
  cwd: string | null,
): Promise<{
  turn?: HistoryTurn;
  completed?: boolean;
  rolledBack?: boolean;
  item?: AgentTimelineItem | null;
  replaceExisting?: boolean;
}> {
  const type = payload.type;
  const turnId = readTurnId(payload);

  switch (type) {
    case "thread_rolled_back":
      rollbackTurns(turns, payload.num_turns);
      return { rolledBack: true };
    case "task_started":
    case "turn_started":
      return { turn: turnFrom(turns, turnId, !turnId || turns.at(-1)?.turnId !== turnId) };
    case "task_complete":
    case "turn_complete": {
      const turn = turnFrom(turns, turnId);
      turn.completed = true;
      return { turn, completed: true };
    }
    case "user_message": {
      const current = turns.at(-1);
      const item = await mapEventTimelineItem(type, payload, cwd);
      const duplicate = current && item && matchingMessageIndex(current, item) !== undefined;
      const startNew =
        !current || current.completed || (!turnId && current.hasUserMessage && !duplicate);
      const turn = turnFrom(turns, turnId, startNew);
      turn.hasUserMessage = true;
      return { turn, item };
    }
    case "agent_message":
      return {
        turn: turnFrom(turns, turnId),
        item: await mapEventTimelineItem(type, payload, cwd),
        replaceExisting: true,
      };
    case "agent_reasoning":
    case "agent_reasoning_raw_content":
      return {
        turn: turnFrom(turns, turnId),
        item: await mapEventTimelineItem(type, payload, cwd),
      };
    case "item_started":
    case "item_completed":
      return { turn: turnFrom(turns, turnId) };
    default:
      return {};
  }
}

function parseRolloutLine(line: string, lineNumber: number, sessionId: string): RolloutRecord {
  let value: unknown;
  try {
    value = JSON.parse(line);
  } catch {
    throw new Error(`Invalid Codex transcript JSON at line ${lineNumber} for session ${sessionId}`);
  }
  if (!isRecord(value)) {
    throw new Error(
      `Invalid Codex transcript record at line ${lineNumber} for session ${sessionId}`,
    );
  }
  return value as RolloutRecord;
}

async function processEventRecord(
  payload: Record<string, unknown>,
  timestamp: string | undefined,
  state: HistoryReadState,
  cwd: string | null,
): Promise<void> {
  const result = await processEventMessage(state.turns, payload, cwd);
  if (result.rolledBack) {
    state.currentTurn = undefined;
    pruneRolledBackTools(state.turns, state.pendingResponseTools);
    return;
  }

  state.currentTurn = result.turn ?? state.currentTurn;
  if (result.item && result.turn) {
    addTimelineItem(result.turn, result.item, timestamp, result.replaceExisting ?? false);
  }

  if (payload.type === "item_started" || payload.type === "item_completed") {
    const item = await mapThreadItem(payload.item, cwd);
    if (item && state.currentTurn && !isMessageOrReasoningItem(item)) {
      addTimelineItem(state.currentTurn, item, timestamp, true);
    }
  }
  if (result.completed && state.currentTurn) state.currentTurn.completed = true;
}

function isMessageOrReasoningItem(item: AgentTimelineItem): boolean {
  return (
    item.type === "user_message" || item.type === "assistant_message" || item.type === "reasoning"
  );
}

async function processRolloutRecord(
  record: RolloutRecord,
  sessionId: string,
  state: HistoryReadState,
  cwd: string | null,
): Promise<void> {
  if (!state.metadataSeen) {
    if (!sessionMetadataMatches(record, sessionId)) {
      throw new Error(`Codex transcript metadata does not match session ${sessionId}`);
    }
    state.metadataSeen = true;
    state.cwdFromMetadata = typeof record.payload?.cwd === "string" ? record.payload.cwd : null;
    return;
  }

  if (record.type === "session_meta") {
    if (!sessionMetadataMatches(record, sessionId)) {
      throw new Error(`Codex transcript metadata does not match session ${sessionId}`);
    }
    return;
  }

  const payload = isRecord(record.payload) ? record.payload : null;
  if (!payload) return;
  const eventCwd = cwd ?? state.cwdFromMetadata;
  const timestamp = readTimestamp(record);

  if (record.type === "response_item") {
    recordResponseTool(payload, state.turns, state.pendingResponseTools, timestamp, eventCwd);
    await processResponseItem(payload, timestamp, state, eventCwd);
    return;
  }
  if (record.type === "turn_context") {
    state.currentTurn = turnFrom(state.turns, readTurnId(payload));
    return;
  }
  if (record.type === "event_msg") {
    await processEventRecord(payload, timestamp, state, eventCwd);
  }
}

async function readCodexTranscript(
  transcriptPath: string,
  sessionId: string,
  cwd: string | null,
): Promise<AgentStreamEvent[]> {
  const stream = createReadStream(transcriptPath, { encoding: "utf8" });
  const lines = createInterface({ input: stream, crlfDelay: Infinity });
  const state: HistoryReadState = {
    turns: [],
    pendingResponseTools: new Map(),
    metadataSeen: false,
    cwdFromMetadata: null,
  };
  let lineNumber = 0;

  try {
    for await (let line of lines) {
      lineNumber += 1;
      if (lineNumber === 1) line = line.replace(/^\uFEFF/, "");
      if (line.trim().length === 0) continue;
      const record = parseRolloutLine(line, lineNumber, sessionId);
      await processRolloutRecord(record, sessionId, state, cwd);
    }
  } catch (error) {
    if (error instanceof Error && error.message.startsWith("Codex transcript")) throw error;
    throw new Error(
      `Cannot read Codex transcript for session ${sessionId}: ${errorMessage(error)}`,
      { cause: error },
    );
  } finally {
    lines.close();
    stream.destroy();
  }

  if (!state.metadataSeen) {
    throw new Error(`Codex transcript metadata is missing for session ${sessionId}`);
  }
  return state.turns.flatMap((turn) => turn.events);
}

export async function readCodexOfflineHistory({
  sessionId,
  codexHome,
  cwd,
}: CodexOfflineHistoryInput): Promise<AgentStreamEvent[]> {
  if (!SESSION_ID_PATTERN.test(sessionId)) {
    throw new Error(`Invalid Codex session id: ${sessionId}`);
  }

  const transcriptPath = await findCodexRollout(codexHome, sessionId);
  const canonicalTranscriptPath = await fs.realpath(transcriptPath).catch((error: unknown) => {
    throw new Error(
      `Cannot read Codex transcript for session ${sessionId}: ${errorMessage(error)}`,
      { cause: error },
    );
  });
  const canonicalHome = await fs.realpath(codexHome);
  if (!isWithin(canonicalHome, canonicalTranscriptPath)) {
    throw new Error(`Codex transcript path escapes the Codex home for session ${sessionId}`);
  }
  return readCodexTranscript(canonicalTranscriptPath, sessionId, cwd ?? null);
}
