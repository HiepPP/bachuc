# TASK-003 GitHub PR and CI attachment in Thread branch

Group: B (shares index.server.ts, index.client.tsx, and README.md in plugins/thread-branch with TASK-002)
Class: code

## Brief

Goal: the composer attachment menu gets a GitHub PR source. The user can attach an open PR, or the failed CI log of a PR, to the next message without copy and paste.

Change: PR context is copied by hand -> composer + -> GitHub PR attaches the PR summary or its failed CI log.

How:

- Add attachment source `github-pr` with search RPC `thread-branch.search-prs`, input `{ query }`.
- Find repos: list non-archived Paseo workspaces, read the `origin` remote in each `cwd`, keep GitHub repos only, and remove duplicates. Cache the repo list for 5 minutes.
- For each repo, run `gh pr list --repo <owner/repo> --state open --limit 20 --json number,title,headRefName,author,url,updatedAt,statusCheckRollup`. Cache each list for 5 minutes. Run at most 3 `gh` calls at once, each with a 15-second timeout.
- Match `query` against repo, PR number, title, and branch. Return at most 20 items, newest first. An empty query returns the newest PRs.
- A PR item's `text` holds the repo, number, title, branch, author, URL, check summary, and the PR body cut to 4,000 characters.
- An attachment's `text` is built inside the search RPC, and there is no hook when the user picks an item. So when a PR has a failed check, start `gh run view <runId> --log-failed` in the background and cache the last 200 lines, at most 8,000 characters. A later search shows an extra item "CI failure: #<n> <check>" with that log.
- If `gh` is missing or not signed in, return no items and log one line. The pills must keep working.

Files:

- [plugins/thread-branch/server/pr-search.ts](plugins/thread-branch/server/pr-search.ts) (new: repo discovery, PR lists, CI log cache, and item text)
- [plugins/thread-branch/shared/pr-search.ts](plugins/thread-branch/shared/pr-search.ts) (new: RPC and attachment source)
- [plugins/thread-branch/index.server.ts](plugins/thread-branch/index.server.ts) (serve the RPC)
- [plugins/thread-branch/index.client.tsx](plugins/thread-branch/index.client.tsx) (register the attachment source)
- [plugins/thread-branch/tests/pr-search.test.ts](plugins/thread-branch/tests/pr-search.test.ts) (new tests with a fake runner)
- [plugins/thread-branch/README.md](plugins/thread-branch/README.md) (new Attach a PR section)

Expected result:

- composer + -> GitHub PR lists open PRs from the repos of Paseo workspaces.
- Picking a PR attaches its summary. After a failed CI run, a later search also offers its failed log.
- Without `gh`, the picker is empty and nothing else breaks.

## Verify

- `cd plugins/thread-branch && npm test` -> all pass. New tests use a fake runner and cover: GitHub and non-GitHub remotes, duplicate repos, query matching, the 20-item limit, body and log cuts, the 5-minute cache, the limit of 3 `gh` calls at once, a CI failure item after the background fetch, and `gh` missing.
- `cd plugins/thread-branch && npm run typecheck` -> exit 0.
- `cd plugins/thread-branch && S=lint; npm run $S` -> exit 0.
- Manual, needs the Paseo app and `gh auth status` signed in: after `paseo plugin reload thread-branch`, composer + -> GitHub PR lists open PRs. Picking one adds its summary to the draft. Nothing is sent until the user sends it.
