# NEXT

## Current Active Plan

- Title: Recap and What Next v2
- Slug: 20260925-next-prompts-v1
- Status: ARCHIVED
- Updated: 2026-09-26

## Tracker

| Order | TASK | Group | Status | Spec | Deps | Context | Notes |
|---|---|---|---|---|---|---|---|
| 1 | TASK-001 Next prompts v1 | plugin | DONE | [Spec](tasks/TASK-001-next-prompts-v1.md) | - | [Context](CONTEXT.md) | [Verification](tasks/TASK-001-outcome.md) passed before the global policy update. |

| 2 | TASK-002 Recap and What Next v2 | plugin | DONE | [Spec](tasks/TASK-002-recap-next-v2.md) | TASK-001 | [Context](CONTEXT.md) | [Verification](tasks/TASK-002-outcome.md) accepted by the user on desktop. |

## Plan Verify

- Pass plugin formatting, typecheck, lint, tests, rendering checks, and reload verification.
- Update the global policy only after the plugin gates pass. Preserve its Codex symlink.

## Handoff

- Both TASKs are DONE. The v3 panel shipped in commit `a0b286a` on `main`.
- The user accepted TASK-002 on desktop, including the multi-choice fixture check.
- Next: archive this plan with `/watchtower archive`.

## Archive

- Existing plans remain in [archive](archive/).
- Archived: 2026-09-26 -> watchtower/archive/20260925-next-prompts-v1/
