import type {
  ReplicaHostStats,
  ReplicaRow,
  ReplicaRowChanges,
  ReplicaRowKey,
  ReplicaRowStore,
} from "./row-store";
import { replicaPayloadBytes } from "./row-store-schema";

export type SqliteValue = string | number | null;

export interface ReplicaSqliteConnection {
  exec(sql: string): Promise<void>;
  run(sql: string, params?: readonly SqliteValue[]): Promise<void>;
  all<Row>(sql: string, params?: readonly SqliteValue[]): Promise<Row[]>;
  transaction(operation: (connection: ReplicaSqliteConnection) => Promise<void>): Promise<void>;
}

export interface ReplicaSqliteDriver {
  open(): Promise<ReplicaSqliteConnection>;
}

const CREATE_ROWS_SQL = `
  CREATE TABLE IF NOT EXISTS rows (
    server_id TEXT NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('agent', 'workspace', 'project', 'timeline', 'checkpoint')),
    id TEXT NOT NULL,
    payload TEXT NOT NULL,
    bytes INTEGER NOT NULL,
    written_at INTEGER NOT NULL,
    PRIMARY KEY (server_id, kind, id)
  )
`;

const CREATE_META_SQL = `
  CREATE TABLE IF NOT EXISTS meta (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL
  )
`;

interface StoredMeta {
  value: string;
}

interface StoredHostStats {
  server_id: string;
  bytes: number;
  last_written_at: number;
}

interface StoredRow {
  server_id: string;
  kind: ReplicaRow["kind"];
  id: string;
  payload: string;
}

export function createSqliteReplicaRowStore(
  driver: ReplicaSqliteDriver,
  schemaVersion: number,
): ReplicaRowStore {
  let connection: ReplicaSqliteConnection | null = null;
  let opening: Promise<void> | null = null;

  function getConnection(): ReplicaSqliteConnection {
    if (!connection) {
      throw new Error("Replica row store has not been opened");
    }
    return connection;
  }

  async function open(): Promise<void> {
    opening ??= (async () => {
      const openedConnection = await driver.open();
      await openedConnection.exec(CREATE_META_SQL);
      const storedMeta = await openedConnection.all<StoredMeta>(
        "SELECT value FROM meta WHERE key = ?",
        ["schema_version"],
      );

      if (storedMeta[0]?.value !== String(schemaVersion)) {
        await openedConnection.transaction(async (transaction) => {
          await transaction.exec("DROP TABLE IF EXISTS rows");
          await transaction.exec(CREATE_ROWS_SQL);
          await transaction.run("DELETE FROM meta");
          await transaction.run("INSERT INTO meta (key, value) VALUES (?, ?)", [
            "schema_version",
            String(schemaVersion),
          ]);
        });
      } else {
        await openedConnection.exec(CREATE_ROWS_SQL);
      }

      connection = openedConnection;
    })();

    try {
      await opening;
    } catch (error) {
      opening = null;
      throw error;
    }
  }

  async function readKeys(
    serverId: string,
    kinds: readonly ReplicaRow["kind"][],
  ): Promise<ReplicaRowKey[]> {
    if (kinds.length === 0) return [];
    const storedKeys = await getConnection().all<Omit<StoredRow, "payload">>(
      `SELECT server_id, kind, id FROM rows
       WHERE server_id = ? AND kind IN (${kinds.map(() => "?").join(", ")})
       ORDER BY kind, id`,
      [serverId, ...kinds],
    );
    return storedKeys.map((key) => ({ serverId: key.server_id, kind: key.kind, id: key.id }));
  }

  async function readHostStats(serverIds?: readonly string[]): Promise<ReplicaHostStats[]> {
    if (serverIds?.length === 0) return [];
    const hostClause = serverIds
      ? ` WHERE server_id IN (${serverIds.map(() => "?").join(", ")})`
      : "";
    const stats = await getConnection().all<StoredHostStats>(
      `SELECT server_id, SUM(bytes) AS bytes, MAX(written_at) AS last_written_at
       FROM rows${hostClause} GROUP BY server_id ORDER BY server_id`,
      serverIds ? [...serverIds] : [],
    );
    return stats.map((host) => ({
      serverId: host.server_id,
      bytes: host.bytes,
      lastWrittenAt: host.last_written_at,
    }));
  }

  async function read(
    serverId: string,
    kinds: readonly ReplicaRow["kind"][],
    ids?: readonly string[],
  ): Promise<ReplicaRow[]> {
    if (kinds.length === 0 || ids?.length === 0) return [];
    const placeholders = kinds.map(() => "?").join(", ");
    const idClause = ids ? ` AND id IN (${ids.map(() => "?").join(", ")})` : "";
    const storedRows = await getConnection().all<StoredRow>(
      `SELECT server_id, kind, id, payload FROM rows
       WHERE server_id = ? AND kind IN (${placeholders})${idClause}
       ORDER BY kind, id`,
      [serverId, ...kinds, ...(ids ?? [])],
    );
    return storedRows.map((row) => ({
      serverId: row.server_id,
      kind: row.kind,
      id: row.id,
      payload: row.payload,
    }));
  }

  async function apply(changes: ReplicaRowChanges): Promise<void> {
    await getConnection().transaction(async (transaction) => {
      for (const key of changes.deletes) {
        await transaction.run("DELETE FROM rows WHERE server_id = ? AND kind = ? AND id = ?", [
          key.serverId,
          key.kind,
          key.id,
        ]);
      }
      const writtenAt = Date.now();
      for (const row of changes.upserts) {
        await transaction.run(
          `INSERT INTO rows (server_id, kind, id, payload, bytes, written_at)
           VALUES (?, ?, ?, ?, ?, ?)
           ON CONFLICT (server_id, kind, id) DO UPDATE SET
             payload = excluded.payload, bytes = excluded.bytes, written_at = excluded.written_at`,
          [
            row.serverId,
            row.kind,
            row.id,
            row.payload,
            replicaPayloadBytes(row.payload),
            writtenAt,
          ],
        );
      }
    });
  }

  async function deleteHost(serverId: string): Promise<void> {
    await getConnection().run("DELETE FROM rows WHERE server_id = ?", [serverId]);
  }

  async function renameHost(oldServerId: string, newServerId: string): Promise<void> {
    if (oldServerId === newServerId) return;
    await getConnection().transaction(async (transaction) => {
      await transaction.run(
        `INSERT INTO rows (server_id, kind, id, payload, bytes, written_at)
         SELECT ?, kind, id, payload, bytes, written_at FROM rows WHERE server_id = ?
         ON CONFLICT (server_id, kind, id) DO UPDATE SET
           payload = excluded.payload, bytes = excluded.bytes, written_at = excluded.written_at`,
        [newServerId, oldServerId],
      );
      await transaction.run("DELETE FROM rows WHERE server_id = ?", [oldServerId]);
    });
  }

  async function clear(): Promise<void> {
    await getConnection().run("DELETE FROM rows");
  }

  return { open, read, readKeys, readHostStats, apply, deleteHost, renameHost, clear };
}
