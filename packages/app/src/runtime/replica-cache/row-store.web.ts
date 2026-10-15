import type {
  ReplicaHostStats,
  ReplicaRow,
  ReplicaRowChanges,
  ReplicaRowKey,
  ReplicaRowStore,
} from "./row-store";
import { REPLICA_ROW_STORE_SCHEMA_VERSION, replicaPayloadBytes } from "./row-store-schema";

export type {
  ReplicaHostStats,
  ReplicaRow,
  ReplicaRowChanges,
  ReplicaRowKey,
  ReplicaRowKind,
  ReplicaRowStore,
} from "./row-store";
export { REPLICA_ROW_STORE_SCHEMA_VERSION, REPLICA_SINGLETON_ROW_ID } from "./row-store-schema";

const DATABASE_NAME = "paseo-replica-row-store";
const DATABASE_VERSION = 2;
const ROWS_STORE = "rows";
const META_STORE = "meta";
const STATS_STORE = "host_stats";
const ROWS_AND_STATS = [ROWS_STORE, STATS_STORE];

// Per-host totals kept in the same transaction as the row writes, so reading them never loads a
// payload. `rows` lets an emptied host drop its record.
interface StoredHostStats extends ReplicaHostStats {
  rows: number;
}
const SCHEMA_VERSION_KEY = "schema_version";

interface IndexedDbReplicaRowStoreOptions {
  databaseName: string;
  schemaVersion: number;
}

function requestResult<Result>(request: IDBRequest<Result>): Promise<Result> {
  return new Promise((resolve, reject) => {
    request.addEventListener("success", () => resolve(request.result));
    request.addEventListener("error", () =>
      reject(request.error ?? new Error("Replica row store request failed")),
    );
  });
}

function transactionComplete(transaction: IDBTransaction): Promise<void> {
  return new Promise((resolve, reject) => {
    transaction.addEventListener("complete", () => resolve());
    transaction.addEventListener("abort", () =>
      reject(transaction.error ?? new Error("Replica row store transaction was aborted")),
    );
    transaction.addEventListener("error", () =>
      reject(transaction.error ?? new Error("Replica row store transaction failed")),
    );
  });
}

async function runTransaction(
  database: IDBDatabase,
  storeNames: string | string[],
  operation: (transaction: IDBTransaction) => Promise<void>,
): Promise<void> {
  const transaction = database.transaction(storeNames, "readwrite");
  const completion = transactionComplete(transaction);
  try {
    await operation(transaction);
    await completion;
  } catch (error) {
    try {
      transaction.abort();
    } catch {
      // The transaction already aborted after the failed request.
    }
    await completion.catch(() => undefined);
    throw error;
  }
}

async function openDatabase(databaseName: string): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(databaseName, DATABASE_VERSION);
    request.addEventListener("upgradeneeded", () => {
      if (!request.result.objectStoreNames.contains(ROWS_STORE)) {
        request.result.createObjectStore(ROWS_STORE, { keyPath: ["serverId", "kind", "id"] });
      }
      if (!request.result.objectStoreNames.contains(META_STORE)) {
        request.result.createObjectStore(META_STORE);
      }
      if (!request.result.objectStoreNames.contains(STATS_STORE)) {
        request.result.createObjectStore(STATS_STORE, { keyPath: "serverId" });
      }
    });
    request.addEventListener("success", () => resolve(request.result));
    request.addEventListener("error", () =>
      reject(request.error ?? new Error("Failed to open replica row store")),
    );
    request.addEventListener("blocked", () =>
      reject(new Error("Replica row store database open was blocked")),
    );
  });
}

/** @package Test seam for exercising the browser engine with an isolated database. */
export function createIndexedDbReplicaRowStore(
  options: IndexedDbReplicaRowStoreOptions,
): ReplicaRowStore {
  let database: IDBDatabase | null = null;
  let opening: Promise<void> | null = null;

  function getDatabase(): IDBDatabase {
    if (!database) throw new Error("Replica row store has not been opened");
    return database;
  }

  async function open(): Promise<void> {
    opening ??= (async () => {
      const openedDatabase = await openDatabase(options.databaseName);
      await runTransaction(openedDatabase, [...ROWS_AND_STATS, META_STORE], async (transaction) => {
        const storedVersion = await requestResult(
          transaction.objectStore(META_STORE).get(SCHEMA_VERSION_KEY),
        );
        if (storedVersion !== options.schemaVersion) {
          await requestResult(transaction.objectStore(ROWS_STORE).clear());
          await requestResult(transaction.objectStore(STATS_STORE).clear());
          await requestResult(
            transaction.objectStore(META_STORE).put(options.schemaVersion, SCHEMA_VERSION_KEY),
          );
        }
      });
      database = openedDatabase;
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
    const transaction = getDatabase().transaction(ROWS_STORE, "readonly");
    const completion = transactionComplete(transaction);
    const store = transaction.objectStore(ROWS_STORE);
    const keys = await Promise.all(
      kinds.map((kind) =>
        requestResult<IDBValidKey[]>(
          store.getAllKeys(IDBKeyRange.bound([serverId, kind], [serverId, kind, []])),
        ),
      ),
    );
    await completion;
    return keys
      .flat()
      .map((key) => {
        const [keyServerId, keyKind, keyId] = key as [string, ReplicaRow["kind"], string];
        return { serverId: keyServerId, kind: keyKind, id: keyId };
      })
      .sort((left, right) =>
        left.kind === right.kind
          ? left.id.localeCompare(right.id)
          : left.kind.localeCompare(right.kind),
      );
  }

  async function readHostStats(serverIds?: readonly string[]): Promise<ReplicaHostStats[]> {
    if (serverIds?.length === 0) return [];
    const transaction = getDatabase().transaction(STATS_STORE, "readonly");
    const completion = transactionComplete(transaction);
    const store = transaction.objectStore(STATS_STORE);
    const stored = serverIds
      ? (
          await Promise.all(
            serverIds.map((serverId) =>
              requestResult<StoredHostStats | undefined>(store.get(serverId)),
            ),
          )
        ).filter((stats): stats is StoredHostStats => stats !== undefined)
      : await requestResult<StoredHostStats[]>(store.getAll());
    await completion;
    return stored
      .map(({ serverId, bytes, lastWrittenAt }) => ({ serverId, bytes, lastWrittenAt }))
      .sort((left, right) => (left.serverId < right.serverId ? -1 : 1));
  }

  /** Loads and saves host stats inside the caller's transaction. */
  function createStatsTracker(transaction: IDBTransaction) {
    const store = transaction.objectStore(STATS_STORE);
    const hosts = new Map<string, StoredHostStats>();
    async function load(serverId: string): Promise<StoredHostStats> {
      let stats = hosts.get(serverId);
      if (!stats) {
        stats = (await requestResult<StoredHostStats | undefined>(store.get(serverId))) ?? {
          serverId,
          bytes: 0,
          rows: 0,
          lastWrittenAt: 0,
        };
        hosts.set(serverId, stats);
      }
      return stats;
    }
    async function save(): Promise<void> {
      for (const stats of hosts.values()) {
        if (stats.rows <= 0) await requestResult(store.delete(stats.serverId));
        else await requestResult(store.put(stats));
      }
    }
    return { load, save };
  }

  async function read(
    serverId: string,
    kinds: readonly ReplicaRow["kind"][],
    ids?: readonly string[],
  ): Promise<ReplicaRow[]> {
    if (kinds.length === 0 || ids?.length === 0) return [];
    const transaction = getDatabase().transaction(ROWS_STORE, "readonly");
    const completion = transactionComplete(transaction);
    const store = transaction.objectStore(ROWS_STORE);
    const requests = ids
      ? kinds.flatMap((kind) =>
          ids.map(async (id) => {
            const row = await requestResult<ReplicaRow | undefined>(
              store.get([serverId, kind, id]),
            );
            return row ? [row] : [];
          }),
        )
      : kinds.map((kind) =>
          requestResult<ReplicaRow[]>(
            store.getAll(IDBKeyRange.bound([serverId, kind], [serverId, kind, []])),
          ),
        );
    const rows = (await Promise.all(requests)).flat();
    await completion;
    return rows.sort((left, right) =>
      left.kind === right.kind
        ? left.id.localeCompare(right.id)
        : left.kind.localeCompare(right.kind),
    );
  }

  async function apply(changes: ReplicaRowChanges): Promise<void> {
    await runTransaction(getDatabase(), ROWS_AND_STATS, async (transaction) => {
      const rows = transaction.objectStore(ROWS_STORE);
      const stats = createStatsTracker(transaction);
      const writtenAt = Date.now();
      // The previous row is read inside the store only to learn its size; it is never retained.
      for (const key of changes.deletes) {
        const previous = await requestResult<ReplicaRow | undefined>(
          rows.get([key.serverId, key.kind, key.id]),
        );
        if (!previous) continue;
        const host = await stats.load(key.serverId);
        host.bytes -= replicaPayloadBytes(previous.payload);
        host.rows -= 1;
        await requestResult(rows.delete([key.serverId, key.kind, key.id]));
      }
      for (const row of changes.upserts) {
        const previous = await requestResult<ReplicaRow | undefined>(
          rows.get([row.serverId, row.kind, row.id]),
        );
        const host = await stats.load(row.serverId);
        host.bytes +=
          replicaPayloadBytes(row.payload) - replicaPayloadBytes(previous?.payload ?? "");
        if (!previous) host.rows += 1;
        host.lastWrittenAt = writtenAt;
        await requestResult(rows.put(row));
      }
      await stats.save();
    });
  }

  async function deleteHost(serverId: string): Promise<void> {
    await runTransaction(getDatabase(), ROWS_AND_STATS, async (transaction) => {
      const rows = transaction.objectStore(ROWS_STORE);
      const range = IDBKeyRange.bound([serverId], [serverId, []]);
      await requestResult(rows.delete(range));
      await requestResult(transaction.objectStore(STATS_STORE).delete(serverId));
    });
  }

  async function renameHost(oldServerId: string, newServerId: string): Promise<void> {
    if (oldServerId === newServerId) return;
    await runTransaction(getDatabase(), ROWS_AND_STATS, async (transaction) => {
      const rows = transaction.objectStore(ROWS_STORE);
      const stats = createStatsTracker(transaction);
      const range = IDBKeyRange.bound([oldServerId], [oldServerId, []]);
      const oldRows = await requestResult<ReplicaRow[]>(rows.getAll(range));
      const oldStats = await stats.load(oldServerId);
      const newStats = await stats.load(newServerId);
      for (const row of oldRows) {
        const collision = await requestResult<ReplicaRow | undefined>(
          rows.get([newServerId, row.kind, row.id]),
        );
        newStats.bytes +=
          replicaPayloadBytes(row.payload) - replicaPayloadBytes(collision?.payload ?? "");
        if (!collision) newStats.rows += 1;
        await requestResult(rows.put({ ...row, serverId: newServerId }));
        await requestResult(rows.delete([row.serverId, row.kind, row.id]));
      }
      if (oldRows.length > 0) {
        newStats.lastWrittenAt = Math.max(newStats.lastWrittenAt, oldStats.lastWrittenAt);
      }
      oldStats.bytes = 0;
      oldStats.rows = 0;
      await stats.save();
    });
  }

  async function clear(): Promise<void> {
    await runTransaction(getDatabase(), ROWS_AND_STATS, async (transaction) => {
      await requestResult(transaction.objectStore(ROWS_STORE).clear());
      await requestResult(transaction.objectStore(STATS_STORE).clear());
    });
  }

  return { open, read, readKeys, readHostStats, apply, deleteHost, renameHost, clear };
}

export function createReplicaRowStore(): ReplicaRowStore {
  return createIndexedDbReplicaRowStore({
    databaseName: DATABASE_NAME,
    schemaVersion: REPLICA_ROW_STORE_SCHEMA_VERSION,
  });
}
