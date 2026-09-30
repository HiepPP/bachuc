# NEXT

## Current Active Plan

- Title: Opt-in Jev reply walk-back and status labels for thread-context-attach
- Slug: 20260923-thread-attach-jev-walkback-status
- Status: ARCHIVED
- Updated: 2026-09-23

## Tracker

One row per TASK. Group ties together items that write the same files.

| Order | TASK | Group | Status | Spec | Deps | Context | Notes |
|-------|------|-------|--------|------|------|---------|-------|
| 1 | TASK-001 Recent replies and status in the snapshot | B | DONE | [watchtower/tasks/TASK-001-snapshot-replies-and-status.md](watchtower/tasks/TASK-001-snapshot-replies-and-status.md) | - | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Pure logic and tests. No Jev call. |
| 2 | TASK-002 Opt-in setting and settings screen | A | DONE | [watchtower/tasks/TASK-002-opt-in-setting.md](watchtower/tasks/TASK-002-opt-in-setting.md) | - | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Default off. |
| 3 | TASK-003 Jev worker and background enrichment | D | DONE | [watchtower/tasks/TASK-003-jev-enrichment.md](watchtower/tasks/TASK-003-jev-enrichment.md) | TASK-001, TASK-002 | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Registers the TASK-002 settings on the server. |
| 4 | TASK-004 README and live check | C | DONE | [watchtower/tasks/TASK-004-readme-and-live-check.md](watchtower/tasks/TASK-004-readme-and-live-check.md) | TASK-003 | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Live Jev check is billable. |

TASK Status labels: TODO, IN PROGRESS, BLOCKED, DONE.
Plan-level Status header: ACTIVE while any row is open, DONE when all rows DONE, ARCHIVED after archive.

## Plan Verify

- `cd plugins/thread-context-attach && npm run typecheck && S=lint; npm run $S && npm test` -> all pass.
- With the setting off, the attached text is byte-identical to the current format for the same thread.
- Manual, needs the Paseo app: with the setting on, a thread whose last reply is short gets older replies and a status line after one reopen of the picker.

## Handoff

- Next action: the user runs the manual in-app check. Turn on Settings -> Plugins -> Thread context -> Jev. Open + -> Thread twice, attach a thread, and confirm the `Status:` line. Then run `paseo plugin logs thread-context-attach` and confirm one Jev line with usage and no reply text.
- Pending user checks: the TASK-002 switch persists after reload, and the TASK-004 live Jev call. Both need the Paseo app. The live call is billable.
- Not committed.

## Archive

- [watchtower/archive/20260918-optional-paseo-plugins](watchtower/archive/20260918-optional-paseo-plugins)
- [watchtower/archive/20260920-arc-style-project-spaces](watchtower/archive/20260920-arc-style-project-spaces)
- [watchtower/archive/20260923-thread-janitor-and-context-attach](watchtower/archive/20260923-thread-janitor-and-context-attach)
- Archived: 2026-09-23 -> watchtower/archive/20260923-thread-attach-jev-walkback-status/
