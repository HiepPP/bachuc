# TASK-003 Outcome

## Outcome

Status: DONE (in-app check PENDING-USER)

## Changed

- plugins/thread-branch/server/pr-search.ts (new): workspace dir listing, GitHub repo discovery, PR list cache, gh limiter, CI log cache, item text.
- plugins/thread-branch/shared/pr-search.ts (new): RPC `thread-branch.search-prs` `{ query }` and attachment source `github-pr`.
- plugins/thread-branch/index.server.ts: serves the RPC with `listWorkspaceDirs(paseo)`.
- plugins/thread-branch/index.client.tsx: registers the attachment source.
- plugins/thread-branch/tests/pr-search.test.ts (new, fake runner).
- plugins/thread-branch/README.md: Attach a PR section.
- Uses the `run` export from plugins/thread-branch/server/git.ts (see TASK-002 outcome), with a 32 MiB buffer for `--log-failed` so the tail survives.
- plugins/thread-branch/server/git.ts (review fix): `run` now returns `code: null` when the process is killed or signaled (timeout, maxBuffer), instead of mapping Node's null code to 0. Before, a timed-out `git diff --numstat` / `gh pr list` / `gh run view` looked like success with partial output. plugins/thread-branch/tests/git.test.ts: one test that a timed-out command is not code 0.

## Contract

- Repos: `paseo.workspaces.list` (first 200, `archivingAt` skipped), `git remote get-url origin` per dir, github.com only, case-insensitive dedupe; cached 5 min.
- `gh pr list --repo <o/r> --state open --limit 20 --json number,title,body,headRefName,author,url,updatedAt,statusCheckRollup`. `body` added to the spec's field list because the item text needs the PR body. Cached 5 min per repo (failures cached too); max 3 gh calls at once; 15 s timeout each.
- Matching: every query word (with `#` stripped) must appear in `repo number title branch`; newest `updatedAt` first; max 20 items.
- PR item text: repo, number, title, branch, author, URL, check summary, body cut to 4,000 chars.
- CI: failed GitHub Actions checks grouped by run id; for matched PRs `gh run view <runId> --repo <o/r> --log-failed` starts in the background; last 200 lines, max 8,000 chars; a later search adds `CI failure: #<n> <checks>` right after the PR. A failed log fetch is retried after 5 min.
- gh missing / not signed in: empty items, one log line per outage (reset on the next success). The RPC never throws; pills and turn diff are separate code paths.

## Verified

Group-level rerun after the review fix (shared run for TASK-002 and TASK-003), in plugins/thread-branch:
- `npm run typecheck` -> exit 0.
- `npm test` -> 34 tests, 34 pass, 0 fail (includes the new timeout test; pr-search.test.ts: GitHub and non-GitHub remotes, duplicate repos, query matching, 20-item limit, body and log cuts, 5-minute cache, 3 concurrent gh calls, CI failure item after background fetch, gh missing).
- `S=lint; npm run $S` -> 0 warnings, 0 errors.
- `npx oxfmt --check` on all changed/new files (incl. server/git.ts, tests/git.test.ts) -> clean.
- Manual in-app check (`gh auth status` signed in, reload, composer + -> GitHub PR lists PRs, pick adds summary): PENDING-USER. Plugin not reloaded, per run rules.

In-app evidence 2026-09-24:
- The picker search ran at 10:38 local. It found PRs in `Expensify/App` and `getpaseo/paseo` and started background CI log fetches.
- All 7 `gh run view --log-failed` calls failed outside the plugin. `getpaseo/paseo` run 35951176054 is still in progress, so gh prints "logs will be available when it is complete". `Expensify/App` returns `HTTP 404` on `actions/workflows/327409428`. So no "CI failure" item appeared.
- Whether PR items show in the picker is not seen by the main session.
