# NEXT

## Current Active Plan

- Title: Thread Janitor and Thread Context Attach plugins
- Slug: 20260923-thread-janitor-and-context-attach
- Status: ARCHIVED
- Updated: 2026-09-23

## Tracker

One row per TASK. Group ties together items that write the same files.

| Order | TASK | Group | Status | Spec | Deps | Context | Notes |
|-------|------|-------|--------|------|------|---------|-------|
| 1 | TASK-001 Thread Janitor plugin | A | DONE | [watchtower/tasks/TASK-001-thread-janitor.md](watchtower/tasks/TASK-001-thread-janitor.md) | - | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Auto-archive idle threads after 1 day, no confirm. |
| 2 | TASK-002 Thread Context Attach plugin | B | DONE | [watchtower/tasks/TASK-002-thread-context-attach.md](watchtower/tasks/TASK-002-thread-context-attach.md) | - | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Attach another thread's last reply. Picker cannot know the current thread, so it lists newest first only. |
| 3 | TASK-003 Root README catalog entries | C | DONE | [watchtower/tasks/TASK-003-readme-catalog.md](watchtower/tasks/TASK-003-readme-catalog.md) | TASK-001, TASK-002 | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Docs only. |

TASK Status labels: TODO, IN PROGRESS, BLOCKED, DONE.

## Plan Verify

- `paseo plugin ls thread-janitor --json` and `paseo plugin ls thread-context-attach --json` -> both show `running` with no load error.
- `git status --short` -> no API keys, local config, or real prompts are staged.

## Handoff

- Next action: check both plugins in the Paseo desktop app. Confirm idle threads left the list, and composer + -> Thread attaches a reply. Then run `/watchtower archive`.

## Archive

- [20260918-optional-paseo-plugins](watchtower/archive/20260918-optional-paseo-plugins)
- [20260920-arc-style-project-spaces](watchtower/archive/20260920-arc-style-project-spaces)
- Archived: 2026-09-23 -> watchtower/archive/20260923-thread-janitor-and-context-attach/
