# Planning Memory

## Core Intent

- Improve autonomous Paseo delegation through plugins, without editing Paseo application source.
- Route subagents using valid saved profiles and measured task outcomes.

## Planning Rules

- Keep provider settings intact and preserve explicit user selections.
- Separate deterministic verification, model judgments, and live runtime evidence.
- Keep raw prompts, credentials, and local run state outside Git.
- Extend an existing plugin that already owns the data before adding a new plugin. The user asked for this on 2026-09-24.

## Source Anchors

- [Jev evaluator](../plugins/jev-evaluator/) provides the existing Gateway evaluation contract.
- [Orchestrator](../plugins/jev-orchestrator/) owns autonomous delegation.

## Verified Integration Lessons

- Installed Paseo 0.8 server bundling failed on a direct AI SDK import; a standalone Node worker loads Gateway successfully.
- Re-evaluate readiness after discovery. A review finding must not train routing as a successful outcome.
- Keep interrupted benchmark samples and distinguish extra review work from routing-speed comparisons.
- 2026-09-23: The daemon rejects camelCase plugin RPC names such as `threadContext.listThreads`. Use lowercase-hyphen names such as `thread-context.list-threads`.
- 2026-09-23: An attachment-source search receives only `{ query }`. It cannot know the current thread or workspace.
- 2026-09-23: SDK 0.9.0-beta.2 agent snapshots have no `lastActivityAt`. Use the newest of `updatedAt` and `lastUserMessageAt`.
- 2026-09-23: A local command hook rewrites a literal `npm run lint` and exits 1. Run `S=lint; npm run $S` to get the real oxlint result.
- 2026-09-23: The Paseo sidebar lists workspaces, not agents. Archiving agents leaves their workspace rows visible. Archive the workspace too, but never a Paseo-owned worktree, because that can remove its directory.
- 2026-09-23: An attachment item's `text` is built inside the search RPC, and there is no hook when the user selects an item. Slow work such as Jev must run in the background and be served from cache on a later search.
- 2026-09-23: Jev is a judge, not a search engine. For finding the right thread or passage, use local search such as qmd. A Jev picker label was built, rejected by the user, and reverted.
- 2026-09-23: qmd's launcher runs the first `node` on PATH, and its `better-sqlite3` module only loads on the Node it was built for (`~/.local/bin/node` v24). When spawning qmd from the daemon, keep the user's PATH order. Putting `/opt/homebrew/bin` first picks Node v25 and qmd exits 1.
- 2026-09-23: A plugin gets a Paseo API only inside hooks and RPCs, not at load. Start startup work such as a backfill on the first hook or RPC.
- 2026-09-24: Node `execFile` sets `error.code` to null when a timeout kills the process. A runner that maps null to 0 reports a timeout as success. Check `killed` or `signal` first.
- 2026-09-24: `paseo agent logs` and the MCP activity summary hide plugin timeline rows. To check a plugin row, run `createPaseoClient({ url: "ws://127.0.0.1:6767/ws" })` then `agents.ref(id).timeline.refetch({ direction: "tail" })` from a plugin folder.
- 2026-09-24: When snapshotting through a copy of the git index, copy the index mtime too (`utimes`). A fresh mtime disables git's racy-clean check, so a same-size rewrite in the same second is missed.
- 2026-09-24: The daemon rebuilds agent timelines from the provider session file. Plugin rows from `timeline.append` are not stored on disk, so a plugin that needs history must keep its own file under `plugin-data/`.
- 2026-09-24: A snapshot through `GIT_INDEX_FILE` must fail closed. Check `git rev-parse --git-path index` returns the temp path before `git add -A`. A runner that drops `env` once staged a whole repo.
- 2026-09-24: A session reload (`paseo agent reload`, Refresh, a stale session) rebuilds the timeline and drops every plugin row, just like a daemon restart. It also restarts turn ids at `<owner>-turn-1`, so a turn id is not unique per agent.
- 2026-09-24: The user wants low RAM and low storage from plugin features. Prefer on-disk files read on demand over in-memory caches. Keep anything stored under `.git` or `plugin-data` bounded by size and age.
- 2026-09-24: A local git wrapper adds decorations such as `--- Changes ---` to piped output of porcelain commands like `git diff`. For machine checks, use plumbing such as `git diff-index --cached --name-status HEAD`. Plumbing lists a rename as a D and an A.
- 2026-09-29: Paseo daemon 0.10.1 fails to load a plugin whose server `contribute` returns nothing ("Plugin contribution must return a cleanup function"). Always return a cleanup function, even `() => {}`. Typecheck against SDK 0.9.0-beta.2 does not catch this.
- 2026-09-29: `prompt-translate` stops the original composer Enter and replays its own. Another plugin should not capture Enter to record per-send state. Watch for new host queue rows instead, and read saved state for direct sends.
- 2026-09-29: `agent.created` carries no env. To act on env that `before("agent.create")` added, use `before("agent.session_open")` with `reason === "create"`; it has `agentId` and `env` and runs before the first turn.
- 2026-09-29: An `agent.create` request does not say which composer or parent sent it. State handed from a new-thread composer to the created agent needs a shared slot, and any other create can take it.
- 2026-09-29: A live skill check prompt must not say "reply with one word". The pinned-skill context lets prompt instructions win, so the model skips the skill.
