# Learn 20261013-turn-step-list-follow-ups

## Summary

Discrepancy: 2 found. TASK-092 needed a view change that the spec did not list, and TASK-094 crashed live on web although every test passed.

## Per TASK

- TASK-092: plan changed the step split in the model and the step expectations in the tests -> shipped that, plus a `turn/view.tsx` change. Mistake: the spec file list missed the view, which drew edits above the note and marked the note as streaming while a tool call ran. Likely cause: the spec checked the model rule, not what the view assumes about the note position. Fix: when a model rule moves an item inside a group, list the view that orders that group.
- TASK-093: match. The thread has a note between its two edits, so the live check did not show two adjacent edit rows.
- TASK-094: plan read `theme.colorScheme` in `StyleSheet.create`, as the workspace label chip does -> shipped per-scheme tones in the theme (`theme.turnLabels`, ADR-0022). Mistake: on web the style factory gets a theme of CSS variables, so `theme.colorScheme` is not `light` or `dark`. The vitest Unistyles stub passes a plain theme, so the tests passed. Likely cause: the spec copied the chip pattern without a live check. Fix: verify a theme-computed style on live before DONE.

## Plan-Level

- The owner first chose to skip the manual live checks. The skipped check was the only one that could catch the TASK-094 crash, and the commit step found it. Keep a live reload for any TASK that changes a style factory.

## Lessons

- A pattern found in the code is not proof that it works on every platform. The workspace label chip has the same `theme.colorScheme` branch and shows dark colors on web in the light theme; report it as a separate bug.
- Read the live window after each app change. The dev window keeps a stale error page until you reload it after Metro rebuilds.
