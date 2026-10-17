# TASK-207 Step list matches Electron

Group: T (shared files [packages/desktop-gpui/crates/timeline/src/lib.rs](packages/desktop-gpui/crates/timeline/src/lib.rs) and its modules)
Class: code

## Brief

Goal: The timeline step list looks like Electron: measured prose spacing, quote blocks, and the step label; and Edit rows show a home path as `~`.

Change: quotes use the gpui-base default, spacing differs in places the owner saw, and an Edit row outside the cwd shows `/Users/<name>/...` -> Electron's quote style, measured spacing fixes, and `~/...` paths.

Boundaries:

- Prose spacing: measure first, then fix only measured gaps (owner choice, 2026-10-17). The last plan matched Electron's pitch on purpose: `BODY_LINE_HEIGHT` 1.618 gives a 24 px line box, and `PARAGRAPH_GAP` is `BLOCK_GAP` less 1 px to cancel a 1 px drift per block (`lib.rs` lines 110-122). Do not change these constants unless a side-by-side screenshot shows a real gap.
- Quote block: Electron [packages/app/src/styles/markdown-styles.ts](packages/app/src/styles/markdown-styles.ts) lines 315-327: `surface1` background, text `foreground` at about 0.8 alpha, left border 4 px in `surface2`, padding x 16, top 12, bottom 0, margin y 12, radius 6 with left corners 0. GPUI sets no blockquote style in `markdown_style` (`lib.rs` lines 1449-1502), so the vendored default applies (2 px border, no background). If the markdown style API cannot set a background or radius, set what it can and record the rest in the outcome. Do not edit `vendor/gpui-base` ([ADR-0044](watchtower/decisions/ADR-0044-vendored-gpui-base-patch-over-git-fork-or-log.md)).
- Edit row path: the edit-diffs plugin draws the row (`plugins/edit-diffs/src/row.rs` line 504 calls `display_path`). `edit_item.rs` lines 113-120 already strip `cwd/` and `/private` + `cwd/`, with tests at lines 240-252. Owner choice (2026-10-17): a path outside the cwd that starts with the home folder shows as `~/...`. This differs from Electron on purpose. Paths inside the cwd stay relative.
- Step label: Electron kind label has letter spacing 0.6 (`view.tsx` lines 727-733). Add it if GPUI text supports letter spacing; otherwise record it.
- The circle, summary, time column, and gaps already match; do not change them.

How:

- Screenshot one thread in GPUI and Electron at the same width. Measure line pitch, paragraph gap, and list gaps. Fix only real gaps.
- Add the quote style to `markdown_style`.
- Add the `~` rule to `display_path` with tests: inside cwd, outside cwd under home, outside home, and empty cwd.

Files:

- [packages/desktop-gpui/crates/timeline/src/lib.rs](packages/desktop-gpui/crates/timeline/src/lib.rs) (quote style, letter spacing, measured spacing fixes)
- [packages/desktop-gpui/plugins/edit-diffs/src/edit_item.rs](packages/desktop-gpui/plugins/edit-diffs/src/edit_item.rs) (`~` rule and tests)

Expected result:

- A quote block shows a 4 px left border, a background, and padding as Electron.
- An Edit of `/Users/hiep/.claude/x.md` with cwd `/Users/hiep/Projects/bachuc` shows `~/.claude/x.md`. An Edit of `/Users/hiep/Projects/bachuc/a.rs` shows `a.rs`.
- Prose line pitch and paragraph gaps measure within 1 px of Electron on the same thread.

Anti-goal: timeline CPU while a turn streams must stay under 10%, the limit in [ADR-0039](watchtower/decisions/ADR-0039-incremental-timeline-sync-over-throttle-or-budget.md); tripwire; read on live with the mock `five-minute-stream` model before the change and at completion.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-plugin-edit-diffs` -> passes, including the 4 path cases.
- `./cargo.sh test -p bachuc-timeline` -> passes.
- UI check on live: a turn with prose, a quote, and an Edit row, screenshot beside Electron -> quote matches; prose pitch and gaps within 1 px.
- Owner hand test: read a long thread -> the step list reads like Electron.
- Anti-goal: CPU during the mock stream -> under 10% at both reads.
