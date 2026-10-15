import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ReplicaHostStats, ReplicaRow, ReplicaRowKind, ReplicaRowStore } from "./row-store";
import { replicaPayloadBytes } from "./row-store-schema";

export interface ReplicaRowStoreHarness {
  store: ReplicaRowStore;
  openWithSchemaVersion(schemaVersion: number): Promise<ReplicaRowStore>;
}

export type CreateReplicaRowStoreHarness = () => Promise<ReplicaRowStoreHarness>;

const ALL_KINDS: readonly ReplicaRowKind[] = [
  "agent",
  "workspace",
  "project",
  "timeline",
  "checkpoint",
];

function row(serverId: string, kind: ReplicaRow["kind"], id: string, payload: string): ReplicaRow {
  return { serverId, kind, id, payload };
}

function readHost(store: ReplicaRowStore, serverId: string): Promise<ReplicaRow[]> {
  return store.read(serverId, ALL_KINDS);
}

/** Totals of every host, as `serverId -> bytes`. */
async function totals(store: ReplicaRowStore): Promise<Record<string, number>> {
  return Object.fromEntries(
    (await store.readHostStats()).map((host) => [host.serverId, host.bytes]),
  );
}

async function hostsOldestWrittenFirst(store: ReplicaRowStore): Promise<[string, number][]> {
  const stats: ReplicaHostStats[] = await store.readHostStats();
  return stats
    .sort((left, right) => left.lastWrittenAt - right.lastWrittenAt)
    .map((host) => [host.serverId, host.lastWrittenAt]);
}

export function runReplicaRowStoreContract(
  engineName: string,
  createHarness: CreateReplicaRowStoreHarness,
): void {
  describe(`${engineName} replica row store`, () => {
    let harness: ReplicaRowStoreHarness;
    let store: ReplicaRowStore;

    beforeEach(async () => {
      harness = await createHarness();
      store = harness.store;
      await store.open();
    });

    afterEach(() => {
      vi.restoreAllMocks();
    });

    it("reads an empty store", async () => {
      expect(await store.readHostStats()).toEqual([]);
      expect(await readHost(store, "server-a")).toEqual([]);
    });

    it("round-trips opaque payloads grouped by server", async () => {
      await store.apply({
        upserts: [
          row("server-b", "checkpoint", "singleton", '{"cursor":2}'),
          row("server-a", "agent", "agent-1", '{"status":"running"}'),
          row("server-a", "timeline", "singleton", "not parsed by the row store"),
        ],
        deletes: [],
      });

      expect(await readHost(store, "server-a")).toEqual([
        row("server-a", "agent", "agent-1", '{"status":"running"}'),
        row("server-a", "timeline", "singleton", "not parsed by the row store"),
      ]);
      expect(await readHost(store, "server-b")).toEqual([
        row("server-b", "checkpoint", "singleton", '{"cursor":2}'),
      ]);
    });

    it("reads only requested kinds for one host", async () => {
      await store.apply({
        upserts: [
          row("server-a", "agent", "agent-1", "agent"),
          row("server-a", "timeline", "singleton", "timeline"),
          row("server-b", "agent", "agent-2", "other host"),
        ],
        deletes: [],
      });

      expect(await store.read("server-a", ["timeline"])).toEqual([
        row("server-a", "timeline", "singleton", "timeline"),
      ]);
      expect(await store.read("server-a", ["agent"], ["missing"])).toEqual([]);
    });

    it("lists the keys of the requested kinds without payloads", async () => {
      await store.apply({
        upserts: [
          row("server-a", "agent", "agent-1", "agent"),
          row("server-a", "timeline", "agent-1", "timeline"),
          row("server-a", "checkpoint", "singleton", "checkpoint"),
          row("server-b", "agent", "agent-2", "other host"),
        ],
        deletes: [],
      });

      expect(await store.readKeys("server-a", ["agent", "checkpoint"])).toEqual([
        { serverId: "server-a", kind: "agent", id: "agent-1" },
        { serverId: "server-a", kind: "checkpoint", id: "singleton" },
      ]);
      expect(await store.readKeys("server-a", [])).toEqual([]);
    });

    it("overwrites an existing row on upsert", async () => {
      await store.apply({
        upserts: [row("server-a", "agent", "agent-1", "old")],
        deletes: [],
      });
      await store.apply({
        upserts: [row("server-a", "agent", "agent-1", "new")],
        deletes: [],
      });

      expect(await readHost(store, "server-a")).toEqual([
        row("server-a", "agent", "agent-1", "new"),
      ]);
    });

    it("deletes one row by its compound identity", async () => {
      await store.apply({
        upserts: [
          row("server-a", "agent", "same-id", "agent"),
          row("server-a", "workspace", "same-id", "workspace"),
        ],
        deletes: [],
      });
      await store.apply({
        upserts: [],
        deletes: [{ serverId: "server-a", kind: "agent", id: "same-id" }],
      });

      expect(await readHost(store, "server-a")).toEqual([
        row("server-a", "workspace", "same-id", "workspace"),
      ]);
    });

    it("rolls back the entire apply when one write fails", async () => {
      const invalidRow = {
        ...row("server-a", "agent", "invalid", "invalid"),
        serverId: undefined,
      } as unknown as ReplicaRow;

      await expect(
        store.apply({
          upserts: [row("server-a", "agent", "would-have-been-written", "payload"), invalidRow],
          deletes: [],
        }),
      ).rejects.toBeInstanceOf(Error);
      expect(await readHost(store, "server-a")).toEqual([]);
      expect(await store.readHostStats()).toEqual([]);
    });

    it("deletes every row belonging to one host", async () => {
      await store.apply({
        upserts: [
          row("server-a", "agent", "agent-1", "a"),
          row("server-a", "project", "project-1", "p"),
          row("server-b", "agent", "agent-2", "b"),
        ],
        deletes: [],
      });

      await store.deleteHost("server-a");

      expect(await readHost(store, "server-a")).toEqual([]);
      expect(await readHost(store, "server-b")).toEqual([row("server-b", "agent", "agent-2", "b")]);
      expect(await totals(store)).toEqual({ "server-b": 1 });
    });

    it("renames a host and lets moved rows replace target collisions", async () => {
      await store.apply({
        upserts: [
          row("temporary", "agent", "agent-1", "temporary-agent"),
          row("temporary", "project", "project-1", "temporary-project"),
          row("canonical", "agent", "agent-1", "canonical-agent"),
          row("canonical", "workspace", "workspace-1", "canonical-workspace"),
        ],
        deletes: [],
      });

      await store.renameHost("temporary", "canonical");

      expect(await readHost(store, "temporary")).toEqual([]);
      expect(await readHost(store, "canonical")).toEqual([
        row("canonical", "agent", "agent-1", "temporary-agent"),
        row("canonical", "project", "project-1", "temporary-project"),
        row("canonical", "workspace", "workspace-1", "canonical-workspace"),
      ]);
      expect(await totals(store)).toEqual({
        canonical: ["temporary-agent", "temporary-project", "canonical-workspace"].reduce(
          (sum, payload) => sum + replicaPayloadBytes(payload),
          0,
        ),
      });
    });

    it("clears all cached rows", async () => {
      await store.apply({
        upserts: [
          row("server-a", "agent", "agent-1", "a"),
          row("server-b", "agent", "agent-2", "b"),
        ],
        deletes: [],
      });

      await store.clear();

      expect(await readHost(store, "server-a")).toEqual([]);
      expect(await readHost(store, "server-b")).toEqual([]);
      expect(await store.readHostStats()).toEqual([]);
    });

    it("wipes cached rows when the schema version changes on open", async () => {
      await store.apply({
        upserts: [row("server-a", "agent", "agent-1", "stale")],
        deletes: [],
      });

      const upgradedStore = await harness.openWithSchemaVersion(2);

      expect(await readHost(upgradedStore, "server-a")).toEqual([]);
      expect(await upgradedStore.readHostStats()).toEqual([]);
    });

    describe("host totals", () => {
      it("sums the UTF-8 bytes of each host's rows after upserts and overwrites", async () => {
        const multiByte = "é😀"; // 2 + 4 bytes
        await store.apply({
          upserts: [
            row("server-a", "agent", "agent-1", "abcd"),
            row("server-a", "timeline", "agent-1", multiByte),
            row("server-b", "agent", "agent-2", "xy"),
          ],
          deletes: [],
        });

        expect(await totals(store)).toEqual({ "server-a": 4 + 6, "server-b": 2 });

        await store.apply({ upserts: [row("server-a", "agent", "agent-1", "a")], deletes: [] });

        expect(await totals(store)).toEqual({ "server-a": 1 + 6, "server-b": 2 });
      });

      it("reports only the requested hosts", async () => {
        await store.apply({
          upserts: [
            row("server-a", "agent", "agent-1", "aaa"),
            row("server-b", "agent", "agent-2", "bb"),
          ],
          deletes: [],
        });

        expect(await store.readHostStats(["server-b", "missing"])).toEqual([
          { serverId: "server-b", bytes: 2, lastWrittenAt: expect.any(Number) },
        ]);
        expect(await store.readHostStats([])).toEqual([]);
      });

      it("lowers the total on delete and drops a host whose last row is gone", async () => {
        await store.apply({
          upserts: [
            row("server-a", "agent", "agent-1", "aaa"),
            row("server-a", "workspace", "workspace-1", "bb"),
          ],
          deletes: [],
        });

        await store.apply({
          upserts: [],
          deletes: [
            { serverId: "server-a", kind: "agent", id: "agent-1" },
            { serverId: "server-a", kind: "agent", id: "never-stored" },
          ],
        });
        expect(await totals(store)).toEqual({ "server-a": 2 });

        await store.apply({
          upserts: [],
          deletes: [{ serverId: "server-a", kind: "workspace", id: "workspace-1" }],
        });
        expect(await store.readHostStats()).toEqual([]);
      });

      it("applies a delete and an upsert of the same row as one total", async () => {
        await store.apply({ upserts: [row("server-a", "agent", "agent-1", "old!")], deletes: [] });

        await store.apply({
          upserts: [row("server-a", "agent", "agent-1", "new")],
          deletes: [{ serverId: "server-a", kind: "agent", id: "agent-1" }],
        });

        expect(await totals(store)).toEqual({ "server-a": 3 });
      });

      it("orders hosts by the time of their last upsert", async () => {
        const now = vi.spyOn(Date, "now");
        now.mockReturnValue(1_000);
        await store.apply({ upserts: [row("server-a", "agent", "agent-1", "a")], deletes: [] });
        now.mockReturnValue(2_000);
        await store.apply({ upserts: [row("server-b", "agent", "agent-2", "b")], deletes: [] });
        now.mockReturnValue(3_000);
        await store.apply({ upserts: [row("server-a", "agent", "agent-3", "c")], deletes: [] });

        expect(await hostsOldestWrittenFirst(store)).toEqual([
          ["server-b", 2_000],
          ["server-a", 3_000],
        ]);
      });
    });
  });
}
