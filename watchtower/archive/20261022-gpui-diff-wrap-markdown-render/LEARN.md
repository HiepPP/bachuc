# Learn 20261022-gpui-diff-wrap-markdown-render

## Summary

Discrepancy: 5 found. All 10 TASKs shipped and passed their checks, but the plan missed that the one-file diff had no entry, and several Verify lines could not run unattended.

## Per TASK

- TASK-288: plan one-file and commit diff toolbar -> shipped both. Mistake: the plan assumed a user path opens the one-file diff; none did (Q-186). Fix: before planning UI on a view, confirm in `shell.rs` that a user path opens it.
- TASK-289: match. The open-at-a-line screen check had no keyboard path (the Command Center has no line syntax); Hiep's owner checks covered the rest.
- TASK-290: match. Verify clippy without `--no-deps` was unsatisfiable on old lints; the run used `--no-deps`.
- TASK-291: plan Rendered mode in the one-file diff -> shipped, but unreachable until TASK-297. Mistake: same as TASK-288. Fix: same.
- TASK-292: match. Clippy fails on old lines in `page.rs` and `core.rs`; one client timing test flaked under load.
- TASK-293: plan Files list -> shipped plus a small edit to `crates/timeline/src/lib.rs` for card state. Mistake: the spec missed that `TimelineView` fields live in `lib.rs`. Fix: list the file that owns a struct when a TASK adds a field to it. A literal fast double press was impossible because the toolbar closes after the first press.
- TASK-294: match.
- TASK-295: match. The Verify count of `"name":"gpui-kit"` in `cargo metadata` rises by one per dependent crate; count `^name = "gpui-kit"` in `Cargo.lock` instead.
- TASK-296: plan double-click selects and does not toggle -> first build toggled once; fix round 1 added `row_click` with a restore on click count 2. Bold runs draw at medium weight in the muted color, accepted by Hiep (Q-185).
- TASK-297: match in code. Mistake: the only entry is a row context menu, which a UI checker cannot open without the pointer, so its checks fell to Hiep (Q-189). Fix: give a new menu item a key or AX press path, or plan its check as an owner check.

## Plan-Level

- Plan misdescribes reality: TASK-288 and TASK-291 were planned on `Source::File` without checking that it is reachable; TASK-297 was added after the run to fix it.
- Verify lines used `clippy -- -D warnings` without `--no-deps` and timing tests that fail under load; MEMORY already held the `--no-deps` rule.
- Several screen checks needed the pointer (right-click, drag, click outside); they became owner checks Q-187 to Q-189.

## Lessons

- Confirm a user path opens a view before planning UI on it.
- Write clippy Verify lines with `--no-deps`, and judge old lints with `git blame`.
- Plan pointer-only checks as owner checks from the start, or add a key or AX path.
