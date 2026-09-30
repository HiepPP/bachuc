# TASK-002 Turn diff summary in Thread branch

Group: B (shares index.server.ts, index.client.tsx, and README.md in plugins/thread-branch with TASK-003)
Class: code

## Brief

Goal: after each agent turn, the thread shows a small row with the files the turn changed, the added and deleted lines, and any commits made. The user sees what the agent touched without opening a terminal.

Change: no per-turn change view -> one "N files changed +A -D" row after each turn that changed files.

How:

- On `agent.turn_started`, take a snapshot in `agent.cwd`: `git rev-parse HEAD`, `git diff --numstat HEAD`, and `git ls-files --others --exclude-standard`. Keep it in memory by agent id. Skip non-git folders.
- On `agent.turn_ended`, take the same snapshot again, outside the hook.
- Changed files are files whose numstat line is new, gone, or different, plus new untracked files. If `HEAD` moved, add `git diff --numstat <startSha> HEAD` and list `git log --oneline <startSha>..HEAD`, at most 10 commits.
- Count binary files (numstat `-`) as changed, with no line counts.
- If no file changed and no commit was made, add no row. If there is no start snapshot, for example after a reload mid-turn, add no row.
- Track running turns per `cwd`. If another agent ran in the same `cwd` during the turn, mark the row "may include changes from another agent".
- Append a plugin row with id `turn-diff:<turnId>`. Add a timeline renderer. It shows the header, the commits, and at most 20 files with `+A -D`, then "and N more".
- Use the bounded `git` runner that [plugins/thread-branch/server/git.ts](plugins/thread-branch/server/git.ts) already uses.

Files:

- [plugins/thread-branch/server/turn-diff.ts](plugins/thread-branch/server/turn-diff.ts) (new: snapshots and diff)
- [plugins/thread-branch/shared/turn-diff.ts](plugins/thread-branch/shared/turn-diff.ts) (new: row kind, version, and schema)
- [plugins/thread-branch/client/turn-diff-card.tsx](plugins/thread-branch/client/turn-diff-card.tsx) (new: timeline renderer)
- [plugins/thread-branch/index.server.ts](plugins/thread-branch/index.server.ts) (turn hooks)
- [plugins/thread-branch/index.client.tsx](plugins/thread-branch/index.client.tsx) (register the renderer)
- [plugins/thread-branch/tests/turn-diff.test.ts](plugins/thread-branch/tests/turn-diff.test.ts) (new tests)
- [plugins/thread-branch/README.md](plugins/thread-branch/README.md) (new Turn diff section)

Expected result:

- A turn that edits two files shows one row with those two files and their line counts.
- A turn that commits shows the commit subjects.
- A turn with no changes, or in a non-git folder, shows no row.
- The pills work as before.

## Verify

- `cd plugins/thread-branch && npm test` -> all pass. New tests use a temp git repo and cover: edited file, new untracked file, deleted file, binary file, a commit during the turn, no change, no start snapshot, non-git folder, a shared `cwd` flag, and the 20-file limit.
- `cd plugins/thread-branch && npm run typecheck` -> exit 0.
- `cd plugins/thread-branch && S=lint; npm run $S` -> exit 0.
- Manual, needs the Paseo app: after `paseo plugin reload thread-branch`, ask an agent to edit one file. The thread shows one row with that file and its line counts. A question-only turn shows no row.
