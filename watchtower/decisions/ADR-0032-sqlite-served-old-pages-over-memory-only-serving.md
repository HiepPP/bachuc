# ADR-0032 SQLite-served old timeline pages over memory-only serving

Status: accepted
Date: 2026-10-15
Supersedes: ADR-0031
Scope: `packages/server/src/server/agent/sqlite-timeline-store.ts`, `packages/server/src/server/agent/agent-timeline-store.ts`, `packages/server/src/server/agent/agent-timeline-store-types.ts`, `packages/server/src/server/agent/agent-manager.ts`, `packages/server/src/server/agent/agent-loading.ts`, `packages/server/src/server/agent/chat-search/**`, `packages/server/src/server/agent/timeline-prompt-index.ts`, `packages/server/src/server/bootstrap.ts`, `package.json`, `packages/*/package.json`
Guard: forbid-text (better-sqlite3|drizzle-orm) in package.json packages/server/package.json packages/desktop/package.json packages/cli/package.json

## Context

- All facts in [ADR-0031](watchtower/decisions/ADR-0031-node-sqlite-timeline-cache-over-json-and-better-sqlite3.md) still hold: `node:sqlite` runs without flags in the release runtime, provider history is the authority, and the cache file is rebuildable.
- A live agent keeps every row in memory with no cap ([agent-timeline-store.ts](packages/server/src/server/agent/agent-timeline-store.ts) line 51). Only non-live timelines are capped, at 64 ([agent-manager.ts](packages/server/src/server/agent/agent-manager.ts) line 108).
- Chat search parses every message with `markdown-it` on each request, and the prompt list copies all rows ([chat-search/index.ts](packages/server/src/server/agent/chat-search/index.ts) line 59, [timeline-prompt-index.ts](packages/server/src/server/agent/timeline-prompt-index.ts) line 24).
- FTS5 with `unicode61 remove_diacritics 2` works in dev Node 24.18.0 (SQLite 3.53.1) and in the release Electron runtime, Node 24.20.0 (SQLite 3.53.4). Probe run on 2026-10-15.

## Decision

- Mirror every committed timeline row into `$PASEO_HOME/timeline-cache.sqlite` through `node:sqlite` (WAL, synchronous NORMAL, writes batched every 100 ms or 256 rows, at most 500 rows per transaction). This part of ADR-0031 is unchanged.
- The file stays a rebuildable cache. Only an agent marked `complete` serves reads. A wrong `user_version` or a failed `quick_check` deletes and rebuilds the file. There is no migration code.
- Memory serves only a bounded tail of each timeline, live or not. Older pages, the prompt list, and search candidates come from SQL over the cache. Search uses an FTS5 table to find candidate rows, then counts matches with the existing block text logic.
- Each agent keeps one stored epoch across close, resume, and daemon restart.
- Agent records, projects, workspaces, and config stay JSON files.
- The owner chose the bounded tail on 2026-10-15, in the `watchtower new` interview for the SQLite RAM and CPU plan.

## Consequences

- A live agent's memory no longer grows with its history. Reading an old page costs one indexed SQL read.
- Old pages and search depend on the cache. When the cache is missing or unusable, the daemon falls back to full memory timelines as before ADR-0031.
- `DatabaseSync` blocks the event loop, so every SQL read stays paged and small.
- The schema version goes up, so the first start after the upgrade rebuilds the cache once.
- All ADR-0031 consequences for closed threads, backfills, and `@types/node` typings still apply.

## Options Considered

- Memory as the only serving layer (ADR-0031): rejected. A long live agent keeps its whole history in memory, and search re-parses it on each request.
- Keep full timelines in memory and add only FTS5 and a prompt index: rejected. It cuts search CPU but not memory.
- Keep timelines in memory only, append-only JSONL, better-sqlite3 or Drizzle, and a full database for all daemon state: rejected in ADR-0031 for the reasons there. Revisit If: does not hold.

## Revisit If

- Old-page reads from SQL make `eventLoopDelay.maxMs` in `ws_runtime_metrics` pass 100 ms during normal use.
- `node:sqlite` stops working in the Electron runtime the daemon uses, or FTS5 is missing from its SQLite build.
- `timeline-cache.sqlite` grows past 1 GB.
