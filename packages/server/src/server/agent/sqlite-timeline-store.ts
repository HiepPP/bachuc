import { rmSync } from "node:fs";
import { join } from "node:path";
import type { Logger } from "pino";
import type { AgentTimelineItem } from "./agent-sdk-types.js";
import type {
  AgentTimelineRow,
  DurableTimelineMeta,
  DurableTimelineSnapshot,
  DurableTimelineStore,
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

const SCHEMA_VERSION = 1;
const FLUSH_DELAY_MS = 100;
const FLUSH_ROW_THRESHOLD = 256;
const ROWS_PER_TRANSACTION = 500;
const LOAD_PAGE_ROWS = 2_000;
const CACHE_FILE_NAME = "timeline-cache.sqlite";

const SCHEMA_SQL = `
CREATE TABLE IF NOT EXISTS timelines (
  agent_id           TEXT PRIMARY KEY,
  epoch              TEXT NOT NULL,
  next_seq           INTEGER NOT NULL,
  complete           INTEGER NOT NULL DEFAULT 0,
  provider_subagents TEXT,
  updated_at         TEXT NOT NULL
) WITHOUT ROWID;

CREATE TABLE IF NOT EXISTS timeline_rows (
  agent_id            TEXT NOT NULL,
  seq                 INTEGER NOT NULL,
  timestamp           TEXT NOT NULL,
  turn_id             TEXT,
  provider_message_id TEXT,
  item                TEXT NOT NULL,
  PRIMARY KEY (agent_id, seq)
) WITHOUT ROWID;
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
}

function prepareStatements(db: SqliteDatabase): Statements {
  return {
    getMeta: db.prepare(
      "SELECT epoch, next_seq, complete, provider_subagents FROM timelines WHERE agent_id = ?",
    ),
    upsertRow: db.prepare(
      `INSERT OR REPLACE INTO timeline_rows
         (agent_id, seq, timestamp, turn_id, provider_message_id, item)
       VALUES (?, ?, ?, ?, ?, ?)`,
    ),
    advanceNextSeq: db.prepare(
      "UPDATE timelines SET next_seq = MAX(next_seq, ?), updated_at = ? WHERE agent_id = ?",
    ),
    setProviderSubagents: db.prepare(
      "UPDATE timelines SET provider_subagents = ?, updated_at = ? WHERE agent_id = ?",
    ),
    setComplete: db.prepare("UPDATE timelines SET complete = ?, updated_at = ? WHERE agent_id = ?"),
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

export class SqliteTimelineStore implements DurableTimelineStore {
  private readonly statements: Statements;
  private readonly buffers = new Map<string, AgentBuffer>();
  /** Agents whose rows were lost by a failed flush; never served until the next reset. */
  private readonly untrusted = new Set<string>();
  private bufferedRowCount = 0;
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
    if (!row) return null;
    return {
      epoch: requireString(row, "epoch"),
      nextSeq: requireNumber(row, "next_seq"),
      complete: requireNumber(row, "complete") === 1 && !this.untrusted.has(agentId),
    };
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
      epoch: requireString(metaRow, "epoch"),
      nextSeq: requireNumber(metaRow, "next_seq"),
      complete: requireNumber(metaRow, "complete") === 1 && !this.untrusted.has(agentId),
      rows,
      providerSubagents: parseProviderSubagents(optionalString(metaRow, "provider_subagents")),
    };
  }

  reset(agentId: string, epoch: string): void {
    this.takeBuffer(agentId);
    this.transaction(() => {
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

  setComplete(agentId: string, complete: boolean): void {
    const buffer = this.takeBuffer(agentId);
    const ok = this.writeBuffers(buffer ? new Map([[agentId, buffer]]) : new Map(), () => {
      const trusted = complete && !this.untrusted.has(agentId);
      this.statements.setComplete.run(trusted ? 1 : 0, new Date().toISOString(), agentId);
    });
    if (!ok) this.writeIncomplete([agentId]);
  }

  deleteAgent(agentId: string): void {
    this.takeBuffer(agentId);
    this.transaction(() => {
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
              JSON.stringify(row.item),
            );
            maxSeq = Math.max(maxSeq, row.seq);
          }
          if (maxSeq > 0) this.statements.advanceNextSeq.run(maxSeq + 1, now, agentId);
          if (buffer.providerSubagents !== null) {
            const json = JSON.stringify(buffer.providerSubagents);
            this.statements.setProviderSubagents.run(json, now, agentId);
          }
        }
        after?.();
      });
      return true;
    } catch (err) {
      this.logger.error({ err, agentIds: [...buffers.keys()] }, "Timeline cache write failed");
      return false;
    }
  }

  private writeIncomplete(agentIds: readonly string[]): void {
    const now = new Date().toISOString();
    for (const agentId of agentIds) {
      this.untrusted.add(agentId);
      try {
        this.statements.setComplete.run(0, now, agentId);
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
