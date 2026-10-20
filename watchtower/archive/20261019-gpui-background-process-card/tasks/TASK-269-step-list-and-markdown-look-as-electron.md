# TASK-269 Step list and markdown look as Electron

Group: B (the `bachuc-timeline` crate and the vendored text view; shares `lib.rs` with TASK-265, TASK-266, TASK-267, and TASK-271)
Class: risky

## Brief

Goal: A step list and its notes look the same in GPUI and Electron: the step label, number, and time font, the bold weight, inline code links, and the space between blocks.

Change: the GPUI step list and markdown differ from Electron in 4 places -> each takes the Electron value, read from the Electron source.

Design: the step list in [packages/app/src/tool-calls/detail-level/turn/view.tsx](packages/app/src/tool-calls/detail-level/turn/view.tsx), the markdown styles in [packages/app/src/styles/markdown-styles.ts](packages/app/src/styles/markdown-styles.ts), list margins in [packages/app/src/utils/markdown-list.ts](packages/app/src/utils/markdown-list.ts), and code links in [packages/app/src/assistant-file-links/link.tsx](packages/app/src/assistant-file-links/link.tsx). Read them before any change.

Boundaries:

- Follows [ADR-0050](watchtower/decisions/ADR-0050-electron-markdown-look-over-gpui-link-underline-and-bold.md): the Electron look applies to all GPUI assistant markdown, not only step notes.
- The turn header is not sticky in Electron either (`view.tsx:261-327`, `:585`). The owner's screenshots differ only by scroll position. Do not add a sticky header.
- Step list font: the Electron styles name the mono font, but the web build draws all text in the UI font unless a parent carries `data-pmono` (`packages/app/src/appearance/apply-root-font.web.ts:16`). The step list has no such mark. So use the UI font (`bachuc_theme::ui_font_family`, `crates/theme/src/lib.rs:322`) for the step label (`lib.rs:2481-2487`), the number (`lib.rs:2470`), the time column (`lib.rs:2498-2506`), the header clock (`lib.rs:2392`), and the header file name (`lib.rs:2358`); Electron draws `clock` and `fileName` the same way (`view.tsx:625`, `:652`).
- Planner call: skip the label's `letterSpacing: 0.6` (`view.tsx:727-733`). GPUI `TextStyle` has no letter spacing (gpui-pre 0.3.9 `style.rs:438-483`), and a text system patch is out of scope.
- Bold: Electron `strong` is weight 500 (`markdown-styles.ts:138-141`). The vendored view hard-codes `FontWeight::BOLD` (`packages/desktop-gpui/vendor/gpui-base/src/text/node.rs:2393-2395`). Make it 500, and update the `node.rs` test at `:4280` that asserts `BOLD`. The vendored crate is not a workspace member, so `cargo test` does not run that test; check it by reading.
- Links: Electron `link` has no underline and the color `accentBright` (`markdown-styles.ts:154-161`). The vendored view always draws a 1 px underline: `link_underline` (`node.rs:2841-2847`) is called at `node.rs:1405`, `:2596-2598`, `:2714`, and `:2778-2780`. Remove every call. GPUI sets `theme.link` to `t.accent` (`crates/theme/src/lib.rs:535`); set it to `t.accent_bright`, which holds the same values as Electron's `accentBright`.
- Code inside a link: Electron draws `[code_inline, link]`, a `surface2` pill with `paddingHorizontal: 4`, `paddingVertical: 2`, and `borderRadius: md` in the link color (`markdown-styles.ts:176-186`). GPUI sets the code background (`lib.rs:1896-1899`) but the screenshot shows none. `merge_children_with_mark` (`vendor/gpui-base/src/text/format/markdown.rs:839`) merges the link and code marks into one `TextMark`, `mark_highlight` (`node.rs:2411-2414`) sets the background, and the link branch (`node.rs:2597`) changes only color and underline. So the loss is in `combine_highlights` or the inline background paint. Trace that first.
- Planner call: a text-run background cannot take padding or a radius. Keep the background on the run; add padding or a radius only if the vendored inline paint already supports them.
- Block spacing: the Electron source alone does not settle the gaps. A paragraph has a 12 px margin and the markdown container adds its own gap, so a paragraph-to-paragraph gap of 24 already matches GPUI. Before any change, write the expected value of each gap in the outcome, from the Electron source and from a screenshot of Electron (the release app, read only) at zoom 1. Then measure GPUI and fix it. The sources:
  - Paragraph: `marginBottom: 12` (`markdown-styles.ts:50-60`). Note blocks get 12 below each except the last (`packages/app/src/components/message.tsx:2012`).
  - List: `marginTop: 4`, and 16 below before prose, 8 before another list, 0 when last or nested (`markdown-list.ts:15-16`). List item: `marginBottom: 4` (`markdown-styles.ts:276-281`).
  - Fence: `marginVertical: 12` (`markdown-styles.ts:209`).
  - GPUI today: `PARAGRAPH_GAP` and `BLOCK_GAP` of 24 (`lib.rs:155`, `:161`, used at `:1888`), `LIST_SPACING_TOP` 4 and `LIST_SPACING_BOTTOM` 16 (`lib.rs:163-164`), and `fence_spacing` (`lib.rs:2933-2938`).
  - The owner's screenshots of one thread show GPUI about 18 px taller from a paragraph to its list, and about 9 px taller from a paragraph to a code block.
- Vendor rules, per [ADR-0044](watchtower/decisions/ADR-0044-vendored-gpui-base-patch-over-git-fork-or-log.md): record each vendor change in [packages/desktop-gpui/vendor/gpui-base/BACHUC-PATCH.md](packages/desktop-gpui/vendor/gpui-base/BACHUC-PATCH.md). That file and other vendor files already hold the owner's uncommitted Text Replacement work. Keep it, and stage only this TASK's hunks.
- Planner call: when a GPUI default and an Electron value differ and the Electron source is unclear, the screen measurement of the Electron app wins.

How:

- Read the Electron files above and write the expected value of each item in the outcome.
- Patch `node.rs` for the bold weight, the link underline, and the code pill in links. Record it in `BACHUC-PATCH.md`.
- Switch the step label, number, and time to the UI font, and add the label tracking.
- Fix the block gaps in `lib.rs` (or the vendored list spacing) until the measured gaps match.

Files:

- [packages/desktop-gpui/vendor/gpui-base/src/text/node.rs](packages/desktop-gpui/vendor/gpui-base/src/text/node.rs) (bold weight, link underline, code pill in a link)
- [packages/desktop-gpui/vendor/gpui-base/BACHUC-PATCH.md](packages/desktop-gpui/vendor/gpui-base/BACHUC-PATCH.md) (a new section for these changes)
- [packages/desktop-gpui/crates/timeline/src/lib.rs](packages/desktop-gpui/crates/timeline/src/lib.rs) (step fonts and tracking, block gaps, markdown style)
- [packages/desktop-gpui/crates/timeline/src/links.rs](packages/desktop-gpui/crates/timeline/src/links.rs) (only if the code pill needs a change in how code links are built)
- [packages/desktop-gpui/crates/theme/src/lib.rs](packages/desktop-gpui/crates/theme/src/lib.rs) (`theme.link` from `t.accent` to `t.accent_bright`)
- [packages/desktop-gpui/vendor/gpui-base/src/text/format/markdown.rs](packages/desktop-gpui/vendor/gpui-base/src/text/format/markdown.rs) (only if the code pill loss starts in the mark merge)

Interfaces:

- Consumes: none. TASK-267 is a start order only, because both write `lib.rs`.
- Produces: no name for TASK-271. TASK-271 follows by the shared `lib.rs`.

Expected result:

- The step label "EXPLORE", the step number, the time "0:47", the header clock, and the header file name use the UI font, as in Electron.
- A `**bold**` run draws at weight 500.
- A file link in code, such as `CLAUDE.local.md`, is accent mono text on a gray pill with no underline.
- A normal link has the `accent_bright` color and no underline.
- Each measured block gap is within 2 px of its Electron value.

Anti-goal: the height of one fixed long note (the owner's "Regression Guard" note, or any note with a paragraph, a list, and a code block) differs from Electron by at most 4 px; tripwire; read from screenshots of the same thread in GPUI and Electron at zoom 1 and the same column width, before changes and at completion.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-timeline` -> every test passes.
- `cd packages/desktop-gpui && ./cargo.sh build` -> the workspace builds with the vendor patch.
- `git diff -- packages/desktop-gpui/vendor/gpui-base/BACHUC-PATCH.md` -> the owner's Text Replacement section is still there, and a new section names each `node.rs` change.
- Read check of `node.rs`: bold resolves to weight 500, no link underline is drawn, and a code span inside a link keeps its background.
- UI checker on live (screen check): open a thread whose note has a bold run, a code link, a list, and a code block. Raise the GPUI window and take a screenshot at zoom 1. Expect the UI font on the step label, number, and time; no underline on links; a gray pill on the code link.
- UI checker on live (screen check): measure the paragraph-to-list, item-to-item, list-to-paragraph, and paragraph-to-code gaps -> each within 2 px of the expected value the outcome lists.
- Owner hand check: compare the same thread in Electron and GPUI side by side. Mark it UNVERIFIED (owner) in the outcome until Hiep checks it.
- Anti-goal: the note height in GPUI and Electron, from the two screenshots -> differs by at most 4 px; record both values.
