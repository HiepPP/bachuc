# TASK-233 Thread spacing like Electron

Group: T ([packages/desktop-gpui/crates/timeline/src/lib.rs](packages/desktop-gpui/crates/timeline/src/lib.rs); after TASK-232, which writes the same file)
Class: code

## Brief

Goal: Agent threads in GPUI have the same vertical rhythm as Electron, so they are as easy to read.

Change: GPUI gap and line-height constants -> values measured from the Electron thread.

Boundaries:

- The owner chose "measure Electron" on 2026-10-18. Do not invent a new, more open spacing.
- Measure in Electron source: paragraph gap, list item gap, heading gaps, code block margins, line height, gap between steps, gap between user and assistant rows. Sources: [packages/app/src/styles/theme.ts](packages/app/src/styles/theme.ts) and the markdown and timeline styles under [packages/app/src](packages/app/src).
- GPUI constants live at about lines 117-150 of [packages/desktop-gpui/crates/timeline/src/lib.rs](packages/desktop-gpui/crates/timeline/src/lib.rs) (`STEP_GAP`, `BLOCK_GAP`, `PARAGRAPH_GAP`, `USER_TRAILING_ROW`, `FOOTER_TOP`), plus `space_below` (about line 1207) and `markdown_style` (about line 1558).

How:

- Write a table of each Electron value and the GPUI value into the outcome.
- Change the GPUI constants and markdown style to match.
- Keep `space_below` rules in a pure function and update its tests.

Files:

- [packages/desktop-gpui/crates/timeline/src/lib.rs](packages/desktop-gpui/crates/timeline/src/lib.rs) (spacing constants, `space_below`, `markdown_style`)

Expected result:

- The outcome has an Electron versus GPUI table with every value equal or within 1 px.
- A UI checker screenshot pair shows the same thread in Electron and GPUI with matching rhythm.

Anti-goal: no row grows taller than its Electron match by more than 2 px; read from the outcome table at the end.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-timeline` -> all pass.
- Outcome table -> every spacing value matches Electron within 1 px.
- UI checker: screenshot one thread with paragraphs, a list, a heading, and a code block -> spacing matches the Electron screenshot of the same thread.
