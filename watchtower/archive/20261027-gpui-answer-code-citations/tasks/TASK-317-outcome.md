# TASK-317 Outcome

## Outcome

Status: DONE

Changed:

- `CodeBlock` keeps `meta` from markdown-rs `raw.meta`, with `meta()` and `PartialEq` on it.
- `TextView::code_block_header` reaches `CodeBlock::render` through `TextViewState` and `NodeContext`.
- With a header, the frame drops its padding; a body box holds the code, the padding, and the corner actions.
- `BACHUC-PATCH.md` documents the patch.

Contract:

- With no header, or a hook that returns `None`, the block renders as before.

Verified:

- `./cargo.sh check -p bachuc-timeline` -> builds.
- `./cargo.sh test -p bachuc-timeline -p bachuc-plugin-host` -> 180 passed, 0 failed.
- The 2 planned parse tests in `format/markdown.rs` are UNSATISFIABLE: `gpui-base` is not a workspace member, so `cargo test -p gpui-base` fails with "requires dev-dependencies and is not a member of the workspace". TASK-318 covers `meta()` through its screen check.

Anti-goal:

- Final: timeline and plugin-host tests 180 passed, 0 failed; no test file changed; 2026-10-27.
- Result: PASS against 0 edited expected values.

Lessons:

- Tests of `vendor/gpui-base` cannot run in place; put them in a workspace crate that uses the public API (BACHUC-PATCH.md, text replacement).
- The lefthook format step fails with exit 2 when the only staged markdown file is ignored by oxfmt, such as `vendor/gpui-base/BACHUC-PATCH.md`. Stage another formatted markdown file with it.
