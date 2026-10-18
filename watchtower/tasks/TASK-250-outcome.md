# TASK-250 Outcome

## Status

DONE (code checks only; the UI checker lines are DEFERRED).

## Changed

- Board (`packages/desktop-gpui/plugins/board/src`): `page.rs`, `header_chip.rs`, `card.rs`, `runs.rs`, `recaps.rs`, `timeline.rs`.
- Watchtower board (`packages/desktop-gpui/plugins/watchtower-board/src`): `page.rs`, `overview.rs`, `cell.rs`, `cells.rs`, `decision_modal.rs`, `needs_you.rs`, `panel.rs`, `plan_tree_view.rs`, `rail.rs`, `run_cells.rs`, `task_list.rs`.
- No `Cargo.toml` change: both crates already depend on `bachuc-theme`.

## Contract

- Interfaces: none.
- Board: the Board size scale is now `board_scale(size) * bachuc_theme::zoom(cx)` at the three render entries and in the timeline item chart (`page.rs`, `runs.rs` column item, `recaps.rs` item, `timeline.rs` item). Every `s(..)`, `p.s(..)`, and `Chart::s(..)` measure follows both the Board size and the zoom. `board_scale` itself is unchanged.
- Board fixed sizes (page toolbar, view switch, size control, header chip, spinners, toggle row `min_h`) use `bachuc_theme::zoomed`.
- `card.rs` `toggle_row` gained a `cx: &App` parameter (private fn, two callers in the same file).
- Watchtower board: every fixed `px(X)` became `bachuc_theme::zoomed(X, cx)`; `overview.rs` `grid_row` gained a `cx: &App` parameter (private fn, two callers).
- Watchtower board page: the measured width is divided by `bachuc_theme::zoom(cx)` before `set_width`, so the 720 px stack and other breakpoints stay in unzoomed px. A zoom change with the same window width changes the logical width, so the canvas defers `set_width` and the layout rebuilds.
- `PLANNED_BORDER` (1.5) and `CHECKBOX_BORDER` (1.5) in `plan_tree_view.rs` and `cells.rs`, and the 2 px plan spine, now zoom: they are icon strokes, not 1 px hairlines.

## Verified

Group-level runs cover this TASK:

- `./cargo.sh test -p bachuc-plugin-board -p bachuc-plugin-watchtower-board` (one call): board lib 97 passed; watchtower-board lib 132 passed, 1 failed. The failure is `model::overview_view::tests::one_thousand_derives_of_a_40_task_plan_take_under_200_ms` (209 ms against 200 ms, load average 6.5). Run alone it passes. It is a pure model timing test that this TASK did not touch. Doc tests did not run, because cargo stopped at the failed target.
- `./cargo.sh build -p bachuc-app` -> exit 0. The only warning is `next_theme` unused in `crates/app/src/state.rs`, not a changed file. One earlier build failed only in `crates/sidebar/src/sidebar.rs` (group A); the build passed after 2 minutes.
- `./cargo.sh fmt -p ...` then `./cargo.sh fmt --check -p bachuc-plugin-board -p bachuc-plugin-watchtower-board` -> exit 0.
- Remaining `px(` sites (59 lines, `rg -P '(?<![\w.])px\('`), by kept case:
  - Derived from an already-zoomed scale: 29 (the `s(..)` helpers: 11; focus ring geometry in `card.rs`: 5; repo actions offset and nested `edge`: 4; orb canvas and avatar from the scaled footprint: 9).
  - Zero: 6 (`px(0.)`).
  - 1 px hairline: 3 (`runs.rs:393`, `timeline.rs:652`, `needs_you.rs:444`).
  - 2 px focus ring spread (`card.rs:160`): 1.
  - Read from the timeline layout (bar and break x and width, tick x, load bucket): 11 in `timeline.rs`.
  - List overdraw, not drawn: 6.
  - Test code: 2 (`orb_view.rs`).

## Anti-goal

DEFERRED (UI checker). At zoom 100%, `zoomed(X, cx) == px(X)` and `zoom(cx) == 1`, so the sizes are the same by construction. No screenshot was taken.

## UI checker

DEFERRED (UI checker): Board with runs and the Watchtower board with a TASK dialog open, at `BACHUC_WINDOW=1200x800` and zoom 150%.

## Lessons

- The Board already had its own size scale. Folding the zoom into that scale was smaller and safer than converting each `s(..)` call.
- A page that picks its layout from a measured width must divide the width by the zoom, or breakpoints move with the zoom.
- Group B edits only `private` helpers' signatures; no public symbol changed.

## UI Checks

- UI checker 1200x800 zoom 150%: Board with runs and Watchtower board with a TASK dialog (also needs Board runs that live may lack) -> UNVERIFIED (unattended): background frame
- Anti-goal: zoom 100%, 0 changed pixel rows before vs after -> UNVERIFIED (unattended): background frame

Evidence: /tmp/wt-ui/zoom/before-initial-1200.png and before-initial-1200b.png (8 s apart), before-key-try.png, before-key-try2.png: the 1200x800 window still shows "Connecting / live" while the app holds an ESTABLISHED socket to 127.0.0.1:6768. GPUI does not redraw a background window (Q-069), so no frame shows thread, composer, or zoom. A Cmd+= and a Cmd+- key reached the app (appearance.json changed 1.25 -> 1.5 -> 1.25) but the window did not redraw. The check bundle was built (exit 0); a baseline copy is at `Bachuc GPUI Zoom Before.app`. The checker did not use the front, the pointer, or AX clicks.

Blocked: BUILT, UNVERIFIED (Q-161)
