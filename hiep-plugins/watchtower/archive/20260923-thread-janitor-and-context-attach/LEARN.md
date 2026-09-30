# Learn 20260923-thread-janitor-and-context-attach

## Summary

Discrepancy: 2 found. Both plugins shipped, but one plan assumption about the sidebar was wrong, and one spec step could not be met.

## Per TASK

- TASK-001: plan archived idle agents only -> shipped agent archiving plus idle workspace archiving. Mistake: the plan assumed sidebar rows are agents. They are workspaces, so 95 empty workspaces stayed visible after 105 agents were archived. Fix: check what a UI surface lists before planning a cleanup for it.
- TASK-002: plan listed the current workspace first and hid the current thread in the picker -> shipped a newest-first picker. Mistake: the plan assumed the attachment search gets composer context. Paseo sends only `{ query }`. Fix: read the host call site of a contribution API before writing Expected result.
- TASK-003: match.

## Plan-Level

- The user could not find the attach entry point. Plugin READMEs and the reply should name the exact UI path, for example the + button in the message box.

## Lessons

- Before planning a UI-facing plugin, confirm what the target UI lists and what context the host passes, using the Paseo source in `~/Projects/paseo`.
- Workspace archive in Paseo also archives its agents and kills its terminals, and it can remove Paseo-owned worktrees. Guard against all three.
