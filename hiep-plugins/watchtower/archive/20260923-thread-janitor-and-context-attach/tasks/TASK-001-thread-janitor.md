# TASK-001 Thread Janitor plugin

Group: A (writes only `plugins/thread-janitor/`)
Class: risky

## Brief

Goal: Archive idle Paseo threads automatically after 1 day, with no confirmation. Keep the thread list short without manual cleanup.

Change: idle threads stay in the list forever -> threads idle for 24 hours are archived automatically.

How:

- Scaffold `plugins/thread-janitor/` with `paseo plugin init`, as in [AGENTS.md](AGENTS.md).
- Register settings: `enabled` (default `true`) and `idleHours` (default `24`, minimum `1`).
- Write a pure function `selectStale(agents, now, settings)` in `shared/` or `server/`. It returns agents to archive.
- An agent is stale when all are true: not archived, `lastActivityAt` is older than `idleHours`, status is not `running`, and it has no pending permission.
- Run a sweep with the `PaseoApi` from hook context. Trigger it on `agent.turn_ended` and `agent.created`. Throttle to at most one sweep per 10 minutes.
- Also run a timer every 30 minutes. The timer reuses the last `PaseoApi` seen in a hook. Skip the timer sweep until one is seen.
- Archive each stale agent with `agents.ref(id).archive()`. Log each archived id and title length, not the title text. One failure must not stop the sweep.
- Clear the timer and listeners in the cleanup function.
- Write tests for `selectStale`: idle boundary, running agent, pending permission, already archived, disabled setting.
- Write a README with the default, the settings, and how to unarchive.

Files:

- [plugins/thread-janitor/](plugins/thread-janitor/) (new plugin: manifest, package files, `index.server.ts`, `server/`, `tests/`, `README.md`)

Expected result:

- A thread idle for more than 24 hours is archived within 30 minutes of the next sweep, with no dialog.
- Running threads, threads waiting for permission, and threads active in the last 24 hours stay in the list.
- Setting `enabled` to `false` stops all archiving.

## Verify

- `cd plugins/thread-janitor && npm run typecheck && npm run lint && npm test` -> all pass, including the five `selectStale` cases.
- `paseo plugin install "$PWD/plugins/thread-janitor" --id thread-janitor`, then `paseo plugin ls thread-janitor --json` -> `running`, no load error.
- `paseo plugin logs thread-janitor` after one agent turn ends -> shows a sweep line with an archived count.
- Manual check in the Paseo app: an agent idle for more than 24 hours is archived. An agent with activity in the last hour is not archived.
