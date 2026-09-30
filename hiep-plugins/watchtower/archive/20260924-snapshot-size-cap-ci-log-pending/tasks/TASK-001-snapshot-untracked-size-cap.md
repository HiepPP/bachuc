# TASK-001 Skip large untracked files in turn snapshots

Group: A (shares plugins/thread-branch/README.md with TASK-002)
Class: risky

## Brief

Goal: a turn snapshot no longer copies large untracked files into `.git`. Refs keep snapshots for 30 days, so today one big untracked artifact costs disk space for a month.

Change: `git add -A` of the whole work tree -> tracked changes plus only untracked files up to 1 MiB.

How:

- In `snapshotTree`, replace `git add -A -- .` with two steps, both on the temp index:
  - `git add -u -- .` stages changes to tracked files, including deletions.
  - List untracked files with `git ls-files -z --others --exclude-standard`, and keep those that are regular files of at most 1 MiB. Write their paths NUL-separated to a file in the same temp dir. Then run `git add --pathspec-from-file=<file> --pathspec-file-nul`. Skip this step when the list is empty.
- Keep the `git rev-parse --git-path index` check before any `git add`. Both adds must pass the temp-index `env`.
- Return the skipped paths with the tree, so the card can show them.
- At turn end, list a skipped path that is new in the turn on the card as `{ added: null, deleted: null, large: true }`. A path is new when it is in the end snapshot's skipped list and not in the start snapshot's untracked or skipped list. A restored start has no untracked list, so after a plugin reload mid-turn, skip this extra listing.
- Show `large, not saved` instead of `binary` on the card, and make that row not clickable.
- Add a README line: untracked files over 1 MiB are left out of snapshots and shown as large, not saved.

Files:

- [plugins/thread-branch/server/turn-diff.ts](plugins/thread-branch/server/turn-diff.ts) (snapshot adds, skipped list, card entries)
- [plugins/thread-branch/shared/turn-diff.ts](plugins/thread-branch/shared/turn-diff.ts) (optional `large` flag on a file entry)
- [plugins/thread-branch/client/turn-diff-card.tsx](plugins/thread-branch/client/turn-diff-card.tsx) (large label, not clickable)
- [plugins/thread-branch/tests/turn-diff.test.ts](plugins/thread-branch/tests/turn-diff.test.ts) (new tests)
- [plugins/thread-branch/README.md](plugins/thread-branch/README.md) (one line)

Expected result:

- A new 2 MiB untracked file does not end up in the snapshot tree, and `git cat-file -e <tree>:<path>` fails for it.
- The card lists that file as large, not saved.
- A new 10 KiB untracked file and an edited tracked file of any size are still in the tree and diff as before.
- The user's real index is unchanged after a snapshot.

## Verify

- `cd plugins/thread-branch && npm test` -> all pass. New tests cover: a 2 MiB untracked file left out of the tree and listed as large; a small untracked file and a large tracked edit still in the tree; a deleted tracked file still diffs; a path with spaces and a newline in its name; and `git diff --cached --name-only` empty after a snapshot.
- `cd plugins/thread-branch && npm run typecheck` -> exit 0.
- `cd plugins/thread-branch && S=lint; npm run $S` -> exit 0.
- The existing test `never stages files in the real index, even with a runner that drops env` -> still passes.
