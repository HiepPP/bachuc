# Plan Context

## Shared Context

- Goal: cut RAM and CPU of the daemon and the app with more SQLite, without slower thread open or search.
- The daemon cache `$PASEO_HOME/timeline-cache.sqlite` is rebuildable. Change its schema by raising `SCHEMA_VERSION` in [packages/server/src/server/agent/sqlite-timeline-store.ts](packages/server/src/server/agent/sqlite-timeline-store.ts). A wrong version rebuilds the file. Write no migration code.
- The app replica cache is rebuildable too. Change its schema by raising `REPLICA_ROW_STORE_SCHEMA_VERSION` in [packages/app/src/runtime/replica-cache/row-store-schema.ts](packages/app/src/runtime/replica-cache/row-store-schema.ts). It runs on IndexedDB (browser and Electron) and expo-sqlite (native). Both must pass [packages/app/src/runtime/replica-cache/row-store.contract.ts](packages/app/src/runtime/replica-cache/row-store.contract.ts).
- `DatabaseSync` from `node:sqlite` blocks the daemon event loop. Keep every SQL read paged and small.
- Daemon stall metric: `eventLoopDelay.maxMs` in the `ws_runtime_metrics` log line, written every 30 s ([packages/server/src/server/websocket-server.ts](packages/server/src/server/websocket-server.ts) line 877). Live log: `.dev/paseo-home/daemon.log`. Limit: 100 ms, from the ADR-0032 `## Revisit If`.
- Test on live only. Never restart or stop the release daemon on port 6767. A server change needs a restart of `npm run dev`.
- Run one test file at a time: `npx vitest run <file> --bail=1`. Never run a full suite.
- No protocol change is planned. If a TASK needs one, follow [docs/protocol-compatibility.md](docs/protocol-compatibility.md).

## Decisions

- [ADR-0032](watchtower/decisions/ADR-0032-sqlite-served-old-pages-over-memory-only-serving.md): memory serves a bounded tail; old pages, prompt list, and search candidates come from SQL. It supersedes ADR-0031.
- The app keeps the streams of the 8 most recently viewed threads per host. The owner chose 8 over 3 and over a byte budget on 2026-10-15.
- New TASK IDs start at TASK-126. The archive ID migration was not applied, by owner choice on 2026-10-15.

## Open Decisions

- None.

## References

- [docs/timeline-sync.md](docs/timeline-sync.md)
- [docs/data-model.md](docs/data-model.md)
- [watchtower/decisions/ADR-0032-sqlite-served-old-pages-over-memory-only-serving.md](watchtower/decisions/ADR-0032-sqlite-served-old-pages-over-memory-only-serving.md)
