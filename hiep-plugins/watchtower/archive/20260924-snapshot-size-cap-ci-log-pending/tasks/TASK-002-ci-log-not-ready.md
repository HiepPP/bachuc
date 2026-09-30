# TASK-002 Keep CI logs of running workflows as not ready

Group: A (shares plugins/thread-branch/README.md with TASK-001)
Class: code

## Brief

Goal: when a workflow run is still in progress, the PR picker stops logging "gh run view failed" every 5 minutes. It fetches the log again soon, and offers the CI failure item once the log exists.

Change: an empty log from a running workflow counts as failed for 5 minutes -> it counts as not ready and is retried after 60 seconds, with no log line.

How:

- In `fetchLog` in [plugins/thread-branch/server/pr-search.ts](plugins/thread-branch/server/pr-search.ts), treat a result as not ready when it has no log text and stdout or stderr matches `/still in progress/i`.
- Cache a not-ready result with a 60-second life, then fetch again on a later search. Keep the 5-minute life for real logs and real failures.
- Do not log a not-ready result. Keep the one log line for real failures.
- Add a README line: logs of running workflows are fetched again after about a minute.

Files:

- [plugins/thread-branch/server/pr-search.ts](plugins/thread-branch/server/pr-search.ts) (not-ready state and shorter retry)
- [plugins/thread-branch/tests/pr-search.test.ts](plugins/thread-branch/tests/pr-search.test.ts) (new tests with the fake runner)
- [plugins/thread-branch/README.md](plugins/thread-branch/README.md) (one line)

Expected result:

- A running workflow run adds no log line and no CI failure item. A search 61 seconds later runs `gh run view` again.
- Once the log exists, the next search offers the CI failure item.
- A real failure (non-zero exit, or HTTP 404) is still logged once and cached for 5 minutes.

## Verify

- `cd plugins/thread-branch && npm test` -> all pass. New tests with the fake runner and fake clock cover: not ready on stdout, not ready on stderr, a retry after 60 seconds but not before, the item appearing once the log exists, and a real failure still logged and cached for 5 minutes.
- `cd plugins/thread-branch && npm run typecheck` -> exit 0.
- `cd plugins/thread-branch && S=lint; npm run $S` -> exit 0.
- Manual, needs a PR whose workflow is running: `paseo plugin logs thread-branch` after a picker search -> no `gh run view ... failed` line for that run.
