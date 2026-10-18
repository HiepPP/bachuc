# TASK-232 Text overflow on narrow windows

Group: O (any view crate the scan finds; runs first, so ordering alone keeps it apart)
Class: risky

## Brief

Goal: On a narrow window, no text leaves its card. Text does not run past the right edge, and wrapped text does not draw over the next row.

Change: text that overflows right or overlaps the row below -> text that wraps or truncates inside its parent, with correct row height.

Boundaries:

- Two bugs. Horizontal: AskUserQuestion option descriptions are cut at the right edge (owner screenshot 1). Vertical: the last bullet of an assistant message draws over the Recap card below it (owner screenshot 2). Find the root cause of each before fixing.
- Known suspects, not proven: the thread row column at [packages/desktop-gpui/crates/timeline/src/lib.rs](packages/desktop-gpui/crates/timeline/src/lib.rs) about line 1186 has `max_w(px(MAX_CONTENT_WIDTH))` (820, [packages/desktop-gpui/crates/theme/src/metrics.rs](packages/desktop-gpui/crates/theme/src/metrics.rs) line 61) and maybe no `min_w_0`; assistant markdown near line 1467 (`RichText::new`) sits in a bare div; the option text column in [packages/desktop-gpui/crates/timeline/src/card_view.rs](packages/desktop-gpui/crates/timeline/src/card_view.rs) about line 649 is `flex_1` with no `min_w_0`.
- For the vertical overlap, check whether the list row height is cached from an older, wider width. Follows [ADR-0039](watchtower/decisions/ADR-0039-incremental-timeline-sync-over-throttle-or-budget.md): keep incremental sync; do not re-measure every row on each frame.
- After the fix, scan every GPUI view crate and plugin crate for the same pattern: a text child in a flex row with no `min_w_0`, no wrap, and no truncate. Fix each hit. Planner call: a hit may stay unchanged only when the text can never be long (for example a fixed short label); list it with that reason. List every site in the outcome.
- Tab titles, sidebar rows, and the header title already truncate; do not change them unless the scan finds a bug.

How:

- Reproduce both bugs at about 700 px window width with a long path and a long option description.
- Find and fix each root cause.
- Put the width rule (for example, the text column width from the row width) in a pure function and test it.
- Scan with `rg` for flex text children, fix each hit, and list them.

Files:

- [packages/desktop-gpui/crates/timeline/src/lib.rs](packages/desktop-gpui/crates/timeline/src/lib.rs) (row column and markdown container)
- [packages/desktop-gpui/crates/timeline/src/card_view.rs](packages/desktop-gpui/crates/timeline/src/card_view.rs) (question options and tab headers)
- Other view and plugin files the scan finds

Expected result:

- At 700 px, the AskUserQuestion option text wraps inside the card.
- At 700 px, no assistant message draws over the next row, including after a window resize.
- The outcome lists every scanned site and its fix.

Anti-goal: timeline app CPU on the mock stream stays within 1 point of the base commit; a re-measure of all rows per frame is the tripwire; read with `packages/desktop-gpui/measure.sh` before and after the fix.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-timeline` -> all pass, including a new test for the width rule.
- UI checker at 700 px: the option text in an AskUserQuestion card wraps inside the card -> no text past the card edge.
- UI checker: resize from 1600 px to 700 px with a long assistant message above a Recap card -> no overlap.
- UI checker at 700 px: a message with a 200-character path with no spaces -> the path breaks or scrolls inside its block.
- `packages/desktop-gpui/measure.sh` before and after -> CPU difference 1 point or less.
