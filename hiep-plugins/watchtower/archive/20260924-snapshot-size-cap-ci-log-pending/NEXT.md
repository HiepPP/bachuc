# NEXT

## Current Active Plan

- Title: Cap untracked file size in turn snapshots, and mark CI logs that are not ready yet
- Slug: 20260924-snapshot-size-cap-ci-log-pending
- Status: ARCHIVED
- Updated: 2026-09-25

## Tracker

One row per TASK. Group ties together items that write the same files.

| Order | TASK | Group | Status | Spec | Deps | Context | Notes |
|-------|------|-------|--------|------|------|---------|-------|
| 1 | TASK-001 Skip large untracked files in turn snapshots | A | DONE | [watchtower/tasks/TASK-001-snapshot-untracked-size-cap.md](watchtower/tasks/TASK-001-snapshot-untracked-size-cap.md) | - | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Touches the real-index safety path. Class risky. |
| 2 | TASK-002 Keep CI logs of running workflows as not ready | A | DONE | [watchtower/tasks/TASK-002-ci-log-not-ready.md](watchtower/tasks/TASK-002-ci-log-not-ready.md) | - | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Shares the thread-branch README with TASK-001. |

TASK Status labels: TODO, IN PROGRESS, BLOCKED, DONE.
Plan-level Status header: ACTIVE while any row is open, DONE when all rows DONE, ARCHIVED after archive.

## Plan Verify

- `cd plugins/thread-branch && npm run typecheck && npm test && S=lint; npm run $S` -> all pass.
- After `paseo plugin reload thread-branch`, `paseo plugin ls thread-branch --json` -> `running` with no error.
- After a real agent turn in a git repository, `git diff --cached --name-only` in that repository -> the same output as before the turn. The snapshot must never stage files.

## Handoff

- Next action: commit the plan and the thread-branch changes, then open a PR. Nothing is committed yet.
- Verified on 2026-09-24: `plugins/thread-branch` passes typecheck, tests 57/57, and lint. After reload, it is `running`. Probe agent `b6c1a9f2` got a clickable card, and nothing new was staged. A plumbing compare showed only the 8 renames of the plan archive.
- Not verified live: the not-ready state, because no workflow run was in progress. A 2 MiB untracked file in a real turn is covered by a unit test only.
- Still open from the archived plan: the user has not confirmed that the PR picker lists PRs in the app, or the `paseo-thread-search` check.
- The unrelated changes in [plugins/thread-context-attach](plugins/thread-context-attach) are not part of this plan. Leave them alone.

## Archive

- [watchtower/archive/20260918-optional-paseo-plugins](watchtower/archive/20260918-optional-paseo-plugins)
- [watchtower/archive/20260920-arc-style-project-spaces](watchtower/archive/20260920-arc-style-project-spaces)
- [watchtower/archive/20260923-thread-janitor-and-context-attach](watchtower/archive/20260923-thread-janitor-and-context-attach)
- [watchtower/archive/20260923-thread-attach-jev-walkback-status](watchtower/archive/20260923-thread-attach-jev-walkback-status)
- [watchtower/archive/20260923-thread-export-qmd-search](watchtower/archive/20260923-thread-export-qmd-search)
- [watchtower/archive/20260924-recap-log-turn-diff-pr-attach](watchtower/archive/20260924-recap-log-turn-diff-pr-attach)

- Archived: 2026-09-25 -> watchtower/archive/20260924-snapshot-size-cap-ci-log-pending/
