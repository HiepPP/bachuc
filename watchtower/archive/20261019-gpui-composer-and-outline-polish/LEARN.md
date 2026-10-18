# Learn 20261019-gpui-composer-and-outline-polish

## Summary

Discrepancy: 1 found. TASK-253 passed every code check but failed its first screen check; one fix round closed it.

## Per TASK

- TASK-253: plan `min_w_0` and shrink on the pill and the `density_box` row -> shipped that, plus `Popover::trigger_style` in fix round 1 (`4be6d17f5`). Mistake: the builder put `min_w_0` on the gpui-component `Popover`, which styles the menu panel, so the trigger never shrank. The spec named "the pill" without saying which element is the flex item. Fix: in a spec, name the exact flex item and how to style it (`trigger_style` for a popover trigger).
- TASK-254: match.
- TASK-255: plan finite 140 ms ease with a per-bar from-value -> shipped as planned. Mistake: none in code. The "two screenshots 70 ms apart" Verify line cannot pass with `cua_repl`, because one screenshot takes 95 to 460 ms. The hover and sweep checks needed the pointer and went to the owner (Q-169, passed). Fix: write animation timing as a unit test plus an owner hand check, not as a screenshot line.

## Plan-Level

- Plan misdescribes reality: two Verify lines (70 ms frames, fast sweep) were not runnable by the lead's tools. Otherwise none.

## Lessons

- Run screen checks on every layout TASK even when crate tests pass; a layout rule on the wrong element passes every code check.
- Plan frame-level animation checks as owner hand checks from the start.
