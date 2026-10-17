# TASK-164 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/timeline/src/lib.rs`: paragraph gap `PARAGRAPH_GAP` (`BLOCK_GAP` less `LINE_BOX_ROUNDING`), body line box `body_line_height`, markdown body size from `content_font_size`, fence highlighter built from `syntax_colors`, `Diff.runs` keyed by `SyntaxColors`, `render` passes the window to the find bar; tests `paragraph_pitch_matches_the_web_stream` and `syntax_themes_give_different_code_colors`.
- `packages/desktop-gpui/crates/timeline/src/tool_view.rs`: `one_colors` and `OneColors` removed; `code_styles` takes `bachuc_theme::SyntaxColors` and resolves through `bachuc_explorer::syntax::ThemeSyntax`; tool and diff runs read `syntax_colors(cx)`.
- `packages/desktop-gpui/crates/timeline/src/find_bar.rs`: `focus_ring` (2 px `ring` border, 1 px offset) shown while the find input has focus; focus in and out notify the view; test `focus_ring_is_two_pixels_wide_outside_the_field`.

## Contract

- `TimelineView::new`, `find`, and `TimelineEvent` are unchanged. `render_find_bar` and `code_styles` are crate-private and changed signature.
- Paragraph pitch at content 15 is `round(15 * 1.618)` = 24 plus `PARAGRAPH_GAP` 23 = 47. The line box was 24 and the gap 24 (pitch 48). The text view's line height is now explicit on the markdown wrapper, so the test and the layout use one number. Fence gap constants and `fence_spacing` are untouched, so heading to fence stays 37 px.
- The markdown body and heading sizes follow `bachuc_theme::content_font_size`. The user bubble still uses `ui_font_size(FONT_CONTENT)`.
- Syntax colors are read in `markdown_style` (cached) and per tool detail. The `Theme` global observer already clears `markdown_style` and `syntax_cache`, and `set_syntax_theme` writes that global, so a theme switch recolors code at once. No per-flush work was added.
- Cross-group dependency: `bachuc_explorer::syntax::ThemeSyntax` is public in group F's working tree (not yet committed). If group F renames it, `tool_view.rs` needs the new name.

## Verified

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-timeline` -> 64 tests pass (20 unit, 8 cards, 16 grouping, 11 links, 9 tools): the 61 existing tests plus `paragraph_pitch_matches_the_web_stream` (47), `syntax_themes_give_different_code_colors`, and `focus_ring_is_two_pixels_wide_outside_the_field`. `fence_gaps_match_the_web_stream`, `fence_colors_follow_the_one_theme`, and `incremental_rows_match_full_regroup` pass.
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-timeline` -> exit 0.
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> Finished. The first try failed in `crates/workspace` (group F, missing `menu_item` and `logo`); the retry after a 4 minute wait passed.
- Pitch 47 px, the 2 px ring, and recolor on a theme switch on screen -> UNVERIFIED (unattended): code tests only. Tests cover the style constants only.
- GitNexus `impact` for `code_styles`: target not found (index behind). Text search shows all callers inside `bachuc-timeline`.

## Anti-goal

`incremental_rows_match_full_regroup` and every existing timeline test passed at completion, 2026-10-16, source: `cargo test -p bachuc-timeline`. The grouper and `sync` are not edited.

## Questions

- The spec says the ring uses the `ring` token. Electron's `controlActive` in `packages/app/src/components/ui/control-geometry.ts` colors the outline with `accent` and the border with `borderAccent`. The build follows the spec (`ring`). If the owner wants the Electron colors, change the `focus_ring(t.ring)` argument to `t.accent`.
- `PARAGRAPH_GAP` is 23 on the premise that Electron's pitch is 47 with a 24 px line box. Re-measure on a screenshot when a window check is allowed.

## Lessons

- GPUI has no outline, so a focus ring is a border on an absolute box that reaches past the field by offset plus width. A box with no handler takes no pointer events.
- A view that draws from focus state needs `cx.on_focus_in` and `cx.on_focus_out` on the input's handle, because the input's own redraw does not notify the parent view.
