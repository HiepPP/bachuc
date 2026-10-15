# TASK-128 Outcome

## Status

DONE. Every spec check passes, and the anti-goal holds at all 3 checkpoints.

## Changed

- `packages/server/src/server/agent/sqlite-timeline-store.ts`: `SCHEMA_VERSION` 2 (TASK-129 and TASK-130 raise it to 4). `timeline_rows.item_type` with index `(agent_id, item_type, seq)`. Table `timeline_messages` holds each projected user or assistant message: assistant chunks join by the `mergeAssistantChunks` rules (consecutive seq, same turn, compatible `messageId`), across flushes. FTS5 table `timeline_search` (`content=''`, `contentless_delete=1`, `tokenize='unicode61 remove_diacritics 2'`) indexes that text by message id. Flush keeps both in step in the same transaction; a rewritten row rebuilds its message. New methods `listPromptRows` and `findSearchCandidates`; both flush the agent first and return null for an agent with a lost write.
- `packages/server/src/server/agent/agent-timeline-store-types.ts`: `DurableTimelineSearchCandidate`; optional `listPromptRows?` and `findSearchCandidates?` on `DurableTimelineStore`.
- `packages/server/src/server/agent/chat-search/index.ts`: `searchCandidates` counts matches in candidate messages only; `searchTimeline` now feeds it the projected messages. Exported `SearchCandidate` and `SearchResult`.
- `packages/server/src/server/agent/timeline-prompt-index.ts`: doc comment only; it already builds from user rows.
- `packages/server/src/server/agent/agent-manager.ts`: `getTimelinePromptRows` and `searchTimeline` read through the cache when it serves the agent (`cacheServingTimeline`: same epoch, no missed write), else memory. A cache error logs and falls back to memory.
- `packages/server/src/server/session.ts`: the list-prompts and search handlers call the two manager methods.
- Tests: `sqlite-timeline-store.test.ts`, `chat-search/index.test.ts`, `timeline-prompt-index.test.ts`, `agent-manager.durable-timeline.test.ts`.

## Contract

- `listPromptRows(agentId): AgentTimelineRow[] | null`: user message rows in seq order.
- `findSearchCandidates(agentId, query, afterSeq, limit): DurableTimelineSearchCandidate[] | null`: `{ seq: last source seq, role, text }` in seq order. Each query word is a prefix term (`"word"*`, AND). A query with no word lists every message.
- `AgentManager.getTimelinePromptRows(id)`, `AgentManager.searchTimeline(id, { query, cursor })`.
- Final counts use the existing block text logic. On the FTS path, a query typed without diacritics matches text with them (`đ` counts as `d`); a query with diacritics matches exactly. The JS fallback path is unchanged.

## Verified

- `npx vitest run packages/server/src/server/agent/sqlite-timeline-store.test.ts --bail=1` -> PASS 14/14 (04:05Z), 16/16 at group end (04:29Z, with TASK-129 and TASK-130 cases).
- `npx vitest run packages/server/src/server/agent/chat-search/index.test.ts --bail=1` -> PASS 9/9, including parity on a word split across chunks, 200-location paging parity, and the diacritic cases.
- `npx vitest run packages/server/src/server/agent/timeline-prompt-index.test.ts --bail=1` -> PASS 3/3.
- `npx vitest run packages/server/src/server/agent/agent-manager.durable-timeline.test.ts --bail=1` -> PASS 11/11 (04:05Z), 19/19 at group end.
- Extra: `agent-manager.test.ts` PASS 194/194; `session.test.ts -t "thread reads of a closed agent"` PASS 2/2.
- Group-level run, once after TASK-130: `npm run typecheck` -> exit 0 (04:29Z). `rtk proxy npm run lint -- <13 group files>` -> 0 errors. `npm run format:check:files -- <13 group files>` -> exit 0.

## Anti-goal

Search on a 5,000-message timeline parses at most the candidate messages with `markdown-it`. Tripwire not crossed.

| Checkpoint     | Value                | Source                                                                       | Time (UTC)           |
| -------------- | -------------------- | ---------------------------------------------------------------------------- | -------------------- |
| Baseline       | 5,000 parses, 5 hits | scratch `/tmp/t128/baseline.mts`, JS path, same 5,000 rows                   | 2026-10-15T03:55:56Z |
| After FTS path | 5 parses, 5 hits     | `chat-search/index.test.ts` parse counter (`MarkdownIt.prototype.parse` spy) | 2026-10-15T04:05:09Z |
| Completion     | 5 parses, 5 hits     | same test                                                                    | 2026-10-15T04:29:33Z |

## Lessons

- FTS5 with `unicode61` matches whole words or prefixes, never a match that starts inside a word. On the cache path, a query whose first word is the inside of a word (`orld` for `world`) finds nothing, while the JS path finds it.
- `unicode61 remove_diacritics 2` does not fold `đ` to `d`. The store folds it before indexing and in the query.
- New `DurableTimelineStore` methods are optional, so `RecordingTimelineStore` in `agent-manager.test.ts` keeps compiling and takes the memory path.
