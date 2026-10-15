# ADR-0031 node:sqlite timeline cache over JSON files and better-sqlite3

Status: accepted
Date: 2026-10-15
Supersedes: -
Scope: `packages/server/src/server/agent/sqlite-timeline-store.ts`, `packages/server/src/server/agent/agent-timeline-store-types.ts`, `packages/server/src/server/agent/agent-manager.ts`, `packages/server/src/server/agent/agent-loading.ts`, `packages/server/src/server/bootstrap.ts`, `package.json`, `packages/*/package.json`
Guard: forbid-text (better-sqlite3|drizzle-orm) in package.json packages/server/package.json packages/desktop/package.json packages/cli/package.json

## Context

- Timelines lived only in daemon memory (`InMemoryAgentTimelineStore`). After a runtime close or a daemon restart, opening a thread ran a full interactive resume through `ensureAgentLoaded`, and each daemon restart minted a new epoch, so the app replica cache lost its cursor.
- The release `daemon.log` on 2026-10-15 had 855 `fetch_agent_timeline_request` calls over 500 ms: p50 10.3 s, p90 145 s. A live agent answered in 0-1 ms. Steady state held about 2 agents in memory, because idle-runtime-closer closes runtimes after 30 minutes.
- On reconnect, about 160 agents resumed within seconds, 12 times in one log.
- Upstream removed SQLite, Drizzle, and better-sqlite3 in `4190a0aa7` (2026-04-14). The pain came from rebuilding better-sqlite3 for the Electron ABI (`149a41501`) and from migration and reconciliation work (`ce821afd2`).
- `node:sqlite` `DatabaseSync` works without flags in the release runtime (Electron 44.2.0, Node 24.20.0, run as `ELECTRON_RUN_AS_NODE`) and in dev Node 24.18.0. It needs no native module.
- [docs/data-model.md](docs/data-model.md) keeps file-based JSON for records and says a store method should map to one SQL statement or transaction.

## Decision

- Mirror every committed timeline row into `$PASEO_HOME/timeline-cache.sqlite` through `node:sqlite` (WAL, synchronous NORMAL, writes batched every 100 ms or 256 rows, at most 500 rows per transaction).
- The file is a rebuildable cache. Provider history stays the authority. Only an agent marked `complete` serves reads; anything else backfills from provider history. A wrong `user_version` or a failed `quick_check` deletes and rebuilds the file. There is no migration code.
- Memory stays the serving layer. Each agent keeps one stored epoch across close, resume, and daemon restart.
- Agent records, projects, workspaces, and config stay JSON files.
- The owner chose `node:sqlite` on 2026-10-15, after the investigation above.

## Consequences

- Reading a closed thread no longer resumes it. Timeline fetch, search, prompt list, attention clear, and the provider subagent list go through `ensureAgentTimelineReadable`. Sending a prompt still resumes.
- If `node:sqlite` is missing or the file cannot open twice, the daemon runs with no store, as before.
- `DatabaseSync` blocks the event loop. Loads page 2,000 rows with a yield between pages; flushes stay small.
- A session changed outside Paseo, for example with `claude --resume`, shows the old copy until Reload agent.
- The first open of each old thread costs one backfill. At most 2 backfills run at once.
- `@types/node` has no `node:sqlite` typings, so the store keeps a local interface and a dynamic import, like the Cursor quota fetcher.

## Options Considered

- Keep timelines in memory only: rejected. Every read after a close or restart pays a resume, measured at p50 10.3 s.
- Append-only JSONL per agent: rejected. A seq window or the prompt list would need a full file scan or an extra index file.
- better-sqlite3 or Drizzle: rejected. Upstream removed them after Electron ABI rebuilds and migration pain; `node:sqlite` gives the same SQL with no native module.
- A full database for all daemon state: rejected. Agent listing scans about 940 records in memory at p50 7 ms, and startup loads all JSON in about 150 ms.

## Revisit If

- `node:sqlite` stops working in the Electron runtime the daemon uses, or needs a flag.
- `timeline-cache.sqlite` grows past 1 GB, or a `load` blocks the daemon loop for more than 100 ms in `ws_runtime_metrics`.
- Agent listing or startup becomes slow enough to need indexed queries over agent records.
