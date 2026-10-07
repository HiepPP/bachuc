# TASK-005 PR watch plugin

Group: C (standalone: new folder `hiep-plugins/plugins/pr-watch/`)
Class: code

## Brief

Goal: An agent calls `watch_pull_request`. The plugin checks the PR of the agent's branch every 2 minutes with `gh`. It wakes that agent when checks fail, all required checks pass, someone else comments or reviews, or a merge conflict appears.

Change: no PR follow-up after an agent opens a PR -> the agent gets a wake prompt for each new PR event.

Design: T3 Code `apps/server/src/orchestration-v2/PullRequestWatchReactor.ts:43-56,213-231` and `apps/server/src/mcp/toolkits/pullRequests/tools.ts:300`.

Boundaries:

- Plugin only. Use `server.registerTool` ([packages/plugin/src/server/contracts.ts](packages/plugin/src/server/contracts.ts), lines 41-71), `paseo.agents.ref(id).send()`, `setInterval` with cleanup, and files in `$PASEO_HOME/plugin-data/pr-watch/`.
- Copy patterns from [hiep-plugins/plugins/board/index.server.ts](hiep-plugins/plugins/board/index.server.ts) (tool registration) and [hiep-plugins/plugins/thread-branch/server/pr-search.ts](hiep-plugins/plugins/thread-branch/server/pr-search.ts) (`gh` calls and `statusCheckRollup`).
- Q-005 answered (default): a trigger during a running turn waits for `agent.turn_ended`. It does not steer.
- Q-006 answered (default): GitHub only, through `gh`.
- Ignore comments and reviews from the `gh` viewer login and from bots.
- Stop a watch after 8 failed reads in a row, when the PR is merged or closed, or on `agent.archived`.
- Native OpenCode and OMP agents cannot see plugin tools ([docs/plugins.md](docs/plugins.md), lines 342-344). Accept this.
- After a plugin reload or daemon restart, saved watches resume on the first hook or tool call that supplies `paseo`. Add no host change.

How:

- Scaffold the plugin from the thread-branch layout and scripts.
- Write pure modules: `server/diff.ts` (compare two PR snapshots and return triggers), `server/store.ts` (watch records), and `server/gh.ts` (one `gh pr view --json state,mergeable,statusCheckRollup,reviews,comments` call).
- In `index.server.ts`, register `watch_pull_request`, `unwatch_pull_request`, and `list_pull_request_watches`. Run the poll loop and the hooks.
- Write tests with `node:assert` for diff, store, and poll rate.
- Write a README with the tools, triggers, limits, and host facts with the Paseo commit SHA.
- Install and reload on live only.

Files:

- `hiep-plugins/plugins/pr-watch/paseo-plugin.json`, `package.json`, `package-lock.json`, `tsconfig.json` (new plugin)
- `hiep-plugins/plugins/pr-watch/index.server.ts` (tools, poll loop, hooks)
- `hiep-plugins/plugins/pr-watch/server/diff.ts`, `server/store.ts`, `server/gh.ts` (pure modules)
- `hiep-plugins/plugins/pr-watch/tests/*.test.ts` (tests)
- `hiep-plugins/plugins/pr-watch/README.md`

Expected result:

- `watch_pull_request` on a branch with no PR -> a clear error.
- Checks go from pending to failing -> one wake prompt that names the failed checks.
- All required checks pass -> one wake prompt.
- A new comment from another login -> a wake prompt with an excerpt. Own and bot comments -> no prompt.
- `mergeable` becomes `CONFLICTING` -> a wake prompt.
- The same state on the next poll -> no prompt.
- 8 failed reads in a row -> the watch stops with one notice.
- PR merged or closed -> the watch stops.

Anti-goal: `gh` calls per watch stay at 1 or fewer per 2-minute interval; tripwire; read from a fake-clock test that counts `gh` calls, after the poll loop is written and at completion.

## Verify

- In `hiep-plugins/plugins/pr-watch`: `npm test` -> pass, including the call-count test (anti-goal read).
- In `hiep-plugins/plugins/pr-watch`: `npm run typecheck` -> exit 0.
- In `hiep-plugins/plugins/pr-watch`: `npm run lint` -> exit 0.
- Needs the live daemon: `npm run cli -- plugin install "$PWD/hiep-plugins/plugins/pr-watch" --id pr-watch`, then `npm run cli -- plugin ls` -> `pr-watch` is listed and enabled.
- Manual live check (needs a human and a real PR): a live agent calls `watch_pull_request` on a branch with a failing check -> it gets one wake prompt within 2 minutes.
