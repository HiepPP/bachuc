# NEXT

## Current Active Plan

- Title: Add a recap log to Board, and turn diffs and PR/CI attachments to Thread branch
- Slug: 20260924-recap-log-turn-diff-pr-attach
- Status: ARCHIVED
- Updated: 2026-09-24

## Tracker

One row per TASK. Group ties together items that write the same files.

| Order | TASK | Group | Status | Spec | Deps | Context | Notes |
|-------|------|-------|--------|------|------|---------|-------|
| 1 | TASK-001 Recap log in Board | A | DONE | [watchtower/tasks/TASK-001-board-recap-log.md](watchtower/tasks/TASK-001-board-recap-log.md) | - | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Board only. Runs in parallel with group B. |
| 2 | TASK-002 Turn diff summary in Thread branch | B | DONE | [watchtower/tasks/TASK-002-thread-branch-turn-diff.md](watchtower/tasks/TASK-002-thread-branch-turn-diff.md) | - | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Shares index files and README with TASK-003. |
| 3 | TASK-003 GitHub PR and CI attachment in Thread branch | B | DONE | [watchtower/tasks/TASK-003-thread-branch-pr-attachment.md](watchtower/tasks/TASK-003-thread-branch-pr-attachment.md) | TASK-002 | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Same files as TASK-002, so it runs after it. |

TASK Status labels: TODO, IN PROGRESS, BLOCKED, DONE.
Plan-level Status header: ACTIVE while any row is open, DONE when all rows DONE, ARCHIVED after archive.

## Plan Verify

- `cd plugins/board && npm run typecheck && npm test && S=lint; npm run $S` -> all pass.
- `cd plugins/thread-branch && npm run typecheck && npm test && S=lint; npm run $S` -> all pass.
- `git status --short -- plugins` -> no new plugin folder. Only files in [plugins/board](plugins/board) and [plugins/thread-branch](plugins/thread-branch) changed.
- Manual, needs the Paseo app: after `paseo plugin reload board` and `paseo plugin reload thread-branch`, one agent turn that ends with `## Recap` and edits a file shows a turn diff row, and the Board Recaps view lists that recap under today and its project.

## Handoff

- Next action: in the Paseo app, finish one agent turn that edits a file and ends with `## Recap`. Then check the turn diff row, the Board Recaps view, and composer + -> GitHub PR.
- Verified on 2026-09-24: both plugins pass typecheck, tests (board 73/73, thread-branch 34/34), and lint. After reload, `paseo plugin ls` shows both `running` with no error.
- The group B reviewer found that the shared `run()` in [plugins/thread-branch/server/git.ts](plugins/thread-branch/server/git.ts) reported a timed-out process as exit 0. The fixer changed it to return `null` and added a test.
- Known limit: a Recaps project with no current run shows a neutral color mark.
- Carried over from the archived plan: the manual in-app check of `paseo-thread-search` is still PENDING-USER.
- Committed on branch `feat/recap-log-turn-diff-pr-attach` (board, thread-branch, watchtower, then the turn diff follow-ups). Not pushed.

## Archive

- [watchtower/archive/20260918-optional-paseo-plugins](watchtower/archive/20260918-optional-paseo-plugins)
- [watchtower/archive/20260920-arc-style-project-spaces](watchtower/archive/20260920-arc-style-project-spaces)
- [watchtower/archive/20260923-thread-janitor-and-context-attach](watchtower/archive/20260923-thread-janitor-and-context-attach)
- [watchtower/archive/20260923-thread-attach-jev-walkback-status](watchtower/archive/20260923-thread-attach-jev-walkback-status)
- [watchtower/archive/20260923-thread-export-qmd-search](watchtower/archive/20260923-thread-export-qmd-search)
- Archived: 2026-09-24 -> watchtower/archive/20260924-recap-log-turn-diff-pr-attach/
