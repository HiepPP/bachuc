# Thread Janitor

Server-only Paseo plugin that archives idle threads automatically, with no confirmation
dialog. By default a thread idle for more than 24 hours is archived, except for the
seven most recently active unarchived threads in each workspace.

Archive is reversible. The janitor never deletes agents, files, or worktrees.

## What gets archived

An agent is archived when all of these are true:

- It is not already archived.
- It is outside the `keepRecent` most recently active threads of its workspace. Threads are
  grouped by workspace id, or by directory when the id is missing. The same last-activity
  timestamp below determines this order; ties use agent ID.
  Threads with unknown activity remain protected.
- It is not an ancestor of a retained thread. Parent links use the
  `paseo.parent-agent-id` label. All ancestors remain protected because archiving a parent
  can cascade to its descendants; this may leave more than `keepRecent` threads.
- Its last activity is older than `idleHours`. Last activity is the later of the agent's
  `updatedAt` (the daemon's stored last-activity time) and `lastUserMessageAt`.
- Its status is not `running`.
- It has no pending permission request.

Right before archiving, the janitor refreshes the unarchived list and the agent, then re-checks these
rules, because archiving a live agent cancels its turn. One failed archive does not stop the
rest of the sweep.

## Sidebar workspaces

The sidebar lists workspaces, not agents, so archived agents alone leave their rows behind.
After the agent pass, the janitor archives a workspace when all of these are true:

- No unarchived agent belongs to it, by workspace id or by directory.
- Its `activityAt` (or `statusEnteredAt`) is older than `idleHours`.
- It is not pinned, not already archiving, and its status is not `running` or `needs_input`.
- It is not a Paseo-owned worktree. Archiving one can remove its directory, so it is skipped.
- It has no open terminal. Archiving a workspace kills its terminals, so it is skipped.

The janitor re-fetches the workspace right before archiving and checks the rules again.

## When sweeps run

- After `agent.turn_ended` and `agent.created` hooks.
- Every 30 minutes on a timer. The timer reuses the Paseo API from the last hook, so timer
  sweeps start only after the first hook fires following a plugin load or reload.
- At most one sweep runs per 10 minutes, and never two at once.

## Settings

Settings id `janitor`, host scope, version 1:

| Setting      | Default | Rule           |
| ------------ | ------- | -------------- |
| `enabled`    | `true`  | boolean        |
| `idleHours`  | `24`    | number, min 1  |
| `keepRecent` | `7`     | integer, min 0 |

Setting `enabled` to `false` stops all archiving. There is no settings screen. Values are stored
in `~/.paseo/plugin-settings/thread-janitor/janitor.json`:

```json
{ "version": 1, "values": { "enabled": false, "idleHours": 24, "keepRecent": 7 } }
```

`keepRecent` applies to each workspace and is configured independently on each host. Set it
to `0` to disable retention. If a workspace has fewer than `keepRecent` threads, all of them
remain protected. Retained threads also protect
their workspaces. This setting does not restore threads that were already archived.

Settings are read at the start of every sweep, so edits apply to the next sweep without a
reload. An invalid file skips the sweep and logs the error.

## Logs

```sh
paseo plugin logs thread-janitor
```

Each sweep logs one line with the archived, stale, checked, and failed counts for agents and
workspaces. Each archived agent logs its id and title length, and each archived workspace logs
its id. Titles are never logged.

## Unarchive

- App: open the thread from History. On the "This agent is archived" banner, choose
  Unarchive. Sending a new message to an archived thread also unarchives it. Paseo restores
  the thread's archived workspace when it restores the thread.
- CLI: find the id with `paseo ls -a` or in the plugin logs, then send it a message with
  `paseo send <id> "<prompt>"`.

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

After edits, run `paseo plugin reload thread-janitor`. Requires Paseo `>=0.9.0-beta.2`.
