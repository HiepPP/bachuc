# TASK-126 App replica budget from stored sizes

Group: A (writes `packages/app/src/runtime/replica-cache/**`; no shared file with other groups)
Class: code

## Brief

Goal: The app replica cache enforces its 32 MiB budget without a copy of every row in JS memory.

Change: `ensureStoredIndex` reads all rows into `storedRows` for the whole session -> the row store keeps per-host byte totals and write times, and the cache reads only those totals.

Boundaries:

- Today the first persist calls `rowStore.readAll()` and keeps every payload string of every host in `storedRows` ([packages/app/src/runtime/replica-cache/index.ts](packages/app/src/runtime/replica-cache/index.ts) line 1513). That can hold up to 32 MiB for the process lifetime.
- Keep the eviction rule: evict whole hosts, least recently written first, when the total passes the budget.
- Keep `MAX_TIMELINE_ITEMS` at 50 and the 1 s write debounce.
- Keep these guarantees from [docs/data-model.md](docs/data-model.md) "Replica row store": host registry startup and painting a requested row never wait for budget work; a directory change and its checkpoint commit in one transaction; one invalid row is a miss.
- Both backends must give the same totals. Keep the totals in the same transaction as the row writes, so they cannot drift.
- Raise `REPLICA_ROW_STORE_SCHEMA_VERSION`. The old cache is dropped and rebuilt once.

How:

- Add a size and a write time to each stored row, or a per-host stats record kept in the same transaction. On expo-sqlite, use columns and `SUM` / `MAX` queries. On IndexedDB, use a small stats object store or an index that does not load payloads.
- Add a row store method that returns, per host, the byte total and the last write time.
- Replace `storedRows` and the `readAll` call in `ensureStoredIndex` with that method. Compute eviction from the totals and the pending changes.
- Remove `readAll` from the row store interface if nothing else uses it.
- Extend the contract test so both backends prove the totals after upsert, delete, host delete, and schema reset.

Files:

- [packages/app/src/runtime/replica-cache/index.ts](packages/app/src/runtime/replica-cache/index.ts) (budget from totals; no `storedRows`)
- [packages/app/src/runtime/replica-cache/row-store.ts](packages/app/src/runtime/replica-cache/row-store.ts) (interface: per-host totals)
- [packages/app/src/runtime/replica-cache/row-store-sqlite.ts](packages/app/src/runtime/replica-cache/row-store-sqlite.ts) (columns and queries)
- [packages/app/src/runtime/replica-cache/row-store.web.ts](packages/app/src/runtime/replica-cache/row-store.web.ts) (IndexedDB stats)
- [packages/app/src/runtime/replica-cache/row-store.native.ts](packages/app/src/runtime/replica-cache/row-store.native.ts) (wire the new method if needed)
- [packages/app/src/runtime/replica-cache/row-store-schema.ts](packages/app/src/runtime/replica-cache/row-store-schema.ts) (version bump)
- [packages/app/src/runtime/replica-cache/row-store.contract.ts](packages/app/src/runtime/replica-cache/row-store.contract.ts) (totals cases)
- [packages/app/src/runtime/replica-cache/index.test.ts](packages/app/src/runtime/replica-cache/index.test.ts) (budget and eviction cases)

Expected result:

- A persist never reads row payloads to compute the budget.
- Eviction still drops the least recently written host when the total passes the budget.
- After eviction or host delete, the reported totals match the rows that remain.
- A cache written by the old schema is dropped and rebuilt on first open.

Anti-goal: Row payload strings held by `ReplicaCache` after the first persist stay at 0 bytes; tripwire; read from a unit test that counts payload reads through the row store at baseline, after the store change, and at completion.

## Verify

- `npx vitest run packages/app/src/runtime/replica-cache/index.test.ts --bail=1` -> pass, including a test that a persist reads no row payloads for the budget.
- `npx vitest run packages/app/src/runtime/replica-cache/row-store.web.test.ts --bail=1` -> pass, including the totals contract cases.
- `npx vitest run packages/app/src/runtime/replica-cache/row-store.native.test.ts --bail=1` -> pass, including the totals contract cases.
- `rg -n "storedRows" packages/app/src/runtime/replica-cache/index.ts` -> no match.
- `npm run typecheck` -> exit 0.
- Anti-goal: the payload-read count test reports 0 payload reads for budget work -> within limit.
