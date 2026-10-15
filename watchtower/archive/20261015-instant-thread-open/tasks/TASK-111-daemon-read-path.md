# TASK-111 Daemon read path and bootstrap

Group: C
Class: risky

## Brief

Goal: Serve thread reads of a closed agent with no resume.

Change: `ensureAgentTimelineReadable` (live, pending load, cache, backfill, then fallback to `ensureAgentLoaded`) serves the timeline fetch, timeline search, prompt list, `clear_agent_attention`, and `agent.provider_subagents.list.request`. Bootstrap opens and closes the store. The daemon sets `timelineLatestPromptTail: true`.

Boundaries:

- `ensureAgentLoaded` stays unchanged (GitNexus CRITICAL, 172 impacted).
- Write handlers keep `ensureAgentLoaded`.
- Follow-up: a cache write error must never fail resume, close, or delete.

Files:

- [packages/server/src/server/agent/agent-loading.ts](packages/server/src/server/agent/agent-loading.ts)
- [packages/server/src/server/agent/agent-loading.test.ts](packages/server/src/server/agent/agent-loading.test.ts)
- [packages/server/src/server/session.ts](packages/server/src/server/session.ts)
- [packages/server/src/server/session.test.ts](packages/server/src/server/session.test.ts)
- [packages/server/src/server/bootstrap.ts](packages/server/src/server/bootstrap.ts)
- [packages/server/src/server/websocket-server.ts](packages/server/src/server/websocket-server.ts)
- [packages/server/src/server/agent/agent-manager.ts](packages/server/src/server/agent/agent-manager.ts) (follow-up only)

Expected result:

- A closed agent read after a daemon restart logs no "Agent resumed from persistence".

## Verify

- `npx vitest run src/server/agent/agent-loading.test.ts --bail=1` in `packages/server` -> pass.
- `npx vitest run src/server/session.test.ts --bail=1` -> pass.
- `npx vitest run src/server/agent/agent-manager.durable-timeline.test.ts --bail=1` -> pass.
- On live, close an agent, restart the live daemon, fetch its timeline -> no resume line for that id in `.dev/paseo-home/daemon.log`.
