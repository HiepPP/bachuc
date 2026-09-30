# TASK-002 Recap and What Next v2

Group: plugin (shared client integration)
Class: risky

## Brief

Complete the [selected v2 design](../../docs/designs/recap-next-multiple-2026-09-25/02-select-exclusive.png) in the existing next-prompt-actions plugin.
Render Recap as a compact responsive strip. Give real exclusive choices and additional suggestions separate groups with a shared action bar.
Preserve single suggestions, exact prompt text, draft behavior, and fail-closed selection rules.
Do not change global policy or Paseo application source.

Anti-goal: Zero failures in existing 65 tests. Check before edits and after integration.

## Verify

- Capture the missing compact Recap behavior before fixing it.
- Run plugin formatting, typecheck, lint, and affected tests.
- Reload only next-prompt-actions and confirm no load errors.
- Verify rendered Recap, one suggestion, exclusive choices, compatible combinations, Edit composer text, and actual Send on Paseo desktop.
- Record desktop evidence separately from fixtures; archive the disposable acceptance agent afterward.
