# TASK-130 Outcome

## Status

DONE. Every spec check passes, and the anti-goal holds at all 3 checkpoints.

## Changed

- `packages/server/src/server/agent/sqlite-timeline-store.ts`: `SCHEMA_VERSION` 4; `timelines.transcript_fingerprint`. `setComplete(agentId, complete, transcriptFingerprint?)` stores it with `complete: true` and clears it otherwise; `getMeta` and `load` return it when set.
- `packages/server/src/server/agent/agent-timeline-store-types.ts`: `transcriptFingerprint?` on `DurableTimelineMeta`; hook interface `TranscriptFingerprintSource`; `CachedHistoryResumeOptions { historyCached? }`.
- `packages/server/src/server/agent/providers/claude/agent.ts`: `ClaudeAgentClient.getTranscriptFingerprint(handle, overrides?)`: sha256 of path, size, and `mtimeMs` of the transcript and each subagent `.jsonl`, from `stat` and `readdir` only; null when missing. `resolveClaudeHistoryPath` moved to module level. `resumeSession` takes `historyCached`: the session defers `loadPersistedHistory` until `ensureQuery`, `streamHistory`, or a rewind needs it.
- `packages/server/src/server/agent/agent-manager.ts`: close and backfill store the fingerprint (backfill takes it before the read). `loadTimelineForRead` compares it for a non-live timeline, from memory or the cache; a change or a null fingerprint drops the cached copy and returns `missing`, so the caller backfills. `resumeAgentFromPersistence` compares it before `resumeSession`: equal passes `historyCached`; different drops the cached copy, and registration rebuilds from provider history. A provider without the hook keeps today's path.
- `packages/server/src/server/agent/agent-loading.ts`: not changed; the resume path lives in `agent-manager.ts`.
- Tests: `agent-manager.durable-timeline.test.ts` (5 cases), `sqlite-timeline-store.test.ts` (1 case).

## Contract

- `TranscriptFingerprintSource.getTranscriptFingerprint(handle: AgentPersistenceHandle, overrides?: Partial<AgentSessionConfig>): Promise<string | null>`; the manager detects it on the client at run time.
- `DurableTimelineStore.setComplete(agentId, complete, transcriptFingerprint?: string | null)`.
- `AgentResumeSessionOptions & CachedHistoryResumeOptions` reaches `client.resumeSession` only when the fingerprints match.

## Verified

- `npx vitest run packages/server/src/server/agent/agent-manager.durable-timeline.test.ts --bail=1` -> PASS 19/19: unchanged transcript reads 0 bytes on open and on resume; appended transcript backfills on open and shows the new row; a change before a resume rebuilds from provider history; a deleted transcript backfills; a provider without the hook and a Codex thread keep today's path.
- `npx vitest run packages/server/src/server/agent/sqlite-timeline-store.test.ts --bail=1` -> PASS 16/16.
- `npx vitest run packages/server/src/server/agent/providers/claude/history-records.test.ts --bail=1` -> PASS 4/4.
- Extra: Claude `agent.sub-agent-replay.test.ts` 20/20, `agent.voice-history-regression.test.ts` 5/5, `agent.env.test.ts` 2/2, `agent.rewind-anchors.test.ts` 4/4, `rewind.test.ts` 5/5; `agent-loading.test.ts` 6/6; `agent-manager.test.ts` 194/194.
- Real Claude hook, scratch probe `/tmp/t130/probe2.mts` -> stable on repeat, changes on a new subagent file and on an append, null for a missing file.
- Group-level run, once after this TASK: `npm run typecheck` -> exit 0 (2026-10-15T04:29Z). Lint and format check of the 13 group files -> clean.
- Impact warnings: Claude `resumeSession` CRITICAL, `ensureQuery` HIGH, `streamHistory` HIGH. Each change is a no-op unless the manager passes `historyCached`.

## Anti-goal

Opening a closed thread with an unchanged transcript reads 0 transcript bytes. Tripwire not crossed.

| Checkpoint              | Value                         | Source                                                                               | Time (UTC)           |
| ----------------------- | ----------------------------- | ------------------------------------------------------------------------------------ | -------------------- |
| Baseline                | open 0 bytes, resume 16 bytes | read counter, "a provider without the hook keeps today's path" (today's path)        | 2026-10-15T04:26:35Z |
| After the compare lands | open 0 bytes, resume 0 bytes  | read counter, "an unchanged transcript serves the cache and skips the history parse" | 2026-10-15T04:26:35Z |
| Completion              | open 0 bytes, resume 0 bytes  | same test                                                                            | 2026-10-15T04:29:33Z |

## Lessons

- The baseline was read from the hookless path in the same test file, because the code before this TASK was not run separately. The read path already read 0 bytes; the resume path read the whole transcript.
- A deferred Claude history still loads before the first turn, because task state and rewind targets come from the transcript. A resume that starts a turn still parses once.
- A complete Claude entry with no stored fingerprint backfills on open. A transcript that never exists makes each open backfill, at no file-read cost.
- `CLAUDE_CONFIG_DIR` from an agent's launch env is not seen by the client hook; such an agent backfills on every open.
