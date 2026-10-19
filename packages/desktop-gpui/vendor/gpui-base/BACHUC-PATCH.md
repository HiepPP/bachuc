# Bachuc patch to gpui-base

This folder is the published crate gpui-base 0.7.3 from crates.io, with one change. The workspace `Cargo.toml` points every crate at it through `[patch.crates-io]`. Reason: [ADR-0044](../../../../watchtower/decisions/ADR-0044-vendored-gpui-base-patch-over-git-fork-or-log.md).

## The change

`src/input/base/state.rs`, in `InputBaseState::render`: the `div().id("input-state")` element tracks focus but had no role. GPUI then logged "has no accessibility node" for every focused text input. The patch adds `.role(gpui::Role::Group)` to that element, and a local `use gpui::StatefulInteractiveElement as _;` so the method resolves.

- The outer frame already has `Role::TextInput` (`src/input/base/mod.rs`), so the patch uses the neutral `Group` role and no label. This adds no second text field and no second label to the AX tree.
- GPUI filters `Role::GenericContainer` out of the AX tree, and `role()` debug-asserts against it, so `Group` is the most neutral role that makes a node.
- Every other file is unchanged. Check with `diff -r ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-base-0.7.3 vendor/gpui-base`.

## List spacing (TASK-243)

`TextViewStyle::with_list_spacing(top, bottom)` and `list_spacing()` (`src/text/style.rs`) set the space above and below a top-level list (`BlockNode::List` in `src/text/node.rs`, `depth == 0` and not inside a blockquote). The default `None` keeps the paragraph gap below and no space above. A list that is the last block keeps no bottom gap. `NodeRenderOptions.in_blockquote` (`src/text/document.rs`) marks blocks inside a blockquote.

## Re-apply after a gpui-kit upgrade

1. If upstream gpui-base now gives `input-state` a role, delete this folder and the `[patch.crates-io]` entry.
2. Otherwise, copy the new gpui-base version from `~/.cargo/registry/src/index.crates.io-*/gpui-base-<version>` into a new `vendor/gpui-base`, with `cp -R`.
3. Re-apply the change above to the `input-state` element and copy this file in.
4. Run `./cargo.sh tree -i gpui-base` and check that it shows the `vendor/gpui-base` path. If cargo warns that the patch was not used, the version in this folder does not match the version gpui-kit requires.

## Markdown look (TASK-269)

`src/text/node.rs` draws assistant markdown as Electron does, per ADR-0050:

- `mark_highlight` gives `strong` `FontWeight::MEDIUM` (500), Electron's `strong` weight in `markdown-styles.ts`. The node test that reads the object style expects `MEDIUM`.
- `apply_link` replaces every link's color and clears its underline: Electron's `link` has `textDecorationLine: "none"` and the `accentBright` color. The four former `link_underline` call sites (paragraph text, inline flow text, inline objects, and inline flow objects) now call `apply_link` or set no underline; `link_underline` is removed.
- `apply_link` keeps an inline code span's background inside a link, so a code link keeps its `surface2` pill. The object path re-applies it with `inline_code_highlight().background_color`.
- Re-apply these edits to `src/text/node.rs` after a gpui-kit upgrade. The link color itself comes from `crates/theme/src/lib.rs`, not this crate.
