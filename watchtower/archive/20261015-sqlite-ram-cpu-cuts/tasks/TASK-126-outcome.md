# TASK-126 Outcome

## Status

DONE

## Changed

- `packages/app/src/runtime/replica-cache/index.ts`: removed `storedRows`, `rowBytes`, `readAll` use. `ReplicaCache` keeps only per-host byte totals and write order. `ensureStoredIndex` reads `readHostStats()`. After each `apply`, `refreshHostStats` re-reads the touched hosts' totals, then `fitBudget` evicts whole hosts, least recently written first. A baseline lists keys with `readKeys`. `reconcileServerId` refreshes totals after `renameHost`.
- `row-store.ts`: removed `readAll` and `ReplicaHostRows`. Added `ReplicaHostStats`, `readKeys`, `readHostStats`.
- `row-store-schema.ts`: `REPLICA_ROW_STORE_SCHEMA_VERSION` 1 -> 2. Added `replicaPayloadBytes` here, not in `row-store.ts`, because the Vitest resolver maps `./row-store` to `row-store.native.ts`.
- `row-store-sqlite.ts`: `rows` gets `bytes` and `written_at` columns. Totals come from `SUM(bytes)` and `MAX(written_at)`.
- `row-store.web.ts`: `DATABASE_VERSION` 1 -> 2 and a `host_stats` object store, updated in the same transaction as row writes, delete host, rename, clear, and schema reset.
- `row-store.native.ts`: re-exports `ReplicaHostStats` instead of `ReplicaHostRows`.
- `row-store.contract.ts`, `index.test.ts`: totals, keys, write-order, payload-read, and eviction cases.
- Outside the Files list (required dep fix): `packages/app/src/runtime/host-runtime.test.ts`, the in-memory fake store replaced `readAll` with `readKeys` and `readHostStats`.

## Contract

- `ReplicaRowStore.readHostStats(serverIds?)` returns `{ serverId, bytes, lastWrittenAt }` per host that has rows. `bytes` is the UTF-8 payload total from `replicaPayloadBytes`. `lastWrittenAt` is the last upsert time (delete-only applies do not move it).
- `ReplicaRowStore.readKeys(serverId, kinds)` returns keys without payloads.
- `readAll` is gone. Totals never drift: SQLite computes them from the same rows; IndexedDB updates `host_stats` in the `apply` transaction.
- The cache now writes a batch first, then evicts. A touched host that alone exceeds the budget is written, then deleted (the old code dropped its changes before the write). End state is the same.
- Old caches (schema 1) are dropped on open on both backends.

## Verified

- `npx vitest run packages/app/src/runtime/replica-cache/index.test.ts --bail=1` -> PASS (32)
- `npx vitest run packages/app/src/runtime/replica-cache/row-store.web.test.ts --bail=1` -> PASS (17)
- `npx vitest run packages/app/src/runtime/replica-cache/row-store.native.test.ts --bail=1` -> PASS (16)
- `npx vitest run src/runtime/host-runtime.test.ts --bail=1` (run in `packages/app`; from the root it fails with `__DEV__ is not defined`) -> PASS (82)
- `npx vitest run src/runtime/directory-sync/index.test.ts --bail=1` (in `packages/app`) -> PASS (29)
- `npx vitest run packages/app/src/timeline/replica.test.ts --bail=1` -> PASS (16)
- `rg -n "storedRows" packages/app/src/runtime/replica-cache/index.ts` -> no match
- `npm run typecheck` -> exit 0
- `npm run lint -- <9 changed files>` -> 0 warnings, 0 errors (run through `/opt/homebrew/bin/npm`; the `npm` wrapper on `PATH` rewrote `npm run lint` into an ESLint call that fails)
- `npm run format:check:files -- <9 changed files>` -> all formatted

## Anti-goal

PASS. Payload strings held by `ReplicaCache` after the first persist: 0 bytes.

| Checkpoint         | Value                                                                               | Source                                                                                                                    | Time             |
| ------------------ | ----------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------- | ---------------- |
| baseline           | every payload of every row (`readAll` in `ensureStoredIndex`, kept in `storedRows`) | source read, `index.ts` line 1513 at HEAD; not re-measured, because stash and checkout are forbidden                      | 2026-10-15       |
| after store change | 0 payloads read                                                                     | `index.test.ts` "reads no row payloads to compute the budget" (`payloadsRead` counter on the fake store; `statReads` > 0) | 2026-10-15 11:10 |
| completion         | 0 payloads read                                                                     | same test, final run                                                                                                      | 2026-10-15 11:10 |

## Lessons

- Import runtime values from `row-store-schema.ts`, not `./row-store`. Vitest resolves `./row-store` to `row-store.native.ts`, so a value exported only from `row-store.ts` is undefined at runtime. Types are fine.
- `packages/app/src/runtime/host-runtime.test.ts` has its own fake `ReplicaRowStore`. Change the interface and that fake together.
- `npm run lint` via the `PATH` `npm` wrapper breaks; use `/opt/homebrew/bin/npm run lint -- <files>`.
- `docs/data-model.md` "Replica row store" still says budget bookkeeping "may scan opaque row sizes during a deferred write". Now it reads stored totals. Not edited: outside this TASK's Files.
- Stale impact: GitNexus reports CRITICAL for `ReplicaCache` and `ReplicaRowStore` (318 and 242 transitive callers). Public `ReplicaCache` API is unchanged; only the `ReplicaRowStore` implementers (3 stores plus test fakes) changed.
