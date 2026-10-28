# TASK-323 Lead label colors in the vendored text view

Group: A (vendored `gpui-base` text files only; ordering for TASK-324, no shared file)
Class: risky

## Brief

Goal: A caller can give the text view a list of labels with a text and a chip color, and a body paragraph that starts with one of the labels draws that label as a tint chip.

Change: paragraph text gets marks only from the markdown -> a body paragraph whose rendered text starts with a style label draws the label as a chip (text color, background, medium weight, the inline code chip shape) through the inline flow path.

Boundaries:

- The vendored text view draws every assistant message, so this is a hot render path. With an empty label list, the output must stay exactly as today.
- Follows [ADR-0050](watchtower/decisions/ADR-0050-electron-markdown-look-over-gpui-link-underline-and-bold.md) (strong is `FontWeight::MEDIUM`) and [ADR-0076](watchtower/decisions/ADR-0076-semantic-tint-chips-for-answer-labels.md) (tint chip with medium weight, also for a label the agent did not write in bold; supersedes ADR-0075).
- The vendor knows no label words. The list comes only from the caller.
- Match rule: the rendered paragraph text starts with the label string exactly (case-sensitive, the colon is part of the label), and the next character is whitespace or the end of the text. "Fix:" matches "Fix: x" and the rendered text of `**Fix:** x`; it does not match "Fixes: x", "fix: x", or "Fix:x".
- Scope: body paragraphs, including the paragraphs inside list items and blockquotes. Headings and table cells are not matched.
- Trap: `mark_highlight` in `node.rs` has three paragraph highlight paths: `plain_render` (around line 1402), the image branch of `Paragraph::render` (around 2646), and `inline_flow_items` (around 2831). A paragraph with an inline code chip renders through `inline_flow_items`. Apply the label highlight in all three. Do not change the custom-node `object_style` call (around 2755) or `inline_highlights` (around 2496): its only caller is the table cell measure (around 3111), and table cells get no label.
- Trap: `**Fix:** x` splits into several `InlineNode` children, and `inline_flow_items` resets its offset at each image or custom node. Match the label on the concatenated paragraph text, then map the range into each chunk.
- Trap: headings (around 3963) and table cells (around 3815) render through the same `Paragraph::render` as body paragraphs (around 3912), which has no block-kind parameter. Planner call: thread a `bool` from the body call site through `render` into `plain_render` and `inline_flow_items`. The `plain_render` cache stays correct because each `Paragraph` has one role.
- Trap: `PartialEq for TextViewStyle` in `style.rs` (around lines 36-55) lists every field. Add the label list, or a theme change does not refresh the paragraph render cache (`plain_render` compares the style). Also set the field in `from_colors` (around line 77), and import `SharedString`.
- Trap: the stream fade-in (`fade_highlights` in `inline.rs`) merges after the cached highlights and fades background and decoration colors, not `color`. Add the label highlight with the cached highlights, before the fade, so a streamed label keeps its color.
- Planner call: when two labels match, the longest one wins.
- Planner call (2026-10-28, chip): the inline flow draws a chip for any highlight with a set `font_size_scale`, so the label highlight sets `font_size_scale: Some(1.)`. `text_size_ranges` in `inline.rs` keys its ranges on the `Option`, so a body-size chip does not merge with the plain text next to it. A paragraph with a label always renders through the inline flow, because the plain path has no chip padding or radius.
- Trap: `gpui-base` is not a workspace member, so `cargo test -p gpui-base` fails ("requires dev-dependencies and is not a member of the workspace"). Keep the pure matcher `match_lead_label` public in `style.rs` and test it from the timeline crate, as the text replacement rule is tested from `crates/composer`.
- Planner call: a label written as inline code, such as `` `Fix:` x ``, still gets the label color on top of the code chip.
- Planner call: the chip layers on top of the marks under it (`with_lead_label`), because `combine_highlights` sorts equal start points unstably; a bold or code label keeps one medium weight and gets the chip colors.

How:

- Add `lead_labels: Arc<[LeadLabel]>` to `TextViewStyle`, with the builder `with_lead_labels` and `PartialEq`.
- Add `match_lead_label` in `style.rs`; in `node.rs`, `Paragraph::lead_label` runs it over the text runs before the first custom node or image, and `lead_label_highlight` plus `with_lead_label` layer the chip over the marks in `inline_flow_items`.
- Thread the body-paragraph `bool` as in the Boundaries planner call, so headings and table cells skip the helper.

Files:

- [packages/desktop-gpui/vendor/gpui-base/src/text/style.rs](packages/desktop-gpui/vendor/gpui-base/src/text/style.rs) (`LeadLabel`, `match_lead_label`, field, builder, `PartialEq`)
- [packages/desktop-gpui/vendor/gpui-base/src/text/node.rs](packages/desktop-gpui/vendor/gpui-base/src/text/node.rs) (label lookup, chip highlight, inline flow path)
- [packages/desktop-gpui/vendor/gpui-base/src/text/inline.rs](packages/desktop-gpui/vendor/gpui-base/src/text/inline.rs) (`text_size_ranges` keys on the scale `Option`)
- [packages/desktop-gpui/vendor/gpui-base/BACHUC-PATCH.md](packages/desktop-gpui/vendor/gpui-base/BACHUC-PATCH.md) (patch note)

Interfaces:

- Produces: `pub struct LeadLabel { pub text: SharedString, pub color: Hsla, pub background: Hsla }` for TASK-324
- Produces: `TextViewStyle::with_lead_labels(self, labels: Arc<[LeadLabel]>) -> Self` for TASK-324
- Produces: `match_lead_label(chunks: impl Iterator<Item = &str> + Clone, labels: &[LeadLabel]) -> Option<&LeadLabel>` for TASK-324

Expected result:

- WHEN a paragraph renders as "Fix: set the flag" and the style holds ("Fix:", c) THE paragraph SHALL highlight bytes 0..4 with color c and `FontWeight::MEDIUM`, and the rest of the text SHALL have no label highlight.
- WHEN the markdown is `**Fix:** set the flag` THE label range SHALL have color c and one medium weight.
- WHEN a list item paragraph starts with "Did:" and the style holds ("Did:", c) THE label SHALL get color c.
- IF the text is "Fixes: x", "fix: x", or "Fix:x" THEN THE paragraph SHALL have no label highlight.
- IF a heading or a table cell starts with "Fix:" THEN THE heading or cell SHALL have no label highlight.
- WHEN a paragraph that starts with a label holds an inline code span THE label SHALL get its color in the inline flow path too.
- WHEN the whole paragraph text is "Fix:" THE label SHALL get its color.
- WHILE the label list is empty, THE highlights of every paragraph SHALL equal today's highlights.

Red tests:

File: [packages/desktop-gpui/vendor/gpui-base/src/text/node.rs](packages/desktop-gpui/vendor/gpui-base/src/text/node.rs) (the existing tests module, next to `code_block_highlights_are_cached_by_highlighter_identity`)

The builder adds one test per EARS line above. Test the match rule (prefix, case, colon, next character, end of text, longest wins) on the pure helper `lead_label`. Test the render paths in the style of the existing tests around lines 4303-4331 and 4580: a `TestApp` window, `cx.set_global(crate::Theme::default())`, `markdown::parse`, a `NodeContext` whose `style` holds the labels, then assert on the `plain_render` highlights and on the `inline_flow_items` items, with their `color` and `font_weight`. Test the heading and table-cell skip by calling `plain_render` with the body flag off. The empty-list test compares the highlights with and without `with_lead_labels(Arc::from([]))`.

Anti-goal: the `bachuc-timeline` tests keep their pass count (plus the new tests) with 0 edited expected values; tripwire; read before the change and at completion.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh check --workspace` -> builds.
- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-timeline` -> pass, with the matcher tests of TASK-324; 0 edited expected values (anti-goal).
- `git diff -U0 -- packages/desktop-gpui/vendor/gpui-base/src/text | rg '^-\s+assert'` -> no match.
- `cd packages/desktop-gpui && ./cargo.sh clippy -p bachuc-timeline` -> no new warning in the changed files.
- `rg -n "Lead label chips" packages/desktop-gpui/vendor/gpui-base/BACHUC-PATCH.md` -> one section, so a gpui-kit upgrade re-applies the patch.
- Screen check: TASK-324 covers the render paths (plain, bold, list item, inline code, heading, table cell).
