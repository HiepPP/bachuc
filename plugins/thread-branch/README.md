# Thread branch

Shows the current git branch of each conversation's working directory as a composer pill,
beside Tasks and Subagents. When the branch has a pull request, a second pill `#42` appears;
one click opens it in the system browser. A third pill named after the `origin` repository opens
its web page (GitHub, GitLab, or any http/ssh remote). A sync pill `↑2 | ↓1` shows commits waiting
to push and to pull against the upstream. Its icon is a green check with **Synced** when there is
nothing to move, an accent upload cloud when only pushing is left, and an amber download cloud when
there is something to pull. It hides when the branch has no upstream, and a click runs `git fetch`
then recounts. Between clicks the pull count reflects the last fetch;
the 15-second poll never fetches.

A changes pill `3 files +14 -3` appears only when the work tree has uncommitted changes. It counts
tracked files from `git status` (untracked files are not counted, matching the dirty check) and
staged plus unstaged lines against `HEAD`. A click refreshes it.

A links pill `3 links` lists the PRs (`#42`), commits (7–40 hex chars), and branches named in the
agent's last reply and opens each on the `origin` remote (GitHub or GitLab routes). Branches count
only as inline code after the word "branch", with a prefix like `feat/`, or as `main`/`master`/
`develop`. The reply is re-read when a turn ends. Refs always resolve against this workspace's
`origin`, so a ref from another repository opens the wrong page.

The branch pill menu offers **Copy branch name**, **Fetch** (runs `git fetch`, shown when the branch
has an upstream), and **Refresh**.
A detached HEAD shows `@<short sha>`. Non-git directories show no pill.

## Turn diff

After each agent turn that changed files or made commits, the thread gets one row:
`2 files changed +14 -3`, then the commits made during the turn (at most 10), then up to 20
files with their `+added -deleted` lines and `and N more`. Binary files show `binary`. A turn
with no change, in a non-git folder, or without a start snapshot adds no row. The start snapshot
is also saved to `$PASEO_HOME/plugin-data/thread-branch/turn-starts/<agent>.json` (about 250
bytes, deleted at turn end, dropped after a day), so a plugin reload mid-turn still gives the card.

- The card shows only what changed during that turn, never the whole diff against `HEAD`. At turn
  start and end the plugin writes the whole work tree, untracked files included, as a git tree
  object through a copy of the index, so your staging area is untouched. The card lists the
  files whose content differs between the two trees, so edits inside files that were already
  dirty, and files that were untracked before the turn, still count. Files edited before the turn
  and not touched during it are left out. The objects stay loose until `git gc` prunes them.
- Untracked files over 1 MiB stay out of snapshots, so they cost no space under `.git`. A new one
  still shows on the card as **large, not saved**, with no diff. Tracked files are always kept.
- If a snapshot times out, the card falls back to comparing `git diff --numstat` against the start
  commit. That fallback can miss an edit that keeps a dirty file's line counts equal.
- When another agent ran a turn in the same `cwd` at the same time, the row says
  **May include changes from another agent**, because git cannot tell whose edit is whose.
- Click a file to open the **Turn diff** panel beside the agent. It shows only that file, and only
  what changed during that turn, from the same two snapshots. Rows from before this version, or
  where a snapshot timed out, are not clickable. **All turns** in the panel, or Command Center →
  **Open turn changes**, lists every recorded turn of the agent, newest first.
- The daemon keeps timeline cards only in memory, so a daemon restart, or a reload of the agent
  session (Refresh, `paseo agent reload`), removes them. Each card is
  also written to `$PASEO_HOME/plugin-data/thread-branch/turn-diffs.jsonl` (mode 600), and the
  panel's turn list reads that file, so turn changes stay viewable after a restart. Nothing is
  cached in memory. Entries older than 30 days, or past 2,000, are dropped.
- Two refs per turn, `refs/thread-branch/turns/<key>/from` and `.../to`, keep the snapshot trees
  safe from `git gc`. They are deleted when the turn leaves the file. They are not branches, so
  `git branch` and pushes of branches do not show them; `git push --mirror` would copy them.
  No worktree is created: snapshots use a temporary index file only.
- Long lines wrap by default and keep their indentation. The **Wrap** switch in the panel header
  turns wrapping off for sideways scrolling; the choice holds until the app reloads.
- Images (`png`, `jpg`, `gif`, `webp`, `bmp`, `ico`, `avif`, `svg`) show a Before and After
  preview read from the same snapshots. An image over 3 MB is not previewed.

## Attach a PR

Composer **+** → **GitHub PR** lists open pull requests from the GitHub repositories of all
non-archived Paseo workspaces, newest first, at most 20. Search matches the repository, `#number`,
title, and branch. The picker cannot tell which thread it belongs to, so it always searches every
workspace. Picking a PR adds its repository, number, title, branch, author, URL, check summary, and
the first 4,000 characters of its description to the draft. Nothing is sent until you send it.

- Repositories come from each workspace's `origin` remote; non-GitHub remotes are skipped.
- `gh pr list` runs per repository, at most 3 `gh` calls at once, 15 seconds each. The repository
  list and each PR list are cached for 5 minutes, so a new PR can take up to 5 minutes to appear.
- When a PR has a failed GitHub Actions check, the search starts `gh run view <run> --log-failed`
  in the background. A later search then also offers **CI failure: #42 build** with the last 200
  lines of that log, at most 8,000 characters. The first search after a failure never has it yet,
  because an attachment's text is built during the search and there is no hook when you pick one.
- A workflow that is still running has no log yet. It is not logged as a failure, and its log is
  fetched again after about a minute.
- Without `gh`, or when it is not signed in, the picker is empty and one line is logged. The pills
  and turn diff keep working.

## How it works

- `index.server.ts` runs `git` with fixed arguments in the agent's `cwd` and `gh pr view <branch>`
  for the pull request. PR lookups are cached for 5 minutes per branch; Refresh bypasses the cache.
- `index.client.tsx` lists non-archived agents, follows live agent updates, probes each distinct
  `cwd` every 15 seconds, and updates pills only when the branch state changed so an open menu
  stays open.
- Opening a PR uses the host's external opener when available and React Native `Linking` otherwise.

## Requirements

- `git` on the daemon `PATH`.
- `gh` on the daemon `PATH` and authenticated (`gh auth status`) for PR numbers. Without it the pill
  still shows the branch.

## Install

```bash
cd plugins/thread-branch
npm install
npm run typecheck
npm test
paseo plugin install "$PWD"
paseo plugin ls
```

Reload after source edits with `paseo plugin reload thread-branch`.
