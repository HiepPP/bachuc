# TASK-243 Outcome

## Status

DONE (code checks pass; UI checker lines DEFERRED).

## Changed

- `packages/desktop-gpui/vendor/gpui-base/src/text/style.rs`: field `list_spacing: Option<(Rems, Rems)>` (default `None`), `with_list_spacing`, `list_spacing`, and `PartialEq`.
- `packages/desktop-gpui/vendor/gpui-base/src/text/node.rs`: the top-level `BlockNode::List` takes `pt(top)` and `pb(bottom)` in place of `pb(mb)`; `bottom` applies only when `mb` is not 0.
- `packages/desktop-gpui/vendor/gpui-base/src/text/document.rs` (not in the spec `Files:`): new `NodeRenderOptions.in_blockquote`, set by the blockquote branch. Needed because `depth` stays 0 inside a blockquote, so `depth == 0` alone would also match a list in a blockquote.
- `packages/desktop-gpui/vendor/gpui-base/BACHUC-PATCH.md`: a section for the patch (ADR-0044).
- `packages/desktop-gpui/crates/timeline/src/lib.rs`: constants `LIST_SPACING_TOP` (4) and `LIST_SPACING_BOTTOM` (16); the style sets them zoomed through `window_rems(zoomed(X, cx), rem_size(cx))`.

## Contract

- `TextViewStyle::with_list_spacing(self, top: Rems, bottom: Rems) -> Self` and `TextViewStyle::list_spacing(&self) -> Option<(Rems, Rems)>`, as TASK-246 consumes them. The timeline already calls them zoomed.
- A top-level list (`depth == 0`, not in a blockquote) gets `pt(top)`; `pb(bottom)` replaces `pb(mb)` only when `mb` is not 0 (a last list keeps 0). A nested list and a list in a blockquote keep today's spacing.

## Verified

- Group-level run (one cargo call for TASK-243 to TASK-246): `./cargo.sh test -p bachuc-timeline -p bachuc-composer -p bachuc-app` -> pass.
- Group-level: `./cargo.sh build -p bachuc-app` -> builds; no warning in the vendor crate or in changed files.
- Group-level: `./cargo.sh fmt --check -p bachuc-timeline`, `-p bachuc-composer`, `-p bachuc-app` -> exit 0.

## Anti-goal

DEFERRED (UI checker): the other rows of the TASK-233 gap table within 1 px of run 2. All four `UI checker:` lines of the spec are DEFERRED (UI checker).

## Lessons

- A list inside a blockquote is still `depth == 0`; `NodeRenderOptions.in_blockquote` tells them apart.
- The TASK-233 outcome moved to `watchtower/archive/20261018-gpui-polish-and-fixes/tasks/`; the spec link is stale but the gap table is not needed for the code.
