# Idle Runtime Closer

Server-only Paseo plugin that releases the processes of idle threads. A thread idle for more
than 30 minutes has its runtime closed: the provider CLI and its MCP servers exit, and the
thread stays in the list with status `closed`. Opening it or sending a prompt resumes it with
its conversation intact.

Nothing is archived or deleted. To archive old threads, use
[thread-janitor](../thread-janitor/README.md).

## What gets closed

An agent's runtime is closed when all of these are true:

- It is not archived.
- Its status is `idle` or `error`. `running` and `initializing` are in use; `closed` holds
  nothing.
- It has no pending permission request.
- No descendant is `running` or `initializing`. Parent links use the `paseo.parent-agent-id`
  label, and every ancestor of an active agent is skipped.
- Its last activity is older than `idleMinutes`. Last activity is the latest of the agent's
  `updatedAt`, its `lastUserMessageAt`, and the time this plugin last saw its runtime open.
  Resuming a thread keeps its stored activity time, so the open time is what gives a thread
  you opened to read its full `idleMinutes`.

Right before closing, the plugin lists agents again and re-checks these rules, because closing
cancels a running turn. One failed close does not stop the rest of the sweep.

## What closing costs

Work parked inside the provider process dies with it and does not come back on resume:
background shells, `Monitor` watches, and workflows started by the agent. The plugin cannot see
that work, so an `idle` agent waiting on a background shell is closed like any other. Raise
`idleMinutes` above your longest background job, or disable the plugin on that host.

MCP servers restart with the runtime, so state they held in memory is lost.

## When sweeps run

- After each `agent.turn_ended` hook.
- Every minute on a timer. The timer reuses the Paseo API from the last `agent.turn_ended` or
  `agent.session_open` hook, so timer sweeps start after the first of those following a plugin
  load or reload.
- Never two at once.

## Settings

Settings id `closer`, host scope, version 1:

| Setting       | Default | Rule          |
| ------------- | ------- | ------------- |
| `enabled`     | `true`  | boolean       |
| `idleMinutes` | `30`    | number, min 1 |

There is no settings screen. Values are stored in
`$PASEO_HOME/plugin-settings/idle-runtime-closer/closer.json`:

```json
{ "version": 1, "values": { "enabled": true, "idleMinutes": 30 } }
```

Settings are read at the start of every sweep, so edits apply within a minute without a reload.
An invalid file skips the sweep and logs the error.

## Logs

```sh
paseo plugin logs idle-runtime-closer
```

Each closed agent logs its id. A sweep that found idle agents logs one line with the closed,
idle, checked, and failed counts. Sweeps that found nothing log nothing.

## Develop and install

```sh
cd plugins/idle-runtime-closer
npm install
npm run format
npm run typecheck
npm run lint
npm test
paseo plugin install "$PWD" --id idle-runtime-closer
```

After edits, run `paseo plugin reload idle-runtime-closer`. Requires the fork host
`>=0.10.2-beta.900` with `server_info.features.agentRuntimeClose`; on a host without it every
close fails with "Update the host to close an agent runtime."
