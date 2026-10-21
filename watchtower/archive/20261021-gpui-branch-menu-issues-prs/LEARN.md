# Learn 20261021-gpui-branch-menu-issues-prs

## Summary

Discrepancy: none. The shipped menu rows, URLs, and tests match the plan.

## Per TASK

- TASK-281: match. The manual pill menu check stayed UNVERIFIED, because the background check app kept a stale frame. Fix: Hiep checks the two rows by hand in the next GPUI build.

## Plan-Level

- none

## Lessons

- A background GPUI check app may paint only after a click. Plan a hand check for menu rows when the screen check cannot show the popup.
