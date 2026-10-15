# TASK-130 Daemon transcript fingerprint

Group: D (writes `packages/server/src/server/agent/sqlite-timeline-store.ts` and `agent-manager.ts`; shared with TASK-128 and TASK-129)
Class: risky

## Brief

Goal: The daemon skips re-parsing a Claude transcript when the file did not change, and rebuilds the cache when it did change outside Paseo.

Change: resume and backfill parse the full JSONL each time, and a complete cache ignores outside edits -> the cache stores a transcript fingerprint; equal skips the parse, different triggers a backfill.

Boundaries:

- Today each history load re-reads and re-parses the parent transcript and every subagent JSONL ([packages/server/src/server/agent/providers/claude/history-records.ts](packages/server/src/server/agent/providers/claude/history-records.ts) line 84, called from `loadPersistedHistory` in [packages/server/src/server/agent/providers/claude/agent.ts](packages/server/src/server/agent/providers/claude/agent.ts) line 4952). Transcripts can pass 100 MB.
- Known gotcha in [docs/timeline-sync.md](docs/timeline-sync.md): after `claude --resume` outside Paseo, the thread shows the old copy until Reload agent. This TASK removes that gotcha for Claude.
- Fingerprint: path, size, and `mtimeMs` of the parent transcript and each subagent file. Store it when the entry is marked complete.
- Only Claude gives a fingerprint in this TASK. A provider without one keeps today's behavior.
- A fingerprint read costs only `stat` calls, never a file read.
- The cache stays rebuildable. Raise `SCHEMA_VERSION`.
- Provider history stays the authority. On any doubt, such as a missing file or a stat error, backfill.

How:

- Add an optional provider hook that returns the transcript fingerprint for an agent.
- Store the fingerprint in the `timelines` table with `setComplete`.
- On a read of a complete entry and on resume, compare fingerprints. Equal: serve the cache and skip the history parse. Different: reset the entry and backfill.
- Add tests: unchanged file skips the parse, an appended file triggers a backfill, a missing file triggers a backfill, and a provider without the hook keeps today's path.

Files:

- [packages/server/src/server/agent/sqlite-timeline-store.ts](packages/server/src/server/agent/sqlite-timeline-store.ts) (fingerprint column)
- [packages/server/src/server/agent/agent-timeline-store-types.ts](packages/server/src/server/agent/agent-timeline-store-types.ts) (fingerprint in meta)
- [packages/server/src/server/agent/agent-manager.ts](packages/server/src/server/agent/agent-manager.ts) (compare on read and resume)
- [packages/server/src/server/agent/agent-loading.ts](packages/server/src/server/agent/agent-loading.ts) (skip the history parse when equal, if the resume path lives here)
- [packages/server/src/server/agent/providers/claude/agent.ts](packages/server/src/server/agent/providers/claude/agent.ts) (fingerprint hook)
- [packages/server/src/server/agent/agent-manager.durable-timeline.test.ts](packages/server/src/server/agent/agent-manager.durable-timeline.test.ts) (fingerprint cases)

Expected result:

- Case: reopen a closed Claude thread whose transcript did not change -> no transcript file read.
- Case: append to the transcript outside Paseo, then open the thread -> the new rows show without Reload agent.
- Case: delete the transcript -> the entry backfills and follows the current error path.
- Case: a Codex thread -> same behavior as before this TASK.

Anti-goal: Opening a closed thread with an unchanged transcript reads 0 transcript bytes; tripwire; read from a read counter in `agent-manager.durable-timeline.test.ts` at baseline, after the fingerprint compare lands, and at completion.

## Verify

- `npx vitest run packages/server/src/server/agent/agent-manager.durable-timeline.test.ts --bail=1` -> pass, including the 4 cases above.
- `npx vitest run packages/server/src/server/agent/sqlite-timeline-store.test.ts --bail=1` -> pass.
- `npx vitest run packages/server/src/server/agent/providers/claude/history-records.test.ts --bail=1` -> pass.
- `npm run typecheck` -> exit 0.
- Anti-goal: the read counter test reports 0 transcript bytes for the unchanged case -> within limit.
