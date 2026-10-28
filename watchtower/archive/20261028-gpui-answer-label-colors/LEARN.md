# Learn 20261028-gpui-answer-label-colors

## Summary

Discrepancy: 3 found. The two TASKs shipped, but the color design changed during the build, the vendor test lines could not run, and later owner asks extended the work past the plan.

## Per TASK

- TASK-323: plan said plain colored text ([ADR-0075](watchtower/decisions/ADR-0075-semantic-status-colors-for-answer-labels.md)) with tests in `gpui-base` -> shipped tint chips through the inline flow ([ADR-0076](watchtower/decisions/ADR-0076-semantic-tint-chips-for-answer-labels.md)), with the matcher made public and tested from the timeline crate. Mistake: the Verify lines used `cargo test -p gpui-base`, which TASK-317 had already shown cannot run. Fix: read the earlier outcome of the same vendor crate before writing Verify lines.
- TASK-324: plan said a screen check on live -> the live check bundle stopped drawing in the background, so most lines stayed UNVERIFIED until a release check with the app in front. Mistake: the plan assumed a background screen check can scroll a long answer. Fix: plan short sample answers, and plan a foreground check when the owner is away.

## Plan-Level

- Scope creep by owner ask after the plan closed: more labels, the one-line Recap ([ADR-0077](watchtower/decisions/ADR-0077-recap-branch-line-only.md)), and the group spacing were recorded in the TASK-324 outcome, not as their own TASKs.
- The owner asked the lead to build every TASK itself, so no builder agents ran.

## Lessons

- Check whether a crate's tests can run in the workspace before you write its Verify lines.
- When an owner ask adds work after a plan is DONE, add TASK rows, so the outcomes do not mix several features.
- A GPUI screen check from `cua_repl` needs a visible window: press `End`, read the AX state, then act, one action per read.
