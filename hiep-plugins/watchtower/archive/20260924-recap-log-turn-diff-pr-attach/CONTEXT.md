# Plan Context

## Shared Context

- The user wants no new plugin. Each feature goes into an existing plugin that already owns the same data.
- `agent.turn_started` gives `{ agent, turnId }`. `agent.turn_ended` gives `{ agent, turnId, outcome, timeline }`. `agent` has `id`, `workspaceId`, `cwd`, and `title`. See [plugins/board/node_modules/@getpaseo/plugin/dist/server/lifecycle.d.ts](plugins/board/node_modules/@getpaseo/plugin/dist/server/lifecycle.d.ts).
- The `timeline` in `agent.turn_ended` holds the ended turn. [plugins/loop-verify/index.server.ts](plugins/loop-verify/index.server.ts) reads the last `assistant_message` from it.
- Do slow work outside the hook. Start it with `void` so the hook returns at once, as in [plugins/loop-verify/index.server.ts](plugins/loop-verify/index.server.ts).
- A plugin timeline row is `paseo.agents.ref(id).timeline.append({ type: "plugin", id, kind, version, data })` plus `client.addTimelineRenderer(...)`. See [plugins/loop-verify/server/paseo.ts](plugins/loop-verify/server/paseo.ts) and [plugins/loop-verify/index.client.tsx](plugins/loop-verify/index.client.tsx).
- A plugin gets a Paseo API only inside hooks and RPCs, not at load.
- RPC names must be lowercase with hyphens, for example `board.recaps`. The daemon rejects camelCase names.
- Plugin data lives under `$PASEO_HOME/plugin-data/<plugin>/`, default `~/.paseo`. Write files with mode `0600`.
- Run lint as `S=lint; npm run $S`. A local hook rewrites a literal `npm run lint`.
- Reload a plugin with `paseo plugin reload <name>`.

## Decisions

- Recap log goes into [plugins/board](plugins/board). Board already listens to turn ends and groups conversations by project.
- Turn diff summary and the PR/CI attachment go into [plugins/thread-branch](plugins/thread-branch). It already runs `git` and `gh` per `cwd` and has a 5-minute PR cache.
- The PR picker searches the GitHub repos of all non-archived Paseo workspaces. An attachment search gets only `{ query }`, so it cannot know the current thread.
- The recap log starts at install. Version 1 has no backfill of old threads.
- Nothing is sent to an agent automatically. Attachments are picked by the user.
- No Jev, no qmd, and no new network service. `gh` is the only network client, and it is already used by Thread branch.

## Open Decisions

- None.

## References

- [plugins/board/server/store.ts](plugins/board/server/store.ts) for project names from `cwd`.
- [plugins/board/client/page.tsx](plugins/board/client/page.tsx) for the Board header, where `BoardSizeControl` sits.
- [plugins/thread-branch/server/git.ts](plugins/thread-branch/server/git.ts) for bounded `git` and `gh` calls and the PR cache.
- [plugins/thread-branch/client/pills.ts](plugins/thread-branch/client/pills.ts) for `copyText`.
- [plugins/thread-context-attach/shared/threads.ts](plugins/thread-context-attach/shared/threads.ts) and [plugins/thread-context-attach/server/threads.ts](plugins/thread-context-attach/server/threads.ts) for an attachment source.
