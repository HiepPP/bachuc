# Plan Context

## Shared Context

- Goal: a running agent can find exact context in another Paseo thread, with few tokens. It searches, then reads only the matching turn.
- Plugin: [plugins/thread-context-attach](plugins/thread-context-attach). The composer picker stays as it is. This plan adds export and search only.
- Paseo does not keep thread text as markdown on disk. Files in `~/.paseo/agents/` hold metadata only. Text comes from `paseo.agents.ref(id).timeline.refetch(...)`, as in [plugins/thread-context-attach/server/threads.ts](plugins/thread-context-attach/server/threads.ts).
- `server.on("agent.turn_ended", ({ agent, turnId, outcome, timeline }, { paseo }) => ...)` fires after each turn. See [plugins/thread-janitor/index.server.ts](plugins/thread-janitor/index.server.ts).
- qmd 2.1.0 is installed at `/opt/homebrew/bin/qmd`. `qmd update` has no collection filter. It re-indexes every collection in the index, and the default index holds 7,665 files. Use the named index `--index paseo-threads` so updates touch only thread files.
- The default qmd index has 0 vector embeddings. Plain `qmd search` (BM25 keyword search) needs no model and is fast.
- Run lint as `S=lint; npm run $S`. A local hook rewrites a literal `npm run lint`.

## Decisions

- Export folder: `$PASEO_HOME/plugin-data/thread-context-attach/threads/`, default `~/.paseo/...`. One file per thread: `<agentId>.md`. Files use mode `0600`.
- A file holds only user messages and assistant replies. Tool calls, reasoning, and tool output are left out.
- One `## Turn N` heading per turn. A turn is one user message and the assistant replies after it. This makes a search hit point at one turn.
- Export on `agent.turn_ended`, debounced per thread. Backfill all non-archived threads once when the plugin starts. Keep files of archived threads.
- Read at most 10 timeline pages of 200 entries per thread. When older entries are cut, the file header says so.
- qmd index name and collection name: `paseo-threads`. Run `qmd --index paseo-threads update` at most once per 30 seconds after exports. Only one update runs at a time.
- Version 1 uses BM25 `qmd search` first, with `qmd query` as a slow fallback. Vector embedding (`qmd embed`) is out of scope.
- Test on 2026-09-23 with 39 real threads: `qmd search` with 2 or 3 keywords put the right thread first in 12 of 14 queries, at 0.2 seconds each. Full-sentence queries got 8 of 14, because every word must match. `qmd query` got 10 of 14 at 2 to 30 seconds each. Synonyms, such as badge for pill, miss in both.
- If qmd is missing, export still works. The plugin logs one line and skips indexing.
- Agents learn about the search from a skill, like [~/.claude/skills/kb-search](~/.claude/skills/kb-search). The plugin does not change `systemPrompt`, because it is not verified whether Paseo appends or replaces it.
- No Jev and no network calls. Everything stays on this machine.

## Open Decisions

- None.

## References

- [plugins/thread-context-attach/server/threads.ts](plugins/thread-context-attach/server/threads.ts) for timeline paging.
- [plugins/thread-janitor/index.server.ts](plugins/thread-janitor/index.server.ts) for lifecycle hooks and a timer.
- [plugins/next-prompt-actions/server/jev.ts](plugins/next-prompt-actions/server/jev.ts) for a bounded child process with a timeout.
- `qmd --help`
