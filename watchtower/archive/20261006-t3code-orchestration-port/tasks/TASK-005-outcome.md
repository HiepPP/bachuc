# TASK-005 Outcome

## Outcome

Status: DONE

Changed:

- New plugin `hiep-plugins/plugins/pr-watch/`: [index.server.ts](hiep-plugins/plugins/pr-watch/index.server.ts) registers `watch_pull_request`, `unwatch_pull_request`, and `list_pull_request_watches`, runs a 30-second tick, and uses the `agent.turn_started`, `agent.turn_ended`, and `agent.archived` hooks.
- Pure modules: [server/diff.ts](hiep-plugins/plugins/pr-watch/server/diff.ts) (triggers and the wake prompt), [server/gh.ts](hiep-plugins/plugins/pr-watch/server/gh.ts) (one `gh pr view --json` call and its parser), [server/store.ts](hiep-plugins/plugins/pr-watch/server/store.ts) (watches in `plugin-data/pr-watch/watches.json`), and [server/watcher.ts](hiep-plugins/plugins/pr-watch/server/watcher.ts) (poll schedule, held wakes, failed-read limit).
- Tests in `hiep-plugins/plugins/pr-watch/tests/`: diff, gh parsing, and watcher.
- [hiep-plugins/plugins/pr-watch/README.md](hiep-plugins/plugins/pr-watch/README.md) and a catalog row in [hiep-plugins/README.md](hiep-plugins/README.md).

Contract:

- Q-005 answered: a trigger during a running turn waits for the turn to end. The plugin refreshes the agent before each wake and holds the prompt while it runs.
- Q-006 answered: GitHub only, through `gh`.
- The first read of a watch is a baseline and wakes nobody. Later reads wake the agent on a newly failing check, all checks passing, a new comment or review from another login (not the `gh` login and not `*[bot]`), a new merge conflict, or a merged or closed PR, which also ends the watch.
- A watch stops after 8 failed reads in a row with one prompt, and on `agent.archived`.
- After a plugin reload or a daemon restart, polling resumes on the first hook or tool call, because the timer has no host context.
- Like Board, the plugin links the SDK from this checkout with `file:` paths, maps `zod` to the root copy in `tsconfig.json`, and lints with `--disable-nested-config`.

Verified:

- In `hiep-plugins/plugins/pr-watch`: `npm test` -> 13 passed. `npm run typecheck` -> exit 0. `npm run lint` -> 0 warnings, 0 errors. `npm run format` -> done; root `npm run format:check:files` on the plugin files -> exit 0.
- Live install (`npm run cli -- plugin install ...`, then `plugin ls`) -> UNVERIFIED (autonomous run). The live daemon on port 6768 was stopped.
- Manual check with a real PR -> UNVERIFIED (autonomous run).

Anti-goal:

- After the poll loop: test "reads each watch at most once per poll interval" passed (0 reads before the interval, 1 per interval, 1 for two concurrent ticks); 2026-10-07 00:29.
- Final: the same test passed; 2026-10-07 00:31.
- Result: PASS against at most 1 `gh` read per watch per 2-minute interval.

Lessons:

- A plugin that uses fork-only SDK APIs needs the `file:` links to `packages/*`. The published `@getpaseo/plugin` 0.9.0-beta.2 has no `registerTool`.
- With `file:` links, map `zod` to `../../../node_modules/zod` in `tsconfig.json`. Two zod copies made `tsc` run out of memory.
- Plugin `npm run lint` needs `--disable-nested-config`; the root `.oxlintrc.json` sets a root-only option. `thread-branch` lint fails today for that reason.
