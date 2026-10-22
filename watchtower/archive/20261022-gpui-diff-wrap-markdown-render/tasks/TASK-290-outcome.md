# TASK-290 outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/explorer/src/markdown_diff.rs` (new)
- `packages/desktop-gpui/crates/explorer/src/lib.rs` (`pub mod markdown_diff;`)
- `packages/desktop-gpui/crates/explorer/tests/models.rs` (10 red tests and the `use` line)

## Contract

- `is_markdown_path(&str) -> bool`, `old_text(&str, &str) -> Option<String>`, `block_diff(&str, &str) -> Vec<RichBlock>`.
- `RichBlock { kind: BlockKind, markdown: String }` and `BlockKind { Same, Added, Removed }`, both `Clone, Debug, PartialEq, Eq` (`BlockKind` is also `Copy`).
- `Same` blocks carry the new text's markdown. A fence mid-paragraph stays in the same block.

## Verified

- Red: `./cargo.sh test -p bachuc-explorer --test models` before the module -> `E0432 unresolved import bachuc_explorer::markdown_diff` (compile error counts as red).
- `./cargo.sh test -p bachuc-explorer --test models` -> 31 passed (baseline 21 + 10).
- `./cargo.sh test -p bachuc-explorer` -> 56 + 38 + 31 passed, 0 failed (group-level run).
- `./cargo.sh fmt --check -p bachuc-explorer` -> exit 0.
- `git diff --stat` on `diff.rs` and explorer `Cargo.toml` -> no change.
- Clippy `--no-deps -- -D warnings` fails on the existing `large_enum_variant` at `file_view.rs:86` (file untouched by this TASK). With `-A clippy::large_enum_variant` it passes, so `markdown_diff.rs` is clean.

## Anti-goal

Explorer suite checkpoints (source `./cargo.sh test -p bachuc-explorer`):

- before the change: 56 + 38 + 21 passed, 0 failed.
- after `old_text` / at completion: 56 + 38 + 31 passed, 0 failed. The new tests were added in one step with the model, so no separate read exists between `old_text` and `block_diff`. PASS.

## Lessons

- `file_view.rs:86` (`Text(TextDoc)` enum) trips `clippy::large_enum_variant` on Rust 1.97; any later explorer TASK that runs the clippy gate sees it. Not caused by this plan.
