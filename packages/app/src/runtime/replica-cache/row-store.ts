export { REPLICA_ROW_STORE_SCHEMA_VERSION, REPLICA_SINGLETON_ROW_ID } from "./row-store-schema";

export type ReplicaRowKind = "agent" | "workspace" | "project" | "timeline" | "checkpoint";

export interface ReplicaRowKey {
  serverId: string;
  kind: ReplicaRowKind;
  id: string;
}

export interface ReplicaRow extends ReplicaRowKey {
  payload: string;
}

export interface ReplicaRowChanges {
  upserts: ReplicaRow[];
  deletes: ReplicaRowKey[];
}

/** Stored payload bytes and the last upsert time of one host. Hosts with no rows are absent. */
export interface ReplicaHostStats {
  serverId: string;
  bytes: number;
  lastWrittenAt: number;
}

export interface ReplicaRowStore {
  open(): Promise<void>;
  read(
    serverId: string,
    kinds: readonly ReplicaRowKind[],
    ids?: readonly string[],
  ): Promise<ReplicaRow[]>;
  /** Keys only; never reads payloads. */
  readKeys(serverId: string, kinds: readonly ReplicaRowKind[]): Promise<ReplicaRowKey[]>;
  /** Per-host totals kept in the same transaction as the row writes. Omit the ids for all hosts. */
  readHostStats(serverIds?: readonly string[]): Promise<ReplicaHostStats[]>;
  apply(changes: ReplicaRowChanges): Promise<void>;
  deleteHost(serverId: string): Promise<void>;
  renameHost(oldServerId: string, newServerId: string): Promise<void>;
  clear(): Promise<void>;
}
