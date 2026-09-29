# NEXT

## Current Active Plan

- Title: Skill pins plugin
- Slug: 20260929-skill-pins-plugin
- Status: ARCHIVED
- Updated: 2026-09-29

## Tracker

One row per TASK. Group ties together items that ship as one transaction.

| Order | TASK | Group | Status | Spec | Deps | Context | Notes |
|-------|------|-------|--------|------|------|---------|-------|
| 1 | TASK-001 Scaffold, catalog, and server state | A | DONE | [watchtower/tasks/TASK-001-scaffold-catalog-state.md](watchtower/tasks/TASK-001-scaffold-catalog-state.md) | - | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Q0 resolved: separate plugin. |
| 2 | TASK-002 Native hook and installer | B | DONE | [watchtower/tasks/TASK-002-native-hook-installer.md](watchtower/tasks/TASK-002-native-hook-installer.md) | TASK-001 | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Needs the catalog and state layout. |
| 3 | TASK-003 Composer skills pill | C | DONE | [watchtower/tasks/TASK-003-composer-skills-pill.md](watchtower/tasks/TASK-003-composer-skills-pill.md) | TASK-001 | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Run after TASK-002; both edit package.json scripts. |
| 4 | TASK-004 Install and live verify | D | DONE | [watchtower/tasks/TASK-004-install-live-verify.md](watchtower/tasks/TASK-004-install-live-verify.md) | TASK-002, TASK-003 | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | User verified Escape and D2. User skipped dark theme, queued prompt, Caveman, and Codex checks. |
| 5 | TASK-005 Default skill set for new agents | E | DONE | [watchtower/tasks/TASK-005-default-skill-set.md](watchtower/tasks/TASK-005-default-skill-set.md) | TASK-004 | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Q2. Adds the agent.create env path that TASK-006 reuses. |
| 6 | TASK-006 Pins in new-thread composers | E | DONE | [watchtower/tasks/TASK-006-new-thread-pins.md](watchtower/tasks/TASK-006-new-thread-pins.md) | TASK-005 | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Q1. Desktop check done through the real composer on 2026-09-29. |

TASK Status labels: TODO, IN PROGRESS, BLOCKED, DONE.
Plan-level Status header: ACTIVE while any row is open, DONE when all rows DONE, ARCHIVED after archive.

## Plan Verify

- `cd plugins/skill-pins && npm run typecheck && S=lint; npm run $S && npm test` -> exit 0.
- `paseo plugin ls skill-pins --json` -> status `running`, no load error.
- Manual in Paseo desktop: pin Watchtower, send a prompt to a Claude agent -> the agent calls the Skill tool for `watchtower` before it answers.
- Manual in Paseo desktop: pin a skill in a new-thread composer and send -> the first turn of the new agent calls that skill.

## Handoff

- Next action: all six TASKs are DONE. Run `/watchtower archive` to close the plan, then commit the verify records.

## Archive

- Archived: 2026-09-29 -> watchtower/archive/20260929-skill-pins-plugin/
- [watchtower/archive/20260927-board-archive-visibility/](watchtower/archive/20260927-board-archive-visibility/)
- [watchtower/archive/20260925-next-prompts-v1/](watchtower/archive/20260925-next-prompts-v1/)
- [watchtower/archive/20260924-snapshot-size-cap-ci-log-pending/](watchtower/archive/20260924-snapshot-size-cap-ci-log-pending/)
- [watchtower/archive/20260924-recap-log-turn-diff-pr-attach/](watchtower/archive/20260924-recap-log-turn-diff-pr-attach/)
- [watchtower/archive/20260923-thread-export-qmd-search/](watchtower/archive/20260923-thread-export-qmd-search/)
- [watchtower/archive/20260923-thread-janitor-and-context-attach/](watchtower/archive/20260923-thread-janitor-and-context-attach/)
- [watchtower/archive/20260923-thread-attach-jev-walkback-status/](watchtower/archive/20260923-thread-attach-jev-walkback-status/)
- [watchtower/archive/20260920-arc-style-project-spaces/](watchtower/archive/20260920-arc-style-project-spaces/)
- [watchtower/archive/20260918-optional-paseo-plugins/](watchtower/archive/20260918-optional-paseo-plugins/)
