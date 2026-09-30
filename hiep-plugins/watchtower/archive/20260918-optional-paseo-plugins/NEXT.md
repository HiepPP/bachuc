# NEXT

## Current Active Plan

- Title: Optional Paseo plugins
- Slug: 20260918-optional-paseo-plugins
- Status: ARCHIVED
- Updated: 2026-09-20

## Tracker

These are proposals, not an approved implementation queue. Select tasks before implementation.

| Order | TASK | Group | Status | Spec | Deps | Context | Notes |
|-------|------|-------|--------|------|------|---------|-------|
| 1 | TASK-001 workspace-preflight | A | DONE | [Spec](tasks/TASK-001-workspace-preflight.md) | - | [Context](CONTEXT.md) | Checks, plugin reload, slash action, and wide/narrow UI verified on 2026-09-20. |
| 2 | TASK-002 evidence-ledger | A | TODO | [Spec](tasks/TASK-002-evidence-ledger.md) | - | [Context](CONTEXT.md) | Awaiting user selection. |
| 3 | TASK-003 handoff-pack | A | TODO | [Spec](tasks/TASK-003-handoff-pack.md) | - | [Context](CONTEXT.md) | Awaiting user selection. |
| 4 | TASK-004 preview-launcher | A | TODO | [Spec](tasks/TASK-004-preview-launcher.md) | - | [Context](CONTEXT.md) | Awaiting user selection. |
| 5 | TASK-005 watchtower-board | A | DONE | [Spec](tasks/TASK-005-watchtower-board.md) | - | [Context](CONTEXT.md) | UI and exact attachment search verified on 2026-09-19. |
| 6 | TASK-006 release-trace | A | TODO | [Spec](tasks/TASK-006-release-trace.md) | - | [Context](CONTEXT.md) | Awaiting user selection. |
| 7 | TASK-007 dashboard-ui | A | DONE | [Spec](tasks/TASK-007-dashboard-ui.md) | - | [Context](CONTEXT.md) | Reference dashboard implemented; checks and native UI pass. |
| 8 | TASK-008 project-explorer-tab | A | DONE | [Spec](tasks/TASK-008-project-explorer-tab.md) | - | [Context](CONTEXT.md) | Explorer placement and absence of obsolete center tab verified on 2026-09-20. |
| 9 | TASK-009 preflight-jev-agent | A | DONE | [Spec](tasks/TASK-009-preflight-jev-agent.md) | TASK-001 | [Context](CONTEXT.md) | Discovery/MCP, 16 offline tests, 3 live Jev scenarios, reload and native UI verified. |

## Handoff

- TASK-001 is complete. The workspace-preflight plugin is installed and running.
- Missing preflight categories now use bounded discovery; configure unsupported layouts and task-critical services explicitly.

- TASK-005 and TASK-007 are complete. The updated plugin is installed and running.
- Desktop wide/narrow, theme, error, refresh, and attachment checks passed on 2026-09-19.
- TASK-008 is complete. Native UI confirmed no obsolete center tab and the original workspace restored on 2026-09-20.
- Keep other tasks unselected. Implement only task IDs chosen by the user.
- TASK-009 is complete. New Paseo Codex/Claude agents receive workspace_preflight; Jev remains independently installed.
- Other proposals remain unselected.
- Group A reflects shared preflight files and the root catalog.

## Archive

- Archived: 2026-09-20 -> watchtower/archive/20260918-optional-paseo-plugins/
- Archived at user request with TASK-002, TASK-003, TASK-004 and TASK-006 still TODO; not implemented.
