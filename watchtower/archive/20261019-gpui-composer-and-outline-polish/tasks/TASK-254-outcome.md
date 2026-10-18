# TASK-254 Outcome

Status: DONE

## Changed

- `packages/desktop-gpui/crates/composer/src/composer.rs` (`input_box`): the two absolute layers became one absolute row (`left`/`right` at `INPUT_INSET`, `gap` `SPACE_2`). The placeholder is `flex_1`, `min_w_0`, truncated; the hint is `flex_none` with the same color, opacity 0.5, and size.

## Contract

No public API change. Hint hidden (focused or no shortcut) -> the placeholder takes the full width. Text typed -> neither shows. The row sits at `top_0`, so a draft with `min_lines` keeps it on the first line.

## Verified

- `./cargo.sh test -p bachuc-composer` -> 124 passed.
- `./cargo.sh build -p bachuc-app` -> builds.
- `./cargo.sh fmt --check -p bachuc-composer` -> exit 0.
- GitNexus impact on `input_box`: LOW.

## Anti-goal

DEFERRED (UI checker):

- zoom 100%, 1200x800, right panel open, empty and unfocused -> placeholder ends in "…" before the hint, no overlap.
- `cmd+l` focus -> hint hides, placeholder takes full width.
- new-thread draft (`min_lines` > 1) -> row on the first line, no overlap.
- zoom 125%, 1600x900 -> placeholder and hint as before.

## Lessons

- The hint has no explicit `line_height` (as before); only the placeholder sets the input line height, so the hint's vertical position is unchanged.

## UI Checks

Lead, computer use, 2026-10-19. Screenshots in `/tmp/wt-ui/`.

- Zoom 100%, 1200x800, right panel open, empty and unfocused: PASS. The placeholder ends in "use..." before "⌘L to focus"; no overlap (`254-z100-1200-panel.png`).
- `cmd+l`: PASS. The hint hides and the placeholder takes the full width (`254-z100-1200-focused.png`).
- New-workspace draft (5 lines): PASS. The row sits on the first line, focused and unfocused (`254-z100-newws-focused.png`, `254-z100-newws-blurred.png`).
- Anti-goal, zoom 125%, 1600x900: PASS. Placeholder and hint sit where the baseline has them (`254-before-z125-1600.png` vs `254-after-z125-1600.png`).
