# TASK-233 Outcome

## Status

DONE, with UI checker lines DEFERRED. The GPUI timeline already used the Electron values for almost every spacing constant. One real difference was fixed. Three differences sit in the vendored text view, outside this TASK's Files.

## Changed

- `packages/desktop-gpui/crates/timeline/src/lib.rs`:
  - The user bubble line height now uses `body_line_height` (`round(size * 1.4)`, as Electron's `Math.round(fontSize.content * 1.4)`). It was the unrounded `size * 1.4`. `USER_LINE_HEIGHT` is removed.
  - The `space_below` rules moved into the pure function `message_gap(is_user: bool, below: Below) -> f32`. `space_below` calls it. The values did not change.
  - New test `message_gaps_match_the_web_stream`.

## Contract

- No public name changed. `message_gap` and `Below` are private.
- GitNexus reported `space_below` as CRITICAL (22 transitive callers, all in this crate). The change is a pure extraction, so no caller sees a different value.

## Electron versus GPUI (content size 15)

| Value                               | Electron source                             | Electron          | GPUI                            | Diff |
| ----------------------------------- | ------------------------------------------- | ----------------- | ------------------------------- | ---- |
| Body line height                    | `markdown-styles.ts` body                   | 21                | `body_line_height` 21           | 0    |
| User bubble line height             | `message.tsx` user text                     | 21                | 21 (was `size*1.4`, same at 15) | 0    |
| Paragraph to paragraph              | paragraph mb 12 + block mb 12               | 24                | `PARAGRAPH_GAP` 24              | 0    |
| Paragraph pitch (1 line)            | 21 + 24                                     | 45                | 45 (test)                       | 0    |
| H1/H2 top                           | mt 24 (flex columns add the 24 above)       | 24                | `heading_style` pt 24           | 0    |
| H1/H2 bottom                        | mb 12 + pb 8 + block 12                     | 24 below the rule | mb 24, pb 8                     | 0    |
| H3/H4 top, bottom                   | mt 16, mb 8 + block 12                      | 16, 20            | pt 16, pb 20                    | 0    |
| H5/H6 top, bottom                   | mt 12, mb 4 + block 12                      | 12, 16            | pt 12, pb 16                    | 0    |
| Heading size and line height        | `round(content*tier/14)`, `round(size*1.3)` | rounded           | same rule                       | 0    |
| Code block padding, line height     | padding 12, mono 12, 1.45                   | 12, 17            | `SPACE_3`, `round(code*1.45)`   | 0    |
| Fence margin above, below           | `fence` marginVertical 12                   | 12, 12            | `fence_spacing` 12, 12          | 0    |
| Heading rule to fence               | 12 + 12 + 12                                | 36                | 36 (test)                       | 0    |
| Paragraph to fence                  | 12 + 12 + 12                                | 36                | 36 (test)                       | 0    |
| User trailing row                   | mt 8 + height 24                            | 32                | `USER_TRAILING_ROW` 32          | 0    |
| User to user                        | `getGapBetweenStreamItems`                  | 4                 | `message_gap` 4                 | 0    |
| User to assistant, user to end      | same                                        | 0                 | 0                               | 0    |
| User to tools or other              | same                                        | 16                | 16                              | 0    |
| Assistant to footer                 | para 12, `compactBottom`, footer gap 0      | 12                | 12                              | 0    |
| Assistant to end                    | para 12 + padding 12                        | 24                | 24                              | 0    |
| Assistant to tools                  | 12 + 12 + gap 4                             | 28                | 28                              | 0    |
| Assistant to assistant, user, other | 12 + 12 + gap 16                            | 40                | 40                              | 0    |
| Footer top, height                  | mt 13, 24                                   | 13, 24            | `FOOTER_TOP` 13, 24             | 0    |
| Step list top, step gap             | `turn/view.tsx` 16, 6                       | 16, 6             | 16, 6                           | 0    |
| Step row padding, inner gap         | 4, 3                                        | 4, 3              | 4, 3                            | 0    |
| Rule above and below the answer     | mt 22, mb 18                                | 22, 18            | `RULE_TOP` 22, `RULE_BOTTOM` 18 | 0    |

### Differences outside this TASK

These values live in `vendor/gpui-base/src/text/node.rs`, which is not in this TASK's Files. The timeline cannot set them through `RichTextStyle`.

| Value                          | Electron                                    | GPUI (vendor)      | Note                              |
| ------------------------------ | ------------------------------------------- | ------------------ | --------------------------------- |
| List item gap                  | 4                                           | `body*0.25` = 3.75 | Within 1 px                       |
| Space around a horizontal rule | 34 above, 22 below                          | 24 above, 24 below | 10 px less above, 2 px more below |
| List to prose gap              | 32 (last item mb 4 + list mb 16 + block 12) | `paragraph_gap` 24 | GPUI is 8 px shorter              |

The list and rule rows miss the 1 px match but stay shorter than Electron, so the anti-goal (never more than 2 px taller) holds. A match needs a vendor change or a new `TextViewStyle` option. Not done here: the TASK limits Files to the timeline `lib.rs`. The rule row is 2 px taller below.

## Verified

- `cd packages/desktop-gpui && CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-timeline` -> PASS, exit 0, 0 failures (24 unit tests in `lib.rs` including `message_gaps_match_the_web_stream`, plus the integration tests).
- `./cargo.sh fmt --check -p bachuc-timeline` -> exit 0.
- Electron values read from source only: `markdown-styles.ts`, `message.tsx`, `agent-stream/spacing.ts`, `agent-stream/turn-footer.tsx`, `turn/view.tsx`, `utils/markdown-list.ts`. No Electron app was opened.
- DEFERRED (UI checker): screenshot one thread with paragraphs, a list, a heading, and a code block in Electron and GPUI; compare the rhythm.

## Anti-goal

Row heights come from source reads, not from a GUI measure. In the table, every timeline row is equal to Electron within 1 px. The vendor rows differ: list to prose (GPUI 8 px shorter), rule above (10 px shorter), rule below (2 px taller, the only row at the 2 px limit). Read from `utils/markdown-list.ts` and `node.rs` (`block_gap`, `rule_space`) on 2026-10-18. The GUI read is DEFERRED (UI checker).

## Lessons

- The earlier GPUI timeline work already copied Electron's spacing with comments and tests. Re-measure from source before changing a constant.
- `space_below` mixed entry lookup with the spacing rule. The pure `message_gap` makes the rule testable without a view.
- Lists, rules, blockquotes, and table margins sit in the vendored text view, so a spacing fix for them needs a vendor change.
