# TASK-207 outcome

## Status

DONE (code), partly. The `~` path rule is done. The quote style and the step label letter spacing cannot be set inside the TASK boundaries; the spec's fallback ("set what it can and record the rest") applies, and nothing quote-only can be set. The spacing measure and the screen checks are DEFERRED (UI checker).

## Changed

- `packages/desktop-gpui/plugins/edit-diffs/src/edit_item.rs`:
  - `display_path(file_path, cwd)` keeps its signature. It reads `HOME` once (`OnceLock`) and calls the new pure `display_path_in(file_path, cwd, home)`.
  - Order: strip `cwd/` or `/private` + `cwd/` as before; else a path under `home/` shows as `~/...`; else the full path.
  - The home match needs the `/` boundary, so `/Users/hiepx/a.rs` stays full. An empty or `/` home is ignored.
  - New test `display_path_shows_the_home_folder_as_a_tilde_outside_the_agent_folder`: inside cwd, outside cwd under home, outside home, a sibling of home, empty cwd, and no home.
- `packages/desktop-gpui/crates/timeline/src/lib.rs`: not changed (see Verified).

## Contract

- No Produces or Consumes lines. Adds private `fn display_path_in(file_path: &str, cwd: Option<&str>, home: Option<&str>) -> String` in `edit_item.rs`. The only caller of `display_path` is `row.rs:504` (GitNexus impact LOW, confirmed with rg).

## Verified

- `CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-plugin-edit-diffs` -> pass (37 + 1 tests, with the new path test). Log: `/tmp/t207-ed.txt`.
- `CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-timeline` -> pass (no code change). Log: `/tmp/t207-tl.txt`.
- `./cargo.sh fmt --check -p bachuc-plugin-edit-diffs` -> exit 0.
- `./cargo.sh metadata --format-version 1 > /dev/null` -> exit 0.
- Static read, quote: `TextViewStyle` (`vendor/gpui-base/src/text/style.rs:14`) has no blockquote field. The blockquote render (`vendor/gpui-base/src/text/node.rs:3916`) hard-codes `border_l_2`, padding `0.9 em`, no background, and takes its colors from the shared `muted_foreground` and `border`. `border` also draws tables (`node.rs:3610`, `3741`, `3799`), the hr (`node.rs:4033`), and the fence (`node.rs:2231`). `muted_foreground` also draws ordered list numbers and task boxes (`node.rs:3305`, `3341`, `3361`). Changing either would regress those, so no style is set. Electron's quote (`markdown-styles.ts:315-327`) needs a vendor patch, which the spec forbids (ADR-0044 boundary).
- Static read, letter spacing: gpui-pre 0.3.9 `TextStyle` has no letter spacing (rg for `letter_spacing` in its `src` finds nothing). The step label keeps no tracking.
- Static read, spacing: no gap is measured, so `BODY_LINE_HEIGHT`, `PARAGRAPH_GAP`, and `BLOCK_GAP` are not changed, per the boundary.
- UI check: prose line pitch, paragraph gaps, and list gaps beside Electron -> DEFERRED (UI checker).
- UI check: quote matches Electron -> DEFERRED (UI checker); expected to differ, see the quote read above.
- UI check: Edit row shows `~/.claude/x.md` -> DEFERRED (UI checker).
- Owner hand test: long thread reads like Electron -> DEFERRED (owner).
- Anti-goal: CPU under 10% during the mock stream -> DEFERRED (UI checker). The change adds no per-frame work in the timeline; `display_path` reads `HOME` once.

## Lessons

- The vendored text view styles a blockquote only through the shared `muted_foreground` and `border` colors. An Electron-like quote needs a `TextViewStyle::with_blockquote(StyleRefinement)` field in `vendor/gpui-base`, which is an ADR-0044 patch decision for the lead.
- gpui-pre 0.3.9 has no letter spacing, so any Electron `letterSpacing` stays unmatched until GPUI adds it.
- Empty cwd: the `~` rule applies, because no path is inside an empty cwd.
