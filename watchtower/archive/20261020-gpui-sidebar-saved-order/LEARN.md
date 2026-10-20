# Learn 20261020-gpui-sidebar-saved-order

## Summary

Discrepancy: 3 found, all small. The code shipped as planned. Two Verify lines and one Context line did not match the repo.

## Per TASK

- TASK-278: match. One mistake in the spec: the Verify clippy line `clippy -p bachuc-sidebar -- -D warnings` cannot pass. It also lints path dependencies (`protocol`, `theme`), and `kebab()` at HEAD has 8 arguments. Fix: write `--no-deps --lib --test <name>` in clippy Verify lines.
- TASK-279: match. One mistake in the spec: the `Cargo.lock` Verify line expected only new entries and a foreign line. A second `errno` version added a suffix to 2 old references, and the foreign line was committed before the run. Fix: let a lock-file line allow version suffixes on existing references. The lead screen check saw only a stale background frame, so the owner checked the order by hand (Q-182).
- TASK-280: match. The drag checks needed the owner's hand, as the spec said (Q-180). No click guard was needed, because GPUI drops the pending click when a drag starts.

## Plan-Level

- `CONTEXT.md` described uncommitted foreign changes that were committed before the run. The first implement run stopped on a dirty tree. Fix: commit or stash foreign work before `implement`, and keep tree notes out of `CONTEXT.md`.
- Another agent edited `crates/app/src/shell.rs` on the run branch during the run. The lead did not stage it.

## Lessons

- A screen check that reads GPUI content needs a painted window. A check bundle opened with `open -g` keeps a stale frame and an empty AX tree, so plan an owner check for it.
- A saved-order lookup on a sorted `Vec` with binary search stays within 1.1x of the old `build_rows` time; a `HashMap` per list cost 1.46x in a debug build.
