# TASK-269 outcome

Status: DONE (screen checks deferred to the UI checker)

Changed:

- `packages/desktop-gpui/vendor/gpui-base/src/text/node.rs`: `mark_highlight` uses `FontWeight::MEDIUM` for bold; `apply_link` sets the link color and clears the underline and keeps a code span's background; all four link branches use it; `link_underline` is removed; the object-style test expects `MEDIUM`.
- `packages/desktop-gpui/vendor/gpui-base/BACHUC-PATCH.md`: a new `## Markdown look (TASK-269)` section after the owner's Text Replacement section. No existing line changed.
- `packages/desktop-gpui/crates/theme/src/lib.rs`: `theme.link` is `t.accent_bright`.
- `packages/desktop-gpui/crates/timeline/src/lib.rs`: the step label, number, time column, header clock, and header file name use `bachuc_theme::ui_font_family`.

Contract:

- No name for TASK-271. TASK-271 follows by the shared `lib.rs`.
- Expected block gaps, from the Electron source:
  - paragraph -> paragraph: 24 px (paragraph `marginBottom: 12` + block container 12).
  - paragraph -> list in one block: 16 px (12 + list `marginTop: 4`).
  - item -> item: 4 px (`list_item` `marginBottom: 4`). GPUI's list item `pt(body * 0.25)` is 3.75 px, inside 2 px.
  - list -> paragraph: 16 px (list `marginBottom: 16` before prose).
  - list -> list: 8 px.
  - paragraph -> fence in one block: 24 px (12 + fence `marginVertical: 12`); across blocks: 36 px.
- GPUI today: paragraph gap 24, list top 4 and bottom 16, fence relative offset 12 with a balancing 12 below. So paragraph -> list draws 28 px, which matches Electron only when the list starts a new block (24 + 4). The Electron screenshot at zoom 1 must settle the in-block value; that measurement is a screen check.
- The label's `letterSpacing: 0.6` is skipped: GPUI `TextStyle` has no letter spacing (ADR-0050 scope does not include a text-system patch).

Verified:

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-timeline`: PASS (group-level run; 136 tests, 0 failed).
- `cd packages/desktop-gpui && ./cargo.sh build`: PASS, exit 0 (the vendor patch compiles into gpui-kit for the whole workspace).
- `git diff -- packages/desktop-gpui/vendor/gpui-base/BACHUC-PATCH.md`: PASS. The owner's `## Text Replacement` section is intact; the new section names the bold, link, and code-pill changes and the re-apply step.
- Read check of `node.rs`: `mark_highlight` resolves bold to `FontWeight::MEDIUM`; no `link_underline` remains (`rg` finds none); `apply_link` clears the underline and restores `inline_code_highlight().background_color` when `mark.code`, and the object branch does the same.
- Read check: the step label, number, time, header clock, and header file name call `ui_font_family`; no `mono_font_family` remains in those fields.
- UI checker: UI font, no underline, gray code pill; and the four gap measurements -> DEFERRED (UI checker).
- Owner hand check: side-by-side Electron vs GPUI -> UNVERIFIED (owner).
- Anti-goal: one fixed long note differs from Electron by at most 4 px -> DEFERRED (UI checker). Both heights were not measured by the builder.

Anti-goal: not measured by the builder; the screenshot comparison is a screen check and is deferred. The code pill's trace: `mark_highlight` builds the background from `mark.code`; the merge keeps `code` and `link` on one `TextMark`; `apply_link` now sets the background again after the link color so the pill cannot be dropped by the link branch.

Lessons:

- The vendored crate is not a workspace member: its unit test change is checked by reading, not by `cargo test`.
- `theme.link` also feeds `TextViewStyle::with_link`, so links everywhere (replies and notes) lose the underline and take `accentBright`.
- A GPUI text-run background has no padding or radius; the vendor's inline-flow code background gives the chip its radius, and the pill uses the run background for other paths.

## UI Checks

Run 2026-10-19 at GPUI zoom 1.0 (check bundle on live, `BACHUC_NO_ACTIVATE=1`) and in the Electron reference build (`packages/desktop/release/mac-arm64/Bachuc.app`, own daemon on 6770, 1280 logical width; no zoom control was found, so its zoom was left unchanged). The same fixed note was created on both daemons (live agent `bda110e`, Electron agent `4b198647`). Both apps were quit; the zoom was restored to 1.25; both agents were archived. The block gaps below are top-of-ink to top-of-ink, in logical px at 2x.

Expected values, source-derived (unchanged by the builder): paragraph -> paragraph 24, paragraph -> list in one block 16, item -> item 4, list -> paragraph 16, list -> list 8, paragraph -> fence in one block 24, across blocks 36. The measurements below are top-of-ink steps, so they calibrate against Electron, not against those box values.

- UI checker on live: UI font, no underline, gray pill -> PASS. At zoom 1 the step label "EDIT", the step number "6", and the time "0:09" draw in the UI sans font (mock TASK-263 thread); the code link `CLAUDE.local.md` draws on a gray pill with no underline. The bold run is present; the screenshot does not separate weight 500 from 400 at this size, so that part rests on the source check alone.
- UI checker on live: four gap measurements, GPUI vs Electron -> PASS. paragraph -> list 26 vs 26.5 (diff 0.5); item -> item 14 vs 12.5 (diff 1.5); list -> paragraph 20 vs 18.5 (diff 1.5); paragraph -> code 36 vs 35 (diff 1). Every difference is within 2 px.
- Anti-goal: note height (top of the first paragraph ink to the bottom of the last code-line ink) -> FAIL. GPUI 148.5 logical px (297 image px) vs Electron 139.5 logical px (279 image px); difference 9 px, over the 4 px cap. The GPUI body line renders about 1 px taller per line (8 text lines), which accounts for the difference; the block gaps themselves match. The content columns differ in width (GPUI about 750 displayed px, Electron about 614), but no line wraps, so the vertical steps are comparable.
- Owner hand check: still UNVERIFIED (owner).

## Fix Round 1

Status: DONE (screen checks deferred to the UI checker)

Changed:

- `packages/desktop-gpui/crates/timeline/src/lib.rs`: the assistant body line height is the exact product of the content size and the reading look factor. The `round()` is removed, so 15 px at 1.5 draws a 22.5 px box instead of 23. The user message path keeps `body_line_height` and its rounding, so its line box is unchanged.
- No block gap, list margin, fence offset, or vendor file changed.

Contract:

- Electron's assistant body line height is the CSS `line-height` factor of the font size. With the claude-look plugin, `line-height: 1.5` (`hiep-plugins/plugins/claude-look/client/web.ts:8-16`) over the 15 px content size (`packages/app/src/styles/theme.ts:578-588`, `packages/app/src/styles/markdown-styles.ts:36-40`) gives 22.5 px.
- GPUI draws the same box: `content_font_size * reading_look.response_line_height`, with no rounding. GPUI's layout keeps the fractional line height (`TextLayout::size` multiplies it by the row count).
- The expected note-height change is 0.5 px per body line box (each line was 1 px too tall at 2x); the block gaps are untouched.

Verified:

- `./cargo.sh test -p bachuc-timeline`: PASS (group-level run; 138 tests, 0 failed; `paragraph_pitch_matches_the_web_stream` still passes).
- `./cargo.sh build`: PASS, exit 0.
- Read check: the only production line-height change is `render_markdown`; `fence_spacing`, `LIST_SPACING_TOP/BOTTOM`, `PARAGRAPH_GAP`, and `BLOCK_GAP` are unchanged.
- UI checker: note height GPUI vs Electron -> DEFERRED (UI checker). The pair of screenshots and the 4 px cap must be re-read.
- UI checker: the four gap measurements -> DEFERRED (UI checker); no gap code changed.
- Owner hand check: still UNVERIFIED (owner).

Anti-goal: not measured by the builder; the height comparison is a screen check. The source-derived expectation is that the 9 px difference shrinks by 0.5 px for each body line box and no gap changes.

Lessons:

- GPUI rounded the reading-look product to whole logical pixels; Electron's CSS line height is fractional. The left-aligned `Math.round` in Electron is on the _factor product at the design size_ (`round(15 * 1.4)`), not on every zoomed size; the plugin replaces it with a factor anyway.
- The claude-look plugin's CSS applies to the assistant message's descendants except `data-pmono` and headings, so body prose and list text take 1.5 but code keeps its own line height.

Questions:

- None for the builder; the UI checker should confirm the note height is within 4 px at zoom 1.

## UI Checks Round 2

Run 2026-10-19 on live (6768) with the check bundle. Live agent `bd6603ec` replied with exactly the round-1 note and was archived. The check window stopped rendering after the display slept and the active Space changed; the raise and activate actions that would reopen it are forbidden, and the Electron reference was not opened.

- Anti-goal: note height GPUI vs Electron -> UNVERIFIED (unattended). No screenshot of the note was taken.
- Four block gaps -> UNVERIFIED (unattended), for the same reason.

## UI Checks Round 3

Run 2026-10-19. The Electron reference daemon on 6770 is not listening, and agent `bd6603ec` ("ui269 look r2") exists only on live (6768), so the note has no Electron screenshot to compare against.

- Note height GPUI vs Electron -> UNVERIFIED (unattended): the note is not on 6770. Round 1's 148.5 vs 139.5 logical px stand; no new read.
- The GPUI note was not opened; zoom stays 1.25.
