# TASK-164 Timeline drift, find ring, and syntax theme

Group: H (writes `crates/timeline`)
Class: code

## Brief

Goal: Remove the 1 px per block drift of reply text, add the 2 px focus ring to the find field, and draw code with the selected syntax theme (audit items 9 and 11).

Change: paragraph pitch 48 against Electron 47 (line-box rounding), no focus ring, and a fixed `one` palette in `tool_view.rs` (`one_colors`) -> Electron pitch, a ring, and `bachuc_theme::syntax_colors`.

Design: the Fix Round 2 and 3 notes in [watchtower/archive/20261016-gpui-electron-parity/tasks/TASK-149-outcome.md](watchtower/archive/20261016-gpui-electron-parity/tasks/TASK-149-outcome.md), and the Electron markdown styles it cites.

Boundaries:

- Find the line-box rounding cause (GPUI rounds line heights to whole pixels) and match the Electron 47 px paragraph pitch; keep the 37 px heading-to-fence gap of Fix Round 3.
- The ring uses the `ring` token and the Electron focus width.
- Keep `TimelineView::new`, `find`, and `TimelineEvent`, and the per-flush work of [ADR-0039](watchtower/decisions/ADR-0039-incremental-timeline-sync-over-throttle-or-budget.md): the syntax colors are read when the style is built, not per flush.
- Planner call: the markdown body font size follows `content_font_size`, as Electron's content size does.

Files:

- [packages/desktop-gpui/crates/timeline](packages/desktop-gpui/crates/timeline) (`src`, `tests`)

Interfaces:

- Consumes: `bachuc_theme::SyntaxThemeId` (enum with the eight ids of `themes.ts`, `id() -> &'static str`, `label() -> &'static str`, `from_id(&str) -> Option<Self>`, `ALL`), `bachuc_theme::set_syntax_theme(id: SyntaxThemeId, cx: &mut gpui_kit::App)`, `bachuc_theme::syntax_theme(cx: &gpui_kit::App) -> SyntaxThemeId`, and `bachuc_theme::syntax_colors(cx: &gpui_kit::App) -> bachuc_theme::SyntaxColors` (one `Hsla` field per highlight role of `colors.ts`) from TASK-159
- Consumes: `bachuc_theme::set_content_font_size(px: f32, cx: &mut gpui_kit::App)` and `bachuc_theme::content_font_size(cx: &gpui_kit::App) -> gpui_kit::Pixels` from TASK-159
- Produces: the same `bachuc_timeline::TimelineView` and `TimelineEvent` API for TASK-167

Expected result:

- Paragraph pitch is 47 px at the default size; the fence gap stays 37 px.
- A syntax theme switch recolors code blocks at once.

Anti-goal: per-flush work stays in proportion to the change; tripwire: `incremental_rows_match_full_regroup` and every existing timeline test pass; read at completion.

## Verify

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-timeline` -> pass, including the existing 61 tests, a test that computes the paragraph pitch from the style constants (47), the existing `fence_gaps_match_the_web_stream`, and a test that two syntax themes give different code colors
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-timeline` -> exit 0
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> exit 0
