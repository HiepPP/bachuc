# NEXT

## Current Active Plan

- Title: Desktop workspace plugins, Jev delegation, and prompt actions
- Slug: 20260920-arc-style-project-spaces
- Status: ARCHIVED
- Updated: 2026-09-22

## Tracker

| Order | TASK | Group | Status | Spec | Deps | Context | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | TASK-001 Confirm desktop sidebar integration | A | DONE | [Spec](tasks/TASK-001-confirm-sidebar-integration.md) | - | [Context](CONTEXT.md) | Desktop sidebar injection approved. |
| 2 | TASK-002 Persist project Spaces | A | DONE | [Spec](tasks/TASK-002-persist-project-spaces.md) | TASK-001 | [Context](CONTEXT.md) | Desktop sidebar injection approved. |
| 3 | TASK-003 Add numbered workspace tabs | A | DONE | [Spec](tasks/TASK-003-workspace-tabs.md) | TASK-002 | [Context](CONTEXT.md) | Desktop sidebar injection approved. |
| 4 | TASK-004 Swipe between Spaces | A | BLOCKED | [Spec](tasks/TASK-004-swipe-between-spaces.md) | TASK-003 | [Context](CONTEXT.md) | Desktop sidebar injection approved. |
| 5 | TASK-005 Move projects between Spaces | A | DONE | [Spec](tasks/TASK-005-move-project-between-spaces.md) | TASK-004 | [Context](CONTEXT.md) | Desktop sidebar injection approved. |
| 6 | TASK-006 Board running and finished tasks | A | DONE | [Spec](tasks/TASK-006-board.md) | - | [Context](CONTEXT.md) | Installed; desktop menu and live run transition verified. Twelve tests pass. |
| 7 | TASK-007 Delegation broker | J | DONE | [Spec](tasks/TASK-007-delegation-broker.md) | - | [Context](CONTEXT.md) | Direct model/effort route added; legacy delegation preserved. |
| 8 | TASK-008 Adaptive escalation | J | DONE | [Spec](tasks/TASK-008-adaptive-escalation.md) | TASK-007 | [Context](CONTEXT.md) | Implemented; evidence in task outcome. |
| 9 | TASK-009 Discovery before implementation | J | DONE | [Spec](tasks/TASK-009-discovery-first.md) | TASK-008 | [Context](CONTEXT.md) | Implemented; evidence in task outcome. |
| 10 | TASK-010 Risk-based review | J | DONE | [Spec](tasks/TASK-010-risk-review.md) | TASK-009 | [Context](CONTEXT.md) | Implemented; evidence in task outcome. |
| 11 | TASK-011 Dependency and ownership scheduler | J | DONE | [Spec](tasks/TASK-011-dependency-scheduler.md) | TASK-010 | [Context](CONTEXT.md) | Implemented; evidence in task outcome. |
| 12 | TASK-012 Measured routing history | J | DONE | [Spec](tasks/TASK-012-routing-history.md) | TASK-011 | [Context](CONTEXT.md) | Direct benchmark: checks pass; 2.1253x tokens and 24.923% slower. |
| 13 | TASK-013 Inline prompt Send and optional Jev auto-send | P | IN PROGRESS | [Spec](tasks/TASK-013-next-prompt-actions.md) | - | [Context](CONTEXT.md) | Desktop adapter authorized and installed; live acceptance in progress. |
| 14 | TASK-014 Group Board conversations by project | A | BLOCKED | [Spec](tasks/TASK-014-board-project-groups.md) | TASK-006 | [Context](CONTEXT.md) | Implemented and reloaded; 27 tests pass. Remaining live acceptance needs test clients and states. |

## Plan Verify

- For TASK-007–012, run the orchestrator format, typecheck, lint, and offline tests.
- Verify the installed orchestrator and live child model/effort, task outputs, timing, and cleanup.
- Report baseline comparisons separately from synthetic/offline tests. Preserve TASK-004.

- Run plugin format, typecheck, lint, and tests.
- Verify the plugin runs and the existing sidebar supports tabs, project moves, and scoped wheel events.
- Verify membership survives reload and existing coding workspaces remain intact.
- Record device/browser limitations without claiming untested behavior.

## Handoff

- TASK-014 code is installed. Finish the remaining live checks in its [outcome](tasks/TASK-014-outcome.md) before DONE.

- TASK-013 desktop DOM adapter approved and installed. Complete live acceptance and record limitations.

- TASK-007–012 complete: Jev orchestrator installed; 24 tests and real MCP/child execution verified.
- [Live comparison](../../../plugins/jev-orchestrator/VERIFICATION.md): routed workflows passed but were slower on two small fixtures; no savings claim.

- Board is installed and running. Desktop light/dark, sidebar placement, and a real run transition are verified.
- Board retains 50 observed finished conversations in memory. Native mobile acceptance remains unverified.
- Spaces sidebar adapter implemented and installed. TASK-004 awaits physical swipe acceptance; other Spaces tasks are complete.
- Swipe horizontally across the native left-sidebar project list. Verify one adjacent tab per gesture.
- Fifteen tests, typecheck, lint, native project moves, filtering, and live disable/enable cleanup pass.
- Physical trackpad input remains unverified. Paseo source is unchanged. Mobile retains the standalone fallback.

## Archive

- [Optional plugins](../20260918-optional-paseo-plugins/NEXT.md)

- Archived: 2026-09-22 -> [Archive](.).
