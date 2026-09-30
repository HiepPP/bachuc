# TASK-002 Outcome

## Outcome

Status: DONE (in-app check PENDING-USER)

## Changed

- plugins/thread-branch/server/turn-diff.ts (new): start/end snapshots, diff, commits, shared-cwd flag.
- plugins/thread-branch/shared/turn-diff.ts (new): row kind `thread-branch-turn-diff`, version 1, schema, header text.
- plugins/thread-branch/client/turn-diff-card.tsx (new): timeline renderer.
- plugins/thread-branch/index.server.ts: `agent.turn_started` / `agent.turn_ended` hooks; row appended outside the hook.
- plugins/thread-branch/index.client.tsx: registers the renderer; returns one cleanup for pills, renderer, attachment source.
- plugins/thread-branch/tests/turn-diff.test.ts (new).
- plugins/thread-branch/README.md: Turn diff section.
- plugins/thread-branch/server/git.ts (dep fix, not in the file list): exported the existing bounded `run` and `Exec`, added an optional `maxBuffer` parameter (default unchanged, 1 MiB). Only caller was `createBranchReader`; LOW impact (GitNexus has no index for this repo, checked with rg).
- plugins/thread-branch/server/git.ts (review fix): `run` now returns `code: null` when the process is killed or signaled (timeout, maxBuffer), instead of mapping Node's null code to 0. Before, a timed-out `git diff --numstat` / `gh pr list` / `gh run view` looked like success with partial output. plugins/thread-branch/tests/git.test.ts: one test that a timed-out command is not code 0.

## Contract

- Row id `turn-diff:<turnId>`; data `{ fileCount, added, deleted, files[<=20]{path, added|null, deleted|null}, commits[<=10]{sha, subject}, shared }`.
- Header `N files changed +A -D` (plus `· N commits`); then commits, up to 20 files, `and N more`; binary shows `binary`; shared shows "May include changes from another agent".
- No row when nothing changed and no commit, no start snapshot (or turnId mismatch), non-git folder, or unborn HEAD.
- Deviation from the How text, same intent: instead of adding `git diff --numstat <startSha> HEAD` to the HEAD-relative numstat, the end snapshot runs `git diff --numstat <startSha>` (work tree vs start commit). This counts committed and uncommitted changes once and avoids double counting a dirty file that was then committed. Commits still come from `git log <startSha>..HEAD`, max 10.
- Dirty-at-start files count only the change in their numstat since the start. Untracked files are read repo-wide (snapshot runs at the repo top level); new untracked files count lines (over 1 MiB or NUL byte: no counts).

## Verified

Group-level rerun after the review fix (shared run for TASK-002 and TASK-003), in plugins/thread-branch:
- `npm run typecheck` -> exit 0.
- `npm test` -> 34 tests, 34 pass, 0 fail (includes the new timeout test; turn-diff.test.ts: edited, new untracked, deleted, binary, dirty-at-start, commit during turn, no change, no start snapshot, non-git folder, shared cwd, 20-file limit).
- `S=lint; npm run $S` -> 0 warnings, 0 errors.
- `npx oxfmt --check` on all changed/new files (incl. server/git.ts, tests/git.test.ts) -> clean.
- Manual in-app check (reload, edit one file -> one row; question-only turn -> no row): PENDING-USER. Plugin not reloaded, per run rules.

In-app evidence 2026-09-24:
- Test turns 12-14 of agent `9ad0b344` changed no files in `~/.claude` (file mtimes and recaps agree). No row is the expected result.
- The positive case (a turn that edits a file) is not checked in the app yet.

In-app positive check 2026-09-24:
- Probe agent `ac4b5b5a` (Haiku 4.5) created `watchtower/turn-diff-probe.txt` with 3 lines. The raw timeline, read with `@getpaseo/client` `timeline.refetch`, holds the row `turn-diff:foreground-turn-1` of kind `thread-branch-turn-diff`: `fileCount 1, added 3, deleted 0`, the one file, no commits.
- `shared: true` is right. The main session had a turn running in the same `cwd` when the probe started. Files that were already dirty before the turn were left out of the row.
- `paseo agent logs` and the MCP activity summary do not show plugin rows. Read the raw timeline to check them.
- Cleanup: probe file removed and probe agent archived.

Follow-up 2026-09-24: click a file to open its turn diff
- The user asked that a click on a file opens a diff of only that file. No plugin API or URL opens the built-in Changes view for one file: `?open=` takes only agent, terminal, draft, file, and setup. The user picked a plugin-owned panel that shows only the changes from that turn.
- New: `snapshotTree` writes the work tree, including untracked files, as a tree object using a copy of the index. The row gets `source { workspaceId, root, from, to }`. The RPC `thread-branch.file-diff` runs `git diff from to -- path`. The agent panel `turn-diff` shows it, and the card file rows are links.
- Checks: typecheck exit 0; tests 37/37 (3 new: a single-file diff that drops edits made before the turn, no source without a workspace, and `diffLines`); lint 0/0; oxfmt clean.
- Live: probe agent `81089549` created a 2-line file. The row has `source`. `git diff <from> <to> -- watchtower/turn-diff-probe.txt` shows exactly `+alpha +beta`. The user's staging area did not change.
- PENDING-USER: click the file in the probe thread and see the Turn diff panel.

Follow-up 2026-09-24: Paseo-style panel and image preview
- The panel now matches Paseo's diff view: a header with a status badge and `+A -D`, old and new line numbers, green and red row tints, a hunk bar, and sideways scroll. Colors and the gutter formula come from the app bundle. `parseUnifiedDiff` replaced `diffLines`.
- Image files get a Before and After preview. The RPC `thread-branch.file-image` reads each side with `git cat-file blob <tree>:<path>`, at most 3 MB per side, as a `data:` URI.
- Checks: typecheck exit 0; tests 39/39; lint 0/0; oxfmt clean; reload `running`.
- Live: for agent `a804dd0e`, the row for `docs/evidence/20260924-worklist-narrow-sheet-open/after-1664-sheet-open.png` gives `before null`, a 157,050-character `after` URI, and `tooLarge false`.
- PENDING-USER: see the image in the panel.

Follow-up 2026-09-24: card redesign
- Header: a FileDiff icon, the file count, and green `+A` and red `-D`. The commit count sits on the right. The shared-cwd warning and the commits sit in their own section, with icons.
- File rows: a file-type icon, a muted folder with a bold file name (the folder is cut from the left), right-aligned colored counts, 5 GitHub-style diffstat blocks, and a chevron. Rows highlight on hover. The file open in the panel is marked with an accent bar. No underline.
- More than 6 files start folded, with Show N more files and Show fewer files.
- Checks: typecheck exit 0, lint 0/0, tests 39/39, reload `running`. PENDING-USER: visual check.
- Follow-up: the user asked for the full list always. Folding and the Show more toggle were removed. The row still keeps at most 20 files (`TURN_DIFF_MAX_FILES`). Beyond that, "and N more files not listed" shows. Checks: typecheck, lint, and tests 39/39 pass, and reload is `running`.
- Follow-up: turn scope check (user request). The card now lists files from `git diff --numstat <startTree> <endTree>`. Before, it compared numstat against the start commit. That missed an edit inside an already-dirty file when the line counts stayed the same, and edits to files untracked before the turn. The probe showed only `c.txt`. Now it shows `a.txt +1 -1`, `c.txt +1 -0`, and `u.txt +1 -0`. It leaves out `keep.txt`, which was dirty before the turn and not touched during it. The old method stays as a fallback when a snapshot fails.
- Found and fixed: the copied index got a new mtime, so git's racy-clean check missed same-size rewrites in the same second. The flaky test `flags a turn that overlapped another agent` failed 1 run in 5. Fix: `utimes` copies the real index times to the copy. After the fix, 0 of 15 runs failed. Tests 40/40.

Follow-up 2026-09-24: persistent turn changes
- Finding: the daemon saves an agent's history only as the provider session file (for Claude, `persistence.sessionId` points to `~/.claude/projects/.../<id>.jsonl`). Plugin rows are neither there nor anywhere under `~/.paseo`. A daemon restart is not tested, because AGENTS.md forbids restarting it.
- Design: no re-append into the timeline, because restored cards would pile up at the bottom. Each card goes to `plugin-data/thread-branch/turn-diffs.jsonl` (mode 600, 30 days, 2,000 entries, no memory cache, pruned on the first add and then every 50 adds). The panel lists all turns from that file, with an All turns back button and Command Center -> Open turn changes. Refs `refs/thread-branch/turns/<key>/from|to` keep the trees until the entry expires. No worktree is created.
- Checks: tests 44/44 (4 new: a new journal instance still reads everything, the 30-day prune with a torn line, the 2,000 limit, and refs keep trees through `git gc --prune=now` and release them after drop). Typecheck, lint, and oxfmt pass. Reload is `running`.
- Live: probe agent `80845146` wrote journal line `1790225404764-foreground-turn-1` with its one file and source. Two tree refs exist. `git diff --stat <ref>/from <ref>/to` -> `turn-diff-probe.txt | 2 ++`. The probe file was removed and the agent archived. Its journal entry and refs stay until they expire.
- PENDING-USER: see All turns in the panel.
- Follow-up: diff lines wrap by default (web `whiteSpace: pre-wrap` and `overflowWrap: anywhere`, so indentation stays). The header has a **Wrap** switch that turns on sideways scroll, and the choice holds for the app session. Tests 44/44, lint and oxfmt pass, reload `running`. PENDING-USER: visual check.
- Follow-up: a plugin reload mid-turn dropped the in-memory start, so those turns had no card. The turn start (root, head, tree, turnId) is now saved to `plugin-data/thread-branch/turn-starts/<agent>.json`. The end reads it back when memory is empty, and the file is deleted at turn end. Records over a day old are ignored and pruned. A restored start without an end tree gives no card, because the numstat fallback would list changes from before the turn. Tests 49/49 (5 new), with 8 repeat runs and 0 failures. Live: probe `7fa3f39f` saved a 249 B start, `thread-branch` was reloaded mid-turn at 05:01:40, and the card `turn-diff:foreground-turn-1` still showed `turn-diff-probe.txt +2` and was clickable. The start file was then gone, and the journal holds the turn.
- Incident: wiring the start store in `index.server.ts` passed a git runner that dropped `env`. From about 05:00 to 05:03:10 UTC, the snapshot `git add -A` hit the real index. The only turn in that window was probe `7fa3f39f` in this repo, and it staged every change here. That was undone with `git reset -q`; nothing had been staged on purpose after 893fd4a. `calendar-ref` and `vorgange` ended their turns at 04:58, before the bug, and have 0 staged files. Fix: `index.server.ts` uses the exported `defaultGit`, which passes `env`. `snapshotTree` now checks that `git rev-parse --git-path index` returns the temp index before `git add -A`, and aborts if not. A new test proves a runner that drops `env` returns null and stages nothing. Tests 50/50. Live: probe `a1a2cedc` -> 0 staged files, its file still untracked, and the card is present and clickable.
- PR #3 review fix 2026-09-24: `readFileDiff` now returns the start of a diff that overflows the 1 MiB runner buffer, with `truncated: true`, instead of throwing. It was reproduced with a 1.38 MB lockfile diff. Row ids now carry the end time (`turn-diff:<ms>-<turnId>`), because turn ids restart at 1 on a session reload. The review first blamed the reused id for a missing card. A live recheck showed that a session reload drops all plugin rows by itself, so the card cannot be kept in the timeline. The All turns list keeps it, and the README now says so. Tests 52/52; the 2 new tests fail on the old code.
