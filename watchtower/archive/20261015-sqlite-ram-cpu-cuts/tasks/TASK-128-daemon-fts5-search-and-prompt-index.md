# TASK-128 Daemon FTS5 search and prompt index

Group: D (writes `packages/server/src/server/agent/sqlite-timeline-store.ts`, `agent-manager.ts`, and `session.ts`; shared with TASK-129 and TASK-130)
Class: risky

## Brief

Goal: Chat search and the prompt list read from the timeline cache with indexes, so they stop scanning and re-parsing the whole timeline in JS.

Change: search runs `markdown-it` on every message and the prompt list copies all rows -> SQL finds prompt rows by index and FTS5 finds search candidates.

Boundaries:

- Follows [ADR-0032](watchtower/decisions/ADR-0032-sqlite-served-old-pages-over-memory-only-serving.md): prompt list and search candidates come from SQL while `node:sqlite` with FTS5 runs in the daemon runtime.
- Today search is a linear scan with a `markdown-it` parse per message ([packages/server/src/server/agent/chat-search/index.ts](packages/server/src/server/agent/chat-search/index.ts) line 59). The prompt list filters a full row copy ([packages/server/src/server/agent/timeline-prompt-index.ts](packages/server/src/server/agent/timeline-prompt-index.ts) line 24; [packages/server/src/server/session.ts](packages/server/src/server/session.ts) lines 7828 and 7877).
- Index projected messages, not source rows. Assistant text arrives in chunks, and one word can split across two chunks.
- FTS5 only narrows candidates. Match counts and hit shapes must stay the same as the current block text logic.
- Use `tokenize='unicode61 remove_diacritics 2'`, so Vietnamese text matches with or without diacritics. Keep the current matching rules for the final count.
- The cache serves an agent only when its rows are trusted: complete, or written by this daemon in the current epoch. Otherwise, keep the current JS path.
- When the store is missing, keep the current JS path. A cache failure never fails a search.
- Raise `SCHEMA_VERSION`. Keep each write transaction within the current batch limits.

How:

- Add an item type column with an index on agent, type, and seq.
- Add an FTS5 table keyed by agent and the first seq of each projected user or assistant message. Upsert it during flush for messages that changed.
- Add store methods: list prompt rows of an agent, and find candidate message seqs for a query.
- Route the prompt list and search through these methods when the cache serves the agent.
- Add tests for prompt list parity, search parity on chunked assistant text, diacritic-free match, and the fallback path.

Files:

- [packages/server/src/server/agent/sqlite-timeline-store.ts](packages/server/src/server/agent/sqlite-timeline-store.ts) (schema, FTS5 upkeep, query methods)
- [packages/server/src/server/agent/agent-timeline-store-types.ts](packages/server/src/server/agent/agent-timeline-store-types.ts) (new store methods)
- [packages/server/src/server/agent/chat-search/index.ts](packages/server/src/server/agent/chat-search/index.ts) (count only candidates)
- [packages/server/src/server/agent/timeline-prompt-index.ts](packages/server/src/server/agent/timeline-prompt-index.ts) (build from prompt rows)
- [packages/server/src/server/agent/agent-manager.ts](packages/server/src/server/agent/agent-manager.ts) (expose the cache path)
- [packages/server/src/server/session.ts](packages/server/src/server/session.ts) (handlers use the new path)
- [packages/server/src/server/agent/sqlite-timeline-store.test.ts](packages/server/src/server/agent/sqlite-timeline-store.test.ts) (schema and query cases)
- [packages/server/src/server/agent/chat-search/index.test.ts](packages/server/src/server/agent/chat-search/index.test.ts) (parity cases)
- [packages/server/src/server/agent/timeline-prompt-index.test.ts](packages/server/src/server/agent/timeline-prompt-index.test.ts) (parity cases)

Expected result:

- The prompt list from SQL equals the list from the JS path for the same timeline.
- Search hits from the FTS path equal the hits from the JS path, including a word split across two assistant chunks.
- A query without diacritics finds Vietnamese text with diacritics.
- With no store, search and the prompt list return the same results as before.

Anti-goal: Search on a 5,000-message timeline parses at most the candidate messages with `markdown-it`, not all 5,000; tripwire; read from a `chat-search/index.test.ts` parse counter at baseline, after the FTS change, and at completion.

## Verify

- `npx vitest run packages/server/src/server/agent/sqlite-timeline-store.test.ts --bail=1` -> pass.
- `npx vitest run packages/server/src/server/agent/chat-search/index.test.ts --bail=1` -> pass, including the parity and diacritic cases.
- `npx vitest run packages/server/src/server/agent/timeline-prompt-index.test.ts --bail=1` -> pass.
- `npx vitest run packages/server/src/server/agent/agent-manager.durable-timeline.test.ts --bail=1` -> pass.
- `npm run typecheck` -> exit 0.
- Anti-goal: the parse counter test shows only candidate messages parsed -> within limit.
