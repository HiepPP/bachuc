# Learn 20260924-recap-log-turn-diff-pr-attach

## Summary

Discrepancy: 5 found. All three TASKs shipped in PR #3 (squash `cda3c46`). TASK-002 grew far past its brief through user requests. Two real bugs reached the PR review, and one bug staged files in a real index during development.

## Per TASK

- TASK-001: plan said to skip a repeated agent turn by `agentId` and `turnId` -> shipped a looser rule after the review. Mistake: the plan assumed a turn id is unique per agent. Paseo restarts turn ids at 1 when it reloads a session, so real recaps were dropped. Fix: skip only a repeat of the agent's latest entry with the same `turnId` and `raw`.
- TASK-002: plan said to take a numstat snapshot at turn start and end and add a row -> shipped work-tree tree snapshots, a clickable per-file diff panel in Paseo style, image previews, word wrap, a journal file with git refs for history, and a saved turn start that survives a plugin reload. Mistakes: (1) the numstat method missed edits inside already-dirty files and in files untracked before the turn; (2) a copied index got a fresh mtime and broke git's racy-clean check; (3) a runner that dropped `env` made `git add -A` stage everything in the real index for about 3 minutes; (4) a diff over the 1 MiB runner buffer could not open. Fix: diff two trees, copy the index mtime, check `git rev-parse --git-path index` before `git add -A`, and truncate the partial output.
- TASK-003: match. The reviewer found that the shared runner reported a timeout kill as exit 0, and that was fixed. `gh run view` on a run that is still in progress is logged as failed, which is a follow-up.

## Plan-Level

- Scope creep: about ten user follow-ups (panel, look, wrap, persistence, reload safety) were built on top of DONE TASK rows. They were recorded only in outcome sidecars, not as new TASK rows.
- The plan misdescribed reality twice. It assumed turn ids are unique, and it assumed timeline rows are stored on disk. Neither is true: a daemon restart or a session reload drops plugin rows.
- The PR review first blamed a reused row id for a missing card. A live recheck showed that the session reload drops rows by itself, so that finding was withdrawn.
- Still open: the user has not confirmed that the PR picker lists PRs in the app. The `paseo-thread-search` in-app check from the older plan is also still open.

## Lessons

- Before relying on host ids or storage, prove what they are. Read the installed Paseo bundle, for example `nextTurnOrdinal` in `ClaudeAgentSession`, and test a session reload.
- Any git command that uses `GIT_INDEX_FILE` must fail closed. Verify the temp index path first, and pass the env-carrying runner explicitly.
- When a user adds work to a DONE plan, add TASK rows so the Tracker still matches what shipped.
- A review reproduction must isolate the cause. Check the state right after each step (reload, then turn) before blaming code.
