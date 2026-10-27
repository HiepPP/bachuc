# TASK-317 Code block header hook in the vendored text view

Group: B (vendored `gpui-base` text files only)
Class: risky

## Brief

Goal: A fenced code block keeps the text after its language (the fence meta), and a caller can draw a header row above the code from it.

Change: `CodeBlock` keeps only `lang`, and callers can add only corner actions -> `CodeBlock` also keeps `meta`, and `TextView::code_block_header` draws a caller element above the code.

Boundaries:

- The vendored text view draws every assistant message, so this is a hot render path. With no header hook set, the output must stay exactly as today.
- Follows [ADR-0050](watchtower/decisions/ADR-0050-electron-markdown-look-over-gpui-link-underline-and-bold.md): keep the Electron markdown look; the header sits inside the code block border, above the code.
- Trap: `Node::Code` in `format/markdown.rs`: markdown-rs gives `raw.meta`, but the code drops it today. A fence like ` ```rust src/lib.rs:10-20 ` has `lang = "rust"` and `meta = "src/lib.rs:10-20"`.
- Trap: `render` of `CodeBlock` in `node.rs`: the top padding grows when a language label shows. A header must replace that padding logic, not stack on it, so the first code line keeps the same gap below the header.
- Trap: `PartialEq for CodeBlock` in `node.rs` compares lang, code, and span only. Add `meta`, or a meta change will not refresh the block.
- Planner call: the meta sits on the opening fence line, so the header shows as soon as the fence opens, also while it streams. It does not wait for the closing fence.
- Planner call: the header sits inside the border, above the scrolled code, and outside the scroll area. The corner actions keep their place over the code.
- Trap: the hook returns `Option<AnyElement>`. When it returns `None` for a block, that block must render exactly as with no hook.

How:

- Add `meta: Option<SharedString>` to `CodeBlock`, with `pub fn meta(&self)`, and pass `raw.meta` in `Node::Code`.
- Add `code_block_header` to `TextView`, stored and passed to `NodeContext` like `code_block_actions`.
- In `CodeBlock::render`, put the header element above the code inside the same bordered box when the hook returns `Some`.

Files:

- [packages/desktop-gpui/vendor/gpui-base/src/text/node.rs](packages/desktop-gpui/vendor/gpui-base/src/text/node.rs) (`meta` field, accessor, and `PartialEq`; header in `render`)
- [packages/desktop-gpui/vendor/gpui-base/src/text/format/markdown.rs](packages/desktop-gpui/vendor/gpui-base/src/text/format/markdown.rs) (pass `raw.meta`; tests)
- [packages/desktop-gpui/vendor/gpui-base/src/text/text_view.rs](packages/desktop-gpui/vendor/gpui-base/src/text/text_view.rs) (`code_block_header` builder and state plumbing)
- [packages/desktop-gpui/vendor/gpui-base/src/text/state.rs](packages/desktop-gpui/vendor/gpui-base/src/text/state.rs) (only if the hook must live in the state, as `code_block_actions` does)

Interfaces:

- Produces: `CodeBlock::meta(&self) -> Option<SharedString>` for TASK-318
- Produces: `TextView::code_block_header<F>(self, f: F) -> Self` where `F: Fn(&CodeBlock, &mut Window, &mut App) -> Option<AnyElement> + Send + Sync + 'static` for TASK-318

Expected result:

- WHEN markdown has the fence ` ```rust src/lib.rs:10-20 ` THE parsed `CodeBlock` SHALL have `lang() == Some("rust")` and `meta() == Some("src/lib.rs:10-20")`.
- WHEN a fence has no meta THE parsed `CodeBlock` SHALL have `meta() == None`.
- WHILE no header hook is set, THE code block SHALL render with the same padding and label as today.
- WHEN the header hook returns `Some(element)` THE code block SHALL draw the element above the code, inside the border.

Red tests:

File: [packages/desktop-gpui/vendor/gpui-base/src/text/format/markdown.rs](packages/desktop-gpui/vendor/gpui-base/src/text/format/markdown.rs) (the existing tests module, next to the fence parse tests)

The builder adds two parse tests in the style of the existing fence tests in that module: the fence `rust src/lib.rs:10-20` gives `lang() == Some("rust")` and `meta() == Some("src/lib.rs:10-20")`; a plain `rust` fence gives `meta() == None`.

Anti-goal: the `gpui-base` text tests and the `bachuc-timeline` tests keep their pass counts (plus the new tests) with 0 edited expected values; tripwire; read before the change and at completion.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p gpui-base text::format::markdown` -> pass, including the 2 new tests.
- `cd packages/desktop-gpui && ./cargo.sh test -p gpui-base text::` -> pass; count is the old count plus 2 (anti-goal).
- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-timeline` -> pass with the same count as before (anti-goal).
- `git diff -U0 -- packages/desktop-gpui/crates/timeline/tests packages/desktop-gpui/vendor/gpui-base/src/text | rg '^-\s+assert'` -> no match.
- `cd packages/desktop-gpui && ./cargo.sh clippy -p gpui-base` -> no new warning.
