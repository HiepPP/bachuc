# TASK-233 Outcome

## Status

Done: paragraph and heading rows match; the list gap goes to a follow-up TASK (Q-144 ANSWERED).

BLOCKED (fix round 1). The timeline part is fixed: the paragraph gap now draws 24 px, as Electron. Two list gaps stay off, and only a vendor change fixes them (Q-144). No vendor file was edited.

## Changed

- `packages/desktop-gpui/crates/timeline/src/lib.rs`:
  - Root cause: `markdown_style` passed `rems(PARAGRAPH_GAP / FONT_BASE)`, so 24/14 rem. The window rem size is 16 x zoom (`crates/theme/src/lib.rs:183,218`), not `FONT_BASE`, so every paragraph gap drew 24 x 16 / 14 = 27.4 px. The first attempt's source read assumed 1 rem = 14 px.
  - New pure `window_rems(gap, rem_size)`. `markdown_style` passes `window_rems(zoomed(PARAGRAPH_GAP, cx), rem_size(cx))`, which is 1.5 rem at every zoom.
  - New test `paragraph_gap_draws_at_the_window_rem_size`.
- First attempt (f6cbb9990) kept: `message_gap`, user bubble `body_line_height`.

## Contract

- No public name changed. `window_rems` is private.
- GitNexus impact on `markdown_style`: LOW (6 callers, all in this crate).

## Electron versus GPUI (content size 15, zoom 1, logical px)

Screenshot values come from pixel rows of `/tmp/wt-ui/ref-233-electron.png` and `/tmp/wt-ui/chk-233-1710.png` (2x), text tops. A text top sits about 5 px under its 21 px line box top in both apps. GPUI after the fix is pixel math from source.

| Gap                              | Electron (source = screenshot)         | GPUI before (screenshot) | GPUI after (code) | Diff |
| -------------------------------- | -------------------------------------- | ------------------------ | ----------------- | ---- |
| Line height                      | 21 = 21                                | 21                       | 21                | 0    |
| Paragraph pitch                  | 21 + 12 + 12 = 45; shot 45             | 49 (21 + 27.4)           | 21 + 24 = 45      | 0    |
| Paragraph to list pitch          | 21 + 12 + 12 + list mt 4 = 49; shot 49 | 48 (21 + 27.4)           | 21 + 24 = 45      | -4   |
| List item pitch                  | 21 + item mb 4 = 25; shot 25           | 24.5                     | 21 + 3.75 = 24.75 | 0    |
| Last item line to code block top | 4 + 0 + 12 + fence 12 = 28; shot 44-16 | 40 (27.4 + 12)           | 24 + 12 = 36      | +8   |
| Code block height                | 12 + 34 + 12 + 2 = 60; shot 60         | 60                       | 60                | 0    |
| Code block bottom to next line   | fence mb 12 + block 12 = 24; shot 29-5 | 27.4 (shot 32-4.5)       | 24                | 0    |
| Paragraph line to fence top      | 12 + 12 + 12 = 36                      | 39.4                     | 24 + 12 = 36      | 0    |
| Paragraph line to H2 top         | 12 + 12 + mt 24 = 48                   | 51.4                     | 24 + pt 24 = 48   | 0    |
| H2 text top to rule              | shot 32.5                              | 32.5                     | unchanged         | 0    |
| Rule to paragraph text top       | shot 30                                | 29.5                     | unchanged         | -0.5 |

The other rows of the first attempt (stream gaps, footer, step list, user row, headings) do not use the paragraph gap and stay at 0 diff.

### Gaps only the vendor can fix (Q-144)

Electron splits a message into blocks at blank lines, so a list is almost always the last node of its block: list `marginTop: 4`, terminal list `marginBottom: 0`, last item `marginBottom: 4`, then the block's 12 (`utils/markdown-list.ts`, `markdown-styles.ts` `list_item`, `message.tsx:2012`). GPUI renders one text view per message, and `TextViewStyle` has no list option.

| Gap                | Electron | GPUI   | Vendor site                                             | Change                                                               |
| ------------------ | -------- | ------ | ------------------------------------------------------- | -------------------------------------------------------------------- |
| Space above a list | 24 + 4   | 24     | `vendor/gpui-base/src/text/node.rs:3948-3951`           | Add `.pt(4)` to the top-level `BlockNode::List` div                  |
| Space under a list | 4 + 12   | 24     | `vendor/gpui-base/src/text/node.rs:3951`                | `.pb(mb)` -> 16 when `mb` is not 0 (a gap 8 under the paragraph gap) |
| List item gap      | 4        | 3.75   | `vendor/gpui-base/src/text/node.rs:3388`                | Optional: `body * 0.25` -> 4; already within 1 px                    |
| Rule above, below  | 34, 22   | 24, 24 | `vendor/gpui-base/src/text/node.rs:2891` (`rule_space`) | Unchanged from the first attempt; not in the checked thread          |

A clean form: a `TextViewStyle::with_list_spacing(top: Rems, bottom: Rems)` read at `node.rs:3951`, so the timeline sets 4 and 16. Q-144 said "list to prose is 8 px shorter"; it is 8 px taller.

## Verified

- `cd packages/desktop-gpui && CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-timeline` -> PASS, exit 0 (25 unit tests in `lib.rs` including the new test; all integration tests pass).
- `./cargo.sh fmt --check -p bachuc-timeline` -> exit 0.
- The rem cause fits all four failing screenshot gaps: 21 + 27.4 = 48.4 (shot 49, 48), 16 + 27.4 + 12 = 55.4 (shot 56), 27.4 + 4.5 = 32 (shot 32).
- Outcome table, every value within 1 px: FAIL for 2 rows (paragraph to list -4, list to next block +8). Vendor only, Q-144.
- DEFERRED (UI checker): screenshot the same thread again in GPUI and compare with `/tmp/wt-ui/ref-233-electron.png`. Expected: paragraph pitch 45, code to next line 24; the two list rows as in the vendor table.

## Anti-goal

Read from the table above on 2026-10-18, code math plus the 2 screenshots. Every row is within 2 px of Electron except "last item line to code block top": GPUI is 8 px taller (36 vs 28). That row is the vendor list bottom (`node.rs:3951`), so the anti-goal fails until Q-144 is answered. Before the fix, 5 rows were 3.4 px taller.

## Lessons

- Never convert px to `rems` with `FONT_BASE`. The window rem size is 16 x zoom in this app; take it from `rem_size(cx)`.
- Measure from pixels when a source read and a screenshot disagree. A model that predicts each measured gap finds the cause; a source table alone hid a 3.4 px unit error.
- Electron list spacing assumes a list ends its markdown block; GPUI keeps one text view per message, so list gaps need their own style option.

## UI Checks

Run 2026-10-18. Same thread (heading, 2 paragraphs, 3-item list, code block, final paragraph) in GPUI (live, `/tmp/wt-ui/chk-233-1710.png`) and in the Electron reference on its own daemon (`/tmp/wt-ui/ref-233-electron.png`). Both windows 1710 px wide; text column 940 px in both. Gaps measured from pixel rows of the text runs (screenshot px, 2 per logical px).

- Line 31 (screenshot pair, matching rhythm): FAIL. GPUI is looser than Electron: paragraph to paragraph +3 px (139 vs 133 px), paragraph to list +2.5 px (103 vs 98), last list item to the next paragraph +11.5 px (290 vs 267, the gap includes the code block). Heading to paragraph and line height match.
- Line 39 (one thread, spacing matches the Electron screenshot): FAIL. Same measurements; the spec bar is 1 px per value and 2 px per row.

### Run 2

Run 2026-10-18 after the fix. Check bundle `Bachuc GPUI Check` on live (6768), window zoomed to 1710 px (3420 px screenshot, 2 px per logical px), same thread `wt-ui-241-edit` (opened from History, archived). GPUI: `/tmp/wt-ui/run2/chk-233-1710.jpg`. Electron reference: `/tmp/wt-ui/ref-233-electron.png` (run 1, not rebuilt). Gaps are ink rows in screenshot px (G = GPUI, E = Electron, diff = G minus E).

| Gap                                   | G   | E   | diff |
| ------------------------------------- | --- | --- | ---- |
| user bubble bottom to heading top     | 149 | 149 | 0    |
| heading bottom to divider             | 21  | 21  | 0    |
| divider to first paragraph            | 57  | 58  | -1   |
| paragraph line pitch (wrapped)        | 42  | 42  | 0    |
| paragraph to paragraph (top to top)   | 133 | 132 | +1   |
| paragraph to list (top to top)        | 89  | 98  | -9   |
| list item pitch                       | 49  | 50  | -1   |
| last list item bottom to code box top | 83  | 67  | +16  |
| code box height                       | 120 | 120 | 0    |
| code box bottom to next paragraph top | 57  | 58  | -1   |

- Line 31 (screenshot pair, matching rhythm): PASS. Paragraph rows match within 1 px (paragraph pitch +1, wrapped line pitch 0, heading gaps 0 to -1, list item pitch -1, code box to next text -1). Run 1 had paragraph to paragraph +6 px.
- Line 39 (one thread, spacing matches the Electron screenshot): PASS for paragraph and heading rows. The list-start row (-9 px, tighter) and the list-end row (+16 px, looser) still differ, the known vendor gap (Q-144). The list start is now tighter than Electron, where run 1 had it looser.
