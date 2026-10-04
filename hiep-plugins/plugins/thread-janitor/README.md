# Thread Janitor

Server-only Paseo plugin that releases the processes of stale threads. A thread with no
activity for more than 24 hours has its runtime closed: the provider CLI and its MCP servers
exit, and the thread stays in the list with status `closed`. Opening it or sending a prompt
resumes it with its conversation intact.

The janitor never archives or deletes threads, files, or worktrees. It archives only empty
sidebar workspaces, as described below.

## Why it closes runtimes soon after a resume

Reading a closed thread's timeline resumes its runtime. When the app reconnects, the app and
some plugins read thread timelines, so one reconnect can start dozens of provider processes,
each with its own MCP servers. On 2026-10-04 one reconnect resumed 43 threads. The `agent.session_open` hook reports these resumes the same way
as a thread you open, so [idle-runtime-closer](../idle-runtime-closer/README.md) keeps each one
for its full `idleMinutes`. The janitor closes the stale ones a few minutes after the burst.

## What gets closed

A thread's runtime is closed when all of these are true:

- It is not archived.
- Its status is `idle` or `error`. `running` and `initializing` are in use; `closed` holds
  nothing.
- It has no pending permission request.
- No descendant is `running` or `initializing`. Parent links use the `paseo.parent-agent-id`
  label, and every ancestor of an active agent is skipped.
- Its last activity is older than `idleHours`. Last activity is the later of the agent's
  `updatedAt` (the daemon's stored last-activity time) and `lastUserMessageAt`. A resume does
  not change it, so a stale thread stays stale after a reconnect.

Right before closing, the janitor lists agents again and re-checks these rules, because closing
cancels a running turn. One failed close does not stop the rest of the sweep.

Work parked inside the provider process dies with it and does not come back on resume:
background shells, `Monitor` watches, and workflows started by the agent. A stale thread that
you open to read is closed again after the next resume sweep. Sending a prompt resumes it.

## Sidebar workspaces

The sidebar lists workspaces, not agents, so archived agents alone leave their rows behind.
After the runtime pass, the janitor archives a workspace when all of these are true:

- No unarchived agent belongs to it, by workspace id or by directory. A closed thread still
  counts, so closing a runtime never archives its workspace.
- Its `activityAt` (or `statusEnteredAt`) is older than `idleHours`.
- It is not pinned, not already archiving, and its status is not `running` or `needs_input`.
- It is not a Paseo-owned worktree. Archiving one can remove its directory, so it is skipped.
- It has no open terminal. Archiving a workspace kills its terminals, so it is skipped.

The janitor re-fetches the workspace right before archiving and checks the rules again.

## When sweeps run

- 5 minutes after the last `agent.session_open` hook. Each resume pushes the sweep back, so a
  reconnect burst is handled by one sweep. This sweep ignores the throttle below.
- After `agent.turn_ended` and `agent.created` hooks.
- Every 30 minutes on a timer.
- Hook and timer sweeps run at most once per 10 minutes. Two sweeps never run at once.

Timer sweeps reuse the Paseo API from the last hook, so they start only after the first hook
fires following a plugin load or reload.

## Settings

Settings id `janitor`, host scope, version 1:

| Setting     | Default | Rule          |
| ----------- | ------- | ------------- |
| `enabled`   | `true`  | boolean       |
| `idleHours` | `24`    | number, min 1 |

Setting `enabled` to `false` stops all closing and workspace archiving. There is no settings
screen. Values are stored in `~/.paseo/plugin-settings/thread-janitor/janitor.json`:

```json
{ "version": 1, "values": { "enabled": true, "idleHours": 24 } }
```

A `keepRecent` value left from the archiving version is ignored. Settings are read at the start
of every sweep, so edits apply to the next sweep without a reload. An invalid file skips the
sweep and logs the error.

## Logs

```sh
paseo plugin logs thread-janitor
```

Each sweep logs one line with the closed, stale, checked, and failed counts for runtimes, and
the archived counts for workspaces. Each closed thread logs its id, and each archived workspace
logs its id. Titles are never logged.

## Develop and install

```sh
cd plugins/thread-janitor
npm install
npm run format
npm run typecheck
npm run lint
npm test
paseo plugin install "$PWD" --id thread-janitor
paseo plugin ls thread-janitor --json
```

After edits, run `paseo plugin reload thread-janitor`. Requires the fork host
`>=0.10.2-beta.900` with `server_info.features.agentRuntimeClose`; on a host without it every
close fails with "Update the host to close an agent runtime."
