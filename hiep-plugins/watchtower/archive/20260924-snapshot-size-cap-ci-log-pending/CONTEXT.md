# Plan Context

## Shared Context

- Both TASKs change only [plugins/thread-branch](plugins/thread-branch).
- Turn snapshots live in [plugins/thread-branch/server/turn-diff.ts](plugins/thread-branch/server/turn-diff.ts). `snapshotTree` copies the real index into a temp dir, checks `git rev-parse --git-path index` returns the copy, then runs `git add -A -- .` and `git write-tree` with `GIT_INDEX_FILE` set.
- Refs `refs/thread-branch/turns/<key>/from|to` keep each snapshot tree for 30 days. Any blob in a snapshot is kept that long.
- `takeSnapshot` already lists untracked files with `git ls-files -z --others --exclude-standard`. `countNewFile` already treats files over 1 MiB (`MAX_READ`) as having no line counts.
- The PR picker lives in [plugins/thread-branch/server/pr-search.ts](plugins/thread-branch/server/pr-search.ts). `fetchLog` runs `gh run view <run> --repo <repo> --log-failed` in the background and caches the tail.
- Seen on 2026-09-24: for a run that is still running, `gh run view --log-failed` exits 0 with no log and prints `run <id> is still in progress; logs will be available when it is complete`. The stream (stdout or stderr) was not recorded.
- Run lint as `S=lint; npm run $S`. Reload with `paseo plugin reload thread-branch`.
- Installed git is 2.50.1. It supports `git add --pathspec-from-file=<file> --pathspec-file-nul`.

## Decisions

- Per-file cap for untracked files in a snapshot: 1 MiB, the same as `MAX_READ`. Tracked files are always snapshotted, because git already stores them.
- A skipped large untracked file still appears on the card when it is new in the turn. It shows as "large, not saved" with no line counts and no diff.
- A CI log that is not ready is not a failure. It is not logged, and it is retried after 60 seconds instead of 5 minutes.
- The real-index safety check in `snapshotTree` stays. Every `git add` in the snapshot must use the temp index.

## Open Decisions

- None.

## References

- [plugins/thread-branch/server/turn-diff.ts](plugins/thread-branch/server/turn-diff.ts)
- [plugins/thread-branch/server/pr-search.ts](plugins/thread-branch/server/pr-search.ts)
- [plugins/thread-branch/tests/turn-diff.test.ts](plugins/thread-branch/tests/turn-diff.test.ts)
- [plugins/thread-branch/tests/pr-search.test.ts](plugins/thread-branch/tests/pr-search.test.ts)
