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
