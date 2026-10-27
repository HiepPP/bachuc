# Learn 20261027-gpui-answer-code-citations

## Summary

Discrepancy: 3 found. Code shipped for all TASKs, but one spec premise was false and some checks stayed unverified.

## Per TASK

- TASK-316: match. The GPUI step list text was not checked by eye; only the data path passed on release.
- TASK-317: plan said 2 parse tests in `gpui-base` -> shipped without them. Mistake: `gpui-base` is not a workspace member, so its tests cannot run. Fix: put vendor tests in a workspace crate that uses the public API.
- TASK-318: match. The screen check passed through the AX tree, not a screenshot.
- TASK-319: match.
- TASK-320: plan said fix a lost Bash description -> shipped no code. Mistake: the spec assumed a bug from 0 stored descriptions; the Claude calls sent none. Fix: check the raw provider transcript before planning a fix.
- TASK-321: match. The owner check after the release install is still pending.
- TASK-322: match. The owner check after the release install is still pending.

## Plan-Level

- Scope creep: TASK-320 to TASK-322 joined the plan after it started; the menu bar app is not about answer citations.

## Lessons

- Confirm a missing-field bug in the raw provider input before writing a TASK for it.
- Put unrelated follow-up work in a new plan, not the active one.
