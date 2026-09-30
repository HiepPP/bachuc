# NEXT

## Current Active Plan

- Title: Export Paseo threads to markdown and let agents search them with qmd
- Slug: 20260923-thread-export-qmd-search
- Status: ARCHIVED
- Updated: 2026-09-24

## Tracker

One row per TASK. Group ties together items that write the same files.

| Order | TASK | Group | Status | Spec | Deps | Context | Notes |
|-------|------|-------|--------|------|------|---------|-------|
| 1 | TASK-001 Export threads to markdown | A | DONE | [watchtower/tasks/TASK-001-export-threads-markdown.md](watchtower/tasks/TASK-001-export-threads-markdown.md) | - | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Hook `agent.turn_ended` plus a startup backfill. |
| 2 | TASK-002 Keep a qmd index of exported threads | A | DONE | [watchtower/tasks/TASK-002-qmd-index.md](watchtower/tasks/TASK-002-qmd-index.md) | TASK-001 | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Shares index.server.ts with TASK-001. |
| 3 | TASK-003 Skill that tells agents to search threads | B | DONE | [watchtower/tasks/TASK-003-thread-search-skill.md](watchtower/tasks/TASK-003-thread-search-skill.md) | TASK-002 | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Skill plus README. Links into Claude and Codex skill folders. |

TASK Status labels: TODO, IN PROGRESS, BLOCKED, DONE.
Plan-level Status header: ACTIVE while any row is open, DONE when all rows DONE, ARCHIVED after archive.

## Plan Verify

- `cd plugins/thread-context-attach && npm run typecheck && npm test && S=lint; npm run $S` -> all pass.
- After `paseo plugin reload thread-context-attach`, the export folder holds one `.md` file per non-archived thread.
- `qmd --index paseo-threads search "watchtower sidebar" -n 3` -> the top results include the thread `241e4fc4-e2c5-4c6b-9ca6-6fe83b34b984`.
- Manual, needs the Paseo app: after any thread finishes a turn, its file changes within a few seconds, and a search finds the new text within about 1 minute.

## Handoff

- Next action: the user asks a new agent in the Paseo app to find what other threads decided about the watchtower sidebar. The agent should use `paseo-thread-search` and cite thread `241e4fc4`.
- Verified live: a real turn end rewrote thread `8dca13d1` at 18:23:14, the index updated at 18:23:37, and words from that turn found the thread.
- Not committed. The two skill symlinks outside the repo are already in place.

## Archive

- [watchtower/archive/20260918-optional-paseo-plugins](watchtower/archive/20260918-optional-paseo-plugins)
- [watchtower/archive/20260920-arc-style-project-spaces](watchtower/archive/20260920-arc-style-project-spaces)
- [watchtower/archive/20260923-thread-janitor-and-context-attach](watchtower/archive/20260923-thread-janitor-and-context-attach)
- [watchtower/archive/20260923-thread-attach-jev-walkback-status](watchtower/archive/20260923-thread-attach-jev-walkback-status)
- Archived: 2026-09-24 -> watchtower/archive/20260923-thread-export-qmd-search/
