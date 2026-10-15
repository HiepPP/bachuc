import { rmSync } from "node:fs";
import { join } from "node:path";
import type { Logger } from "pino";
import { timelineItemIdentity } from "@getpaseo/protocol/timeline-identity";
import type { AgentTimelineItem } from "./agent-sdk-types.js";
import type {
  AgentTimelineRow,
  DurableTimelineMeta,
  DurableTimelineSearchCandidate,
  DurableTimelineSnapshot,
  DurableTimelineStore,
  TimelineFlushListener,
} from "./agent-timeline-store-types.js";
import type { ProviderSubagentDescriptor } from "./provider-subagents/store.js";

// @types/node@20 predates the node:sqlite typings; declare the slice we use.
type SqliteValue = string | number | bigint | null | Uint8Array;
type SqliteRow = Record<string, SqliteValue>;
interface SqliteStatement {
  run(...params: SqliteValue[]): unknown;
  get(...params: SqliteValue[]): SqliteRow | undefined;
  all(...params: SqliteValue[]): SqliteRow[];
}
interface SqliteDatabase {
  exec(sql: string): void;
  prepare(sql: string): SqliteStatement;
  close(): void;
}
interface NodeSqliteModule {
  DatabaseSync: new (path: string) => SqliteDatabase;
}

const SCHEMA_VERSION = 4;
const FLUSH_DELAY_MS = 100;
const FLUSH_ROW_THRESHOLD = 256;
const ROWS_PER_TRANSACTION = 500;
const LOAD_PAGE_ROWS = 2_000;
const CACHE_FILE_NAME = "timeline-cache.sqlite";

// timeline_messages holds each projected user or assistant message: assistant chunks merge by
// the same rules as timeline-projection.ts, so a word split across two chunks stays one token.
// timeline_search indexes that text by message id; it holds no copy of the text.
const SCHEMA_SQL = `
CREATE TABLE IF NOT EXISTS timelines (
  agent_id           TEXT PRIMARY KEY,
  epoch              TEXT NOT NULL,
  next_seq           INTEGER NOT NULL,
  complete           INTEGER NOT NULL DEFAULT 0,
  provider_subagents TEXT,
  transcript_fingerprint TEXT,
  updated_at         TEXT NOT NULL
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS timeline_rows (
  agent_id            TEXT NOT NULL,
  seq                 INTEGER NOT NULL,
  timestamp           TEXT NOT NULL,
  turn_id             TEXT,
  provider_message_id TEXT,
  item_type           TEXT NOT NULL,
  identity            TEXT,
  item                TEXT NOT NULL,
  PRIMARY KEY (agent_id, seq)
) WITHOUT ROWID;

CREATE INDEX IF NOT EXISTS timeline_rows_by_type ON timeline_rows (agent_id, item_type, seq);

CREATE INDEX IF NOT EXISTS timeline_rows_by_identity ON timeline_rows (agent_id, identity, seq)
  WHERE identity IS NOT NULL;

CREATE TABLE IF NOT EXISTS timeline_messages (
  id         INTEGER PRIMARY KEY,
  agent_id   TEXT NOT NULL,
  seq        INTEGER NOT NULL,
  end_seq    INTEGER NOT NULL,
  role       TEXT NOT NULL,
  turn_id    TEXT,
  message_id TEXT,
  text       TEXT NOT NULL,
  UNIQUE (agent_id, seq)
);

CREATE INDEX IF NOT EXISTS timeline_messages_by_end ON timeline_messages (agent_id, end_seq);

CREATE VIRTUAL TABLE IF NOT EXISTS timeline_search USING fts5(
  text, content='', contentless_delete=1, tokenize='unicode61 remove_diacritics 2'
);
`;

class TimelineCacheSchemaError extends Error {
  constructor(
    public readonly path: string,
    public readonly detail: string,
  ) {
    super(`Timeline cache ${path} is unusable: ${detail}`);
    this.name = "TimelineCacheSchemaError";
  }
}

interface AgentBuffer {
  /** Keyed by seq so an update replaces a pending append of the same row. */
  rows: Map<number, AgentTimelineRow>;
  providerSubagents: readonly ProviderSubagentDescriptor[] | null;
}

interface Statements {
  getMeta: SqliteStatement;
  upsertRow: SqliteStatement;
  advanceNextSeq: SqliteStatement;
  setProviderSubagents: SqliteStatement;
  setComplete: SqliteStatement;
  deleteRows: SqliteStatement;
  deleteMeta: SqliteStatement;
  upsertMeta: SqliteStatement;
  loadPage: SqliteStatement;
  loadRange: SqliteStatement;
  loadIdentityRows: SqliteStatement;
  listPromptRows: SqliteStatement;
  maxMessageEnd: SqliteStatement;
  messageAtOrBefore: SqliteStatement;
  messageEndingAt: SqliteStatement;
  insertMessage: SqliteStatement;
  updateMessage: SqliteStatement;
  deleteMessage: SqliteStatement;
  deleteMessages: SqliteStatement;
  deleteSearchText: SqliteStatement;
  deleteSearchTexts: SqliteStatement;
  insertSearchText: SqliteStatement;
  searchMessages: SqliteStatement;
  listMessages: SqliteStatement;
}

const MESSAGE_COLUMNS = "id, seq, end_seq, role, turn_id, message_id, text";

function prepareStatements(db: SqliteDatabase): Statements {
  return {
    getMeta: db.prepare(
      `SELECT epoch, next_seq, complete, provider_subagents, transcript_fingerprint
       FROM timelines WHERE agent_id = ?`,
    ),
    upsertRow: db.prepare(
      `INSERT OR REPLACE INTO timeline_rows
         (agent_id, seq, timestamp, turn_id, provider_message_id, item_type, identity, item)
       VALUES (?, ?, ?, ?, ?, ?, ?, ?)`,
    ),
    loadRange: db.prepare(
      `SELECT seq, timestamp, turn_id, provider_message_id, item FROM timeline_rows
       WHERE agent_id = ? AND seq >= ? AND seq <= ? ORDER BY seq LIMIT ?`,
    ),
    loadIdentityRows: db.prepare(
      `SELECT seq, timestamp, turn_id, provider_message_id, item FROM timeline_rows
       WHERE agent_id = ? AND identity IN (SELECT value FROM json_each(?)) AND seq < ?
       ORDER BY seq`,
    ),
    listPromptRows: db.prepare(
      `SELECT seq, timestamp, turn_id, provider_message_id, item FROM timeline_rows
       WHERE agent_id = ? AND item_type = 'user_message' ORDER BY seq`,
    ),
    maxMessageEnd: db.prepare(
      "SELECT MAX(end_seq) AS end_seq FROM timeline_messages WHERE agent_id = ?",
    ),
    messageAtOrBefore: db.prepare(
      `SELECT ${MESSAGE_COLUMNS} FROM timeline_messages
       WHERE agent_id = ? AND seq <= ? ORDER BY seq DESC LIMIT 1`,
    ),
    messageEndingAt: db.prepare(
      `SELECT ${MESSAGE_COLUMNS} FROM timeline_messages WHERE agent_id = ? AND end_seq = ?`,
    ),
    insertMessage: db.prepare(
      `INSERT INTO timeline_messages (agent_id, seq, end_seq, role, turn_id, message_id, text)
       VALUES (?, ?, ?, ?, ?, ?, ?) RETURNING id`,
    ),
    updateMessage: db.prepare("UPDATE timeline_messages SET end_seq = ?, text = ? WHERE id = ?"),
    deleteMessage: db.prepare("DELETE FROM timeline_messages WHERE id = ?"),
    deleteMessages: db.prepare("DELETE FROM timeline_messages WHERE agent_id = ?"),
    deleteSearchText: db.prepare("DELETE FROM timeline_search WHERE rowid = ?"),
    deleteSearchTexts: db.prepare(
      `DELETE FROM timeline_search
       WHERE rowid IN (SELECT id FROM timeline_messages WHERE agent_id = ?)`,
    ),
    insertSearchText: db.prepare("INSERT INTO timeline_search (rowid, text) VALUES (?, ?)"),
    searchMessages: db.prepare(
      `SELECT m.end_seq, m.role, m.text FROM timeline_search
       JOIN timeline_messages AS m ON m.id = timeline_search.rowid
       WHERE timeline_search MATCH ? AND m.agent_id = ? AND m.end_seq > ?
       ORDER BY m.end_seq LIMIT ?`,
    ),
    listMessages: db.prepare(
      `SELECT end_seq, role, text FROM timeline_messages
       WHERE agent_id = ? AND end_seq > ? ORDER BY end_seq LIMIT ?`,
    ),
    advanceNextSeq: db.prepare(
      "UPDATE timelines SET next_seq = MAX(next_seq, ?), updated_at = ? WHERE agent_id = ?",
    ),
    setProviderSubagents: db.prepare(
      "UPDATE timelines SET provider_subagents = ?, updated_at = ? WHERE agent_id = ?",
    ),
    setComplete: db.prepare(
      `UPDATE timelines SET complete = ?, transcript_fingerprint = ?, updated_at = ?
       WHERE agent_id = ?`,
    ),
    deleteRows: db.prepare("DELETE FROM timeline_rows WHERE agent_id = ?"),
    deleteMeta: db.prepare("DELETE FROM timelines WHERE agent_id = ?"),
    upsertMeta: db.prepare(
      `INSERT OR REPLACE INTO timelines
         (agent_id, epoch, next_seq, complete, provider_subagents, updated_at)
       VALUES (?, ?, 1, 0, NULL, ?)`,
    ),
    loadPage: db.prepare(
      `SELECT seq, timestamp, turn_id, provider_message_id, item FROM timeline_rows
       WHERE agent_id = ? AND seq > ? ORDER BY seq LIMIT ?`,
    ),
  };
}

function requireString(row: SqliteRow, column: string): string {
  const value = row[column];
  if (typeof value !== "string") {
    throw new TimelineCacheSchemaError(CACHE_FILE_NAME, `column ${column} is not text`);
  }
  return value;
}

function requireNumber(row: SqliteRow, column: string): number {
  const value = row[column];
  if (typeof value !== "number") {
    throw new TimelineCacheSchemaError(CACHE_FILE_NAME, `column ${column} is not an integer`);
  }
  return value;
}

function optionalString(row: SqliteRow, column: string): string | null {
  const value = row[column];
  return typeof value === "string" ? value : null;
}

function parseItem(text: string): AgentTimelineItem {
  // The cache holds only rows this store serialized; quick_check guards file damage.
  const item: AgentTimelineItem = JSON.parse(text);
  return item;
}

function parseProviderSubagents(text: string | null): ProviderSubagentDescriptor[] {
  if (text === null) return [];
  const descriptors: ProviderSubagentDescriptor[] = JSON.parse(text);
  return descriptors;
}

function rowFromSqlite(row: SqliteRow): AgentTimelineRow {
  const turnId = optionalString(row, "turn_id");
  const providerMessageId = optionalString(row, "provider_message_id");
  return {
    seq: requireNumber(row, "seq"),
    timestamp: requireString(row, "timestamp"),
    item: parseItem(requireString(row, "item")),
    ...(turnId === null ? {} : { turnId }),
    ...(providerMessageId === null ? {} : { providerMessageId }),
  };
}

function yieldToEventLoop(): Promise<void> {
  return new Promise((resolve) => setImmediate(resolve));
}

/** One projected user or assistant message, as stored in timeline_messages. */
interface IndexedMessage {
  /** Null until the message is first saved. */
  id: number | null;
  seq: number;
  endSeq: number;
  role: "user" | "assistant";
  turnId: string | null;
  messageId: string | null;
  text: string;
}

function messageFromSqlite(row: SqliteRow): IndexedMessage {
  return {
    id: requireNumber(row, "id"),
    seq: requireNumber(row, "seq"),
    endSeq: requireNumber(row, "end_seq"),
    role: requireString(row, "role") === "user" ? "user" : "assistant",
    turnId: optionalString(row, "turn_id"),
    messageId: optionalString(row, "message_id"),
    text: requireString(row, "text"),
  };
}

function candidateFromSqlite(row: SqliteRow): DurableTimelineSearchCandidate {
  return {
    seq: requireNumber(row, "end_seq"),
    role: requireString(row, "role") === "user" ? "user" : "assistant",
    text: requireString(row, "text"),
  };
}

function startMessage(row: AgentTimelineRow): IndexedMessage | null {
  const { item } = row;
  if (item.type !== "user_message" && item.type !== "assistant_message") return null;
  return {
    id: null,
    seq: row.seq,
    endSeq: row.seq,
    role: item.type === "user_message" ? "user" : "assistant",
    turnId: row.turnId ?? null,
    messageId: item.type === "assistant_message" ? item.messageId || null : null,
    text: item.text,
  };
}

/** The chunk text when `row` continues `message`; mirrors mergeAssistantChunks. */
function continuationText(message: IndexedMessage, row: AgentTimelineRow): string | null {
  const { item } = row;
  if (message.role !== "assistant" || item.type !== "assistant_message") return null;
  if (message.endSeq + 1 !== row.seq || message.turnId !== (row.turnId ?? null)) return null;
  if (item.messageId !== undefined && item.messageId !== message.messageId) return null;
  return item.text;
}

/** unicode61 folds combining marks but not đ, which Vietnamese readers type as d. */
function searchText(text: string): string {
  return text.normalize("NFC").replace(/đ/g, "d").replace(/Đ/g, "D");
}

/**
 * Each query word as a prefix term, so candidates cover every block-text match that starts at
 * a word. Null when the query has no word to look up; every message is then a candidate.
 */
function searchExpression(query: string): string | null {
  const tokens = searchText(query).match(/[\p{L}\p{N}\p{M}]+/gu) ?? [];
  if (tokens.length === 0) return null;
  return tokens.map((token) => `"${token}"*`).join(" AND ");
}

export class SqliteTimelineStore implements DurableTimelineStore {
  private readonly statements: Statements;
  private readonly buffers = new Map<string, AgentBuffer>();
  /** Agents whose rows were lost by a failed flush; never served until the next reset. */
  private readonly untrusted = new Set<string>();
  private bufferedRowCount = 0;
  private flushListener: TimelineFlushListener | null = null;
  private flushTimer: ReturnType<typeof setTimeout> | null = null;
  private flushImmediate: ReturnType<typeof setImmediate> | null = null;
  private closed = false;

  constructor(
    private readonly db: SqliteDatabase,
    private readonly logger: Logger,
  ) {
    this.statements = prepareStatements(db);
  }

  getMeta(agentId: string): DurableTimelineMeta | null {
    const row = this.statements.getMeta.get(agentId);
    return row ? this.metaFromSqlite(agentId, row) : null;
  }

  async load(agentId: string): Promise<DurableTimelineSnapshot | null> {
    this.flushAgent(agentId);
    const metaRow = this.statements.getMeta.get(agentId);
    if (!metaRow) return null;
    const rows: AgentTimelineRow[] = [];
    let afterSeq = 0;
    for (;;) {
      const page = this.statements.loadPage.all(agentId, afterSeq, LOAD_PAGE_ROWS);
      for (const row of page) rows.push(rowFromSqlite(row));
      if (page.length < LOAD_PAGE_ROWS) break;
      afterSeq = rows[rows.length - 1].seq;
      await yieldToEventLoop();
    }
    return {
      ...this.metaFromSqlite(agentId, metaRow),
      rows,
      providerSubagents: parseProviderSubagents(optionalString(metaRow, "provider_subagents")),
    };
  }

  private metaFromSqlite(agentId: string, row: SqliteRow): DurableTimelineMeta {
    const transcriptFingerprint = optionalString(row, "transcript_fingerprint");
    return {
      epoch: requireString(row, "epoch"),
      nextSeq: requireNumber(row, "next_seq"),
      complete: requireNumber(row, "complete") === 1 && !this.untrusted.has(agentId),
      ...(transcriptFingerprint === null ? {} : { transcriptFingerprint }),
    };
  }

  /** User message rows in seq order; null when lost writes keep the agent's rows from reads. */
  listPromptRows(agentId: string): AgentTimelineRow[] | null {
    if (!this.flushForRead(agentId)) return null;
    return this.statements.listPromptRows.all(agentId).map(rowFromSqlite);
  }

  findSearchCandidates(
    agentId: string,
    query: string,
    afterSeq: number,
    limit: number,
  ): DurableTimelineSearchCandidate[] | null {
    if (!this.flushForRead(agentId)) return null;
    const expression = searchExpression(query);
    const rows =
      expression === null
        ? this.statements.listMessages.all(agentId, afterSeq, limit)
        : this.statements.searchMessages.all(expression, agentId, afterSeq, limit);
    return rows.map(candidateFromSqlite);
  }

  readRowWindow(agentId: string, fromSeq: number, toSeq: number): AgentTimelineRow[] | null {
    if (!this.flushForRead(agentId)) return null;
    return this.loadRows(agentId, fromSeq, toSeq);
  }

  readIdentityRows(
    agentId: string,
    identities: readonly string[],
    beforeSeq: number,
  ): AgentTimelineRow[] | null {
    if (!this.flushForRead(agentId)) return null;
    if (identities.length === 0) return [];
    const json = JSON.stringify(identities);
    return this.statements.loadIdentityRows.all(agentId, json, beforeSeq).map(rowFromSqlite);
  }

  setFlushListener(listener: TimelineFlushListener | null): void {
    this.flushListener = listener;
  }

  reset(agentId: string, epoch: string): void {
    this.takeBuffer(agentId);
    this.transaction(() => {
      this.deleteIndexedMessages(agentId);
      this.statements.deleteRows.run(agentId);
      this.statements.upsertMeta.run(agentId, epoch, new Date().toISOString());
    });
    this.untrusted.delete(agentId);
  }

  append(agentId: string, rows: readonly AgentTimelineRow[]): void {
    const buffer = this.bufferFor(agentId);
    for (const row of rows) {
      if (!buffer.rows.has(row.seq)) this.bufferedRowCount += 1;
      buffer.rows.set(row.seq, { ...row });
    }
    this.scheduleFlush();
  }

  update(agentId: string, row: AgentTimelineRow): void {
    this.append(agentId, [row]);
  }

  setProviderSubagents(agentId: string, descriptors: readonly ProviderSubagentDescriptor[]): void {
    this.bufferFor(agentId).providerSubagents = [...descriptors];
    this.scheduleFlush();
  }

  setComplete(agentId: string, complete: boolean, transcriptFingerprint?: string | null): void {
    const buffer = this.takeBuffer(agentId);
    const ok = this.writeBuffers(buffer ? new Map([[agentId, buffer]]) : new Map(), () => {
      const trusted = complete && !this.untrusted.has(agentId);
      const fingerprint = trusted ? (transcriptFingerprint ?? null) : null;
      const now = new Date().toISOString();
      this.statements.setComplete.run(trusted ? 1 : 0, fingerprint, now, agentId);
    });
    if (!ok) this.writeIncomplete([agentId]);
  }

  deleteAgent(agentId: string): void {
    this.takeBuffer(agentId);
    this.transaction(() => {
      this.deleteIndexedMessages(agentId);
      this.statements.deleteRows.run(agentId);
      this.statements.deleteMeta.run(agentId);
    });
    this.untrusted.delete(agentId);
  }

  flush(): void {
    this.cancelScheduledFlush();
    while (this.bufferedRowCount > 0 || this.hasPendingMeta()) this.flushChunk();
  }

  close(): void {
    if (this.closed) return;
    this.flush();
    this.closed = true;
    this.db.close();
  }

  private bufferFor(agentId: string): AgentBuffer {
    let buffer = this.buffers.get(agentId);
    if (!buffer) {
      buffer = { rows: new Map(), providerSubagents: null };
      this.buffers.set(agentId, buffer);
    }
    return buffer;
  }

  private takeBuffer(agentId: string): AgentBuffer | undefined {
    const buffer = this.buffers.get(agentId);
    if (!buffer) return undefined;
    this.buffers.delete(agentId);
    this.bufferedRowCount -= buffer.rows.size;
    return buffer;
  }

  private hasPendingMeta(): boolean {
    for (const buffer of this.buffers.values()) {
      if (buffer.providerSubagents !== null) return true;
    }
    return false;
  }

  private flushAgent(agentId: string): void {
    const buffer = this.takeBuffer(agentId);
    if (!buffer) return;
    if (!this.writeBuffers(new Map([[agentId, buffer]]))) this.writeIncomplete([agentId]);
  }

  /** Rows with seq in [fromSeq, toSeq], read in pages of LOAD_PAGE_ROWS. */
  private loadRows(agentId: string, fromSeq: number, toSeq: number): AgentTimelineRow[] {
    const rows: AgentTimelineRow[] = [];
    let nextSeq = fromSeq;
    for (;;) {
      const page = this.statements.loadRange.all(agentId, nextSeq, toSeq, LOAD_PAGE_ROWS);
      for (const row of page) rows.push(rowFromSqlite(row));
      if (page.length < LOAD_PAGE_ROWS) return rows;
      nextSeq = rows[rows.length - 1].seq + 1;
    }
  }

  /** Flushes the agent's buffer; false when a lost write means its rows cannot serve reads. */
  private flushForRead(agentId: string): boolean {
    this.flushAgent(agentId);
    return !this.untrusted.has(agentId);
  }

  private deleteIndexedMessages(agentId: string): void {
    this.statements.deleteSearchTexts.run(agentId);
    this.statements.deleteMessages.run(agentId);
  }

  /** Keeps timeline_messages and timeline_search in step with rows written in this transaction. */
  private indexMessages(agentId: string, rows: readonly AgentTimelineRow[]): void {
    if (rows.length === 0) return;
    const sorted = [...rows].sort((left, right) => left.seq - right.seq);
    const maxEnd = this.statements.maxMessageEnd.get(agentId)?.["end_seq"];
    const lastIndexedSeq = typeof maxEnd === "number" ? maxEnd : 0;
    let open: IndexedMessage | null = null;
    let rebuiltThrough = 0;
    for (const row of sorted) {
      if (row.seq <= rebuiltThrough) continue;
      // A row at or below lastIndexedSeq may already belong to a stored message.
      const chunk = open && row.seq > lastIndexedSeq ? continuationText(open, row) : null;
      if (open && chunk !== null) {
        open.text += chunk;
        open.endSeq = row.seq;
        continue;
      }
      if (open) this.saveMessage(agentId, open);
      open = null;
      const covering =
        row.seq <= lastIndexedSeq ? this.statements.messageAtOrBefore.get(agentId, row.seq) : null;
      if (covering && requireNumber(covering, "end_seq") >= row.seq) {
        // A rewritten row, such as an enriched user message: rebuild its message from storage.
        const message = messageFromSqlite(covering);
        rebuiltThrough = message.endSeq;
        this.deleteMessage(message);
        this.indexMessages(agentId, this.loadRows(agentId, message.seq, message.endSeq));
        continue;
      }
      const previousRow =
        row.item.type === "assistant_message"
          ? this.statements.messageEndingAt.get(agentId, row.seq - 1)
          : undefined;
      const previous = previousRow ? messageFromSqlite(previousRow) : null;
      const previousChunk = previous ? continuationText(previous, row) : null;
      if (previous && previousChunk !== null) {
        previous.text += previousChunk;
        previous.endSeq = row.seq;
        open = previous;
      } else {
        open = startMessage(row);
      }
    }
    if (open) this.saveMessage(agentId, open);
  }

  private saveMessage(agentId: string, message: IndexedMessage): void {
    let id = message.id;
    if (id === null) {
      const inserted = this.statements.insertMessage.get(
        agentId,
        message.seq,
        message.endSeq,
        message.role,
        message.turnId,
        message.messageId,
        message.text,
      );
      if (!inserted) throw new TimelineCacheSchemaError(CACHE_FILE_NAME, "message insert failed");
      id = requireNumber(inserted, "id");
    } else {
      this.statements.updateMessage.run(message.endSeq, message.text, id);
      this.statements.deleteSearchText.run(id);
    }
    this.statements.insertSearchText.run(id, searchText(message.text));
  }

  private deleteMessage(message: IndexedMessage): void {
    if (message.id === null) return;
    this.statements.deleteSearchText.run(message.id);
    this.statements.deleteMessage.run(message.id);
  }

  private scheduleFlush(): void {
    if (this.closed || this.flushImmediate) return;
    if (this.bufferedRowCount >= FLUSH_ROW_THRESHOLD) {
      this.cancelScheduledFlush();
      this.flushImmediate = setImmediate(() => this.runScheduledFlush());
      return;
    }
    if (!this.flushTimer) {
      this.flushTimer = setTimeout(() => this.runScheduledFlush(), FLUSH_DELAY_MS);
    }
  }

  private cancelScheduledFlush(): void {
    if (this.flushTimer) clearTimeout(this.flushTimer);
    if (this.flushImmediate) clearImmediate(this.flushImmediate);
    this.flushTimer = null;
    this.flushImmediate = null;
  }

  private runScheduledFlush(): void {
    this.cancelScheduledFlush();
    if (this.closed) return;
    this.flushChunk();
    if (this.bufferedRowCount > 0 || this.hasPendingMeta()) {
      this.flushImmediate = setImmediate(() => this.runScheduledFlush());
    }
  }

  /** Writes at most ROWS_PER_TRANSACTION rows from the buffers in one transaction. */
  private flushChunk(): void {
    const chunk = new Map<string, AgentBuffer>();
    let budget = ROWS_PER_TRANSACTION;
    for (const [agentId, buffer] of this.buffers) {
      if (budget === 0) break;
      const taken: AgentBuffer = { rows: new Map(), providerSubagents: null };
      for (const [seq, row] of buffer.rows) {
        if (budget === 0) break;
        taken.rows.set(seq, row);
        buffer.rows.delete(seq);
        budget -= 1;
      }
      if (buffer.rows.size === 0) {
        taken.providerSubagents = buffer.providerSubagents;
        this.buffers.delete(agentId);
      }
      this.bufferedRowCount -= taken.rows.size;
      chunk.set(agentId, taken);
    }
    if (!this.writeBuffers(chunk)) this.writeIncomplete([...chunk.keys()]);
  }

  /** Returns false when the transaction failed and the buffered writes were lost. */
  private writeBuffers(buffers: Map<string, AgentBuffer>, after?: () => void): boolean {
    const now = new Date().toISOString();
    const flushed: Array<[agentId: string, maxSeq: number]> = [];
    try {
      this.transaction(() => {
        for (const [agentId, buffer] of buffers) {
          let maxSeq = 0;
          for (const row of buffer.rows.values()) {
            this.statements.upsertRow.run(
              agentId,
              row.seq,
              row.timestamp,
              row.turnId ?? null,
              row.providerMessageId ?? null,
              row.item.type,
              timelineItemIdentity(row.item),
              JSON.stringify(row.item),
            );
            maxSeq = Math.max(maxSeq, row.seq);
          }
          this.indexMessages(agentId, [...buffer.rows.values()]);
          if (maxSeq > 0) {
            this.statements.advanceNextSeq.run(maxSeq + 1, now, agentId);
            flushed.push([agentId, maxSeq]);
          }
          if (buffer.providerSubagents !== null) {
            const json = JSON.stringify(buffer.providerSubagents);
            this.statements.setProviderSubagents.run(json, now, agentId);
          }
        }
        after?.();
      });
    } catch (err) {
      this.logger.error({ err, agentIds: [...buffers.keys()] }, "Timeline cache write failed");
      return false;
    }
    for (const [agentId, maxSeq] of flushed) {
      try {
        this.flushListener?.(agentId, maxSeq);
      } catch (err) {
        this.logger.error({ err, agentId }, "Timeline cache flush listener failed");
      }
    }
    return true;
  }

  private writeIncomplete(agentIds: readonly string[]): void {
    const now = new Date().toISOString();
    for (const agentId of agentIds) {
      this.untrusted.add(agentId);
      try {
        this.statements.setComplete.run(0, null, now, agentId);
      } catch (err) {
        this.logger.error({ err, agentId }, "Timeline cache could not mark agent incomplete");
      }
    }
  }

  private transaction(write: () => void): void {
    this.db.exec("BEGIN IMMEDIATE");
    try {
      write();
      this.db.exec("COMMIT");
    } catch (err) {
      this.db.exec("ROLLBACK");
      throw err;
    }
  }
}

function removeCacheFiles(path: string): void {
  for (const suffix of ["", "-wal", "-shm"]) rmSync(`${path}${suffix}`, { force: true });
}

function openDatabase(sqlite: NodeSqliteModule, path: string): SqliteDatabase {
  const db = new sqlite.DatabaseSync(path);
  try {
    db.exec("PRAGMA journal_mode = WAL");
    db.exec("PRAGMA synchronous = NORMAL");
    db.exec("PRAGMA busy_timeout = 2000");
    const version = db.prepare("PRAGMA user_version").get()?.["user_version"];
    if (version !== 0 && version !== SCHEMA_VERSION) {
      throw new TimelineCacheSchemaError(path, `user_version ${String(version)}`);
    }
    const check = db.prepare("PRAGMA quick_check").get()?.["quick_check"];
    if (check !== "ok") {
      throw new TimelineCacheSchemaError(path, `quick_check ${String(check)}`);
    }
    db.exec(SCHEMA_SQL);
    db.exec(`PRAGMA user_version = ${SCHEMA_VERSION}`);
    return db;
  } catch (err) {
    db.close();
    throw err;
  }
}

/**
 * Opens `$PASEO_HOME/timeline-cache.sqlite`. The file is a rebuildable cache: an unusable file is
 * deleted and recreated once. Returns null when node:sqlite is missing or the second open fails.
 */
export async function openTimelineCacheStore(
  paseoHome: string,
  logger: Logger,
): Promise<SqliteTimelineStore | null> {
  const storeLogger = logger.child({ module: "timeline-cache" });
  // Held in a variable so TypeScript skips module resolution: @types/node@20 has no
  // node:sqlite typings yet, while the runtime (Node 22+ / Electron) provides it.
  const sqliteSpecifier: string = "node:sqlite";
  let sqlite: NodeSqliteModule;
  try {
    sqlite = (await import(sqliteSpecifier)) as unknown as NodeSqliteModule;
  } catch (err) {
    storeLogger.warn({ err }, "node:sqlite unavailable; timeline cache disabled");
    return null;
  }

  const path = join(paseoHome, CACHE_FILE_NAME);
  try {
    return new SqliteTimelineStore(openDatabase(sqlite, path), storeLogger);
  } catch (err) {
    storeLogger.warn({ err, path }, "Timeline cache unusable; rebuilding it");
  }
  try {
    removeCacheFiles(path);
    return new SqliteTimelineStore(openDatabase(sqlite, path), storeLogger);
  } catch (err) {
    storeLogger.error({ err, path }, "Timeline cache rebuild failed; timeline cache disabled");
    return null;
  }
}
