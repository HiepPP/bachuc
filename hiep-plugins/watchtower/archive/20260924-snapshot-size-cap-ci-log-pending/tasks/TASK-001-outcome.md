# TASK-001 Outcome

## Outcome

Status: DONE

Changed:
- [plugins/thread-branch/server/turn-diff.ts](plugins/thread-branch/server/turn-diff.ts): `snapshotTree` now runs `git add -u -- .`, then adds only untracked files of at most `MAX_SNAPSHOT_UNTRACKED` (1 MiB) through `--pathspec-from-file` and `--pathspec-file-nul` with `GIT_LITERAL_PATHSPECS=1`. All of this runs on the temp index. The function returns `{ tree, skipped }`. `ended` lists a skipped file that is new in the turn as `large: true`, except for a start restored from disk.
- [plugins/thread-branch/shared/turn-diff.ts](plugins/thread-branch/shared/turn-diff.ts): an optional `large` flag on a file entry.
- [plugins/thread-branch/client/turn-diff-card.tsx](plugins/thread-branch/client/turn-diff-card.tsx): a large file shows `large, not saved` and is not clickable.
- [plugins/thread-branch/tests/turn-diff.test.ts](plugins/thread-branch/tests/turn-diff.test.ts): 2 new tests.
- [plugins/thread-branch/README.md](plugins/thread-branch/README.md): one bullet.

Contract:
- Tracked files are always snapshotted. Untracked files over 1 MiB never enter `.git` through snapshots.
- The real-index guard (`git rev-parse --git-path index` must return the temp copy) still runs before any `git add`.
- Not shown: a large untracked file that existed before the turn and changed during it.

Verified:
- `npm test` -> 54/54 pass. The test `keeps large untracked files out of the snapshot and lists them as large` fails on the old code, when stashed. It covers a 2 MiB untracked file left out of the tree and listed as large, a small file with a space, a file with a newline and `*` in its name, a 2.6 MB tracked edit still in the tree, a deletion made before the turn, and an empty `git diff --cached`.
- The test `never stages files in the real index, even with a runner that drops env` -> still passes.
- `npm run typecheck` -> exit 0. `S=lint; npm run $S` -> 0 warnings, 0 errors. oxfmt -> clean.
