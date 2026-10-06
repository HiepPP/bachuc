# PR watch

Server-only Paseo plugin. An agent asks Paseo to watch its GitHub pull request, then keeps
working or stops. Every 2 minutes the plugin reads the PR with `gh` and sends the agent a
prompt when something it should react to happens.

## Tools

The daemon `paseo` MCP server serves these tools to agents:

| Tool                        | What it does                                                                                                                                           |
| --------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `watch_pull_request`        | Watch a PR. Omit `number` to watch the PR of the branch checked out in the agent's working directory. Returns the starting checks and mergeable state. |
| `unwatch_pull_request`      | Stop one watch, or all of the agent's watches when `number` is omitted.                                                                                |
| `list_pull_request_watches` | List the agent's watches with their last known state.                                                                                                  |

## What wakes the agent

The first read is a baseline. After that, each poll compares the PR with the last read:

- a check starts failing;
- every check passes (the plugin cannot tell required checks apart, so it uses all of them);
- a new comment or review from someone else; comments by the `gh` login and by `*[bot]`
  accounts are ignored;
- the PR gets merge conflicts;
- the PR is merged or closed. This also ends the watch.

The prompt arrives inside a `<pr-watch>` block. If the agent is running when a change is
found, the prompt waits until its turn ends; it never steers a running turn.

## Limits

- GitHub only, through an authenticated `gh` CLI on the daemon machine.
- A watch stops after 8 failed reads in a row, and the agent gets one prompt saying so.
- A watch stops when its agent is archived.
- Native OpenCode and OMP agents do not see plugin MCP tools.
- Watches live in `$PASEO_HOME/plugin-data/pr-watch/watches.json`. After a plugin reload or a
  daemon restart, polling resumes on the first agent turn or tool call, because the poll loop
  needs a Paseo connection from a hook.

## Host facts

Built against the fork at `cb5f86fdb`: `server.registerTool`, `PluginToolContext.callerAgentId`,
and the `agent.turn_started`, `agent.turn_ended`, and `agent.archived` hooks. The SDK packages
are linked from this checkout with `file:` paths, like the Board plugin.

## Develop

```sh
npm install
npm run format
npm run typecheck
npm run lint
npm test
```

Install on live from the Paseo repo root:

```sh
npm run cli -- plugin install "$PWD/hiep-plugins/plugins/pr-watch" --id pr-watch
npm run cli -- plugin reload pr-watch
```
