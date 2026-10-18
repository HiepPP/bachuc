# TASK-254 Placeholder stops before the focus hint

Group: C (composer crate; shares `composer.rs` with TASK-253, runs after it)
Class: code

## Brief

Goal: in a narrow composer, the "⌘L to focus" hint never overlaps the placeholder; the placeholder ends in "…" before the hint.

Change: the placeholder and the hint are two absolute layers, so they overlap -> one row where the placeholder shrinks and truncates and the hint keeps its width.

Boundaries:

- Q-163 (owner, 2026-10-19): cut the placeholder before the hint. This differs from Electron, which overlaps (`focusHintText` in `packages/app/src/composer/input/input.tsx`).
- Planner call: draw one absolute row over the input (`left` and `right` at `INPUT_INSET`): the placeholder is `flex_1`, `min_w_0`, truncated; the hint is `flex_none` with a gap of `SPACE_2` before it. Keep the hint's color, opacity 0.5, and size.
- When the hint is hidden (input focused, or no shortcut), the placeholder takes the full width as today.
- When the input has text, neither shows, as today.
- A draft composer with `min_lines` keeps the row on the first line.

Files:

- [packages/desktop-gpui/crates/composer/src/composer.rs](packages/desktop-gpui/crates/composer/src/composer.rs) (the input box near `focus_hint`)

Interfaces:

- Consumes: nothing from TASK-253; runs after it only because both write `composer.rs`.
- Produces: no public API.

Expected result:

- At zoom 100% with the right panel open, and at zoom 150% in a 1200x800 window, the placeholder ends in "…" before "⌘L to focus"; no glyphs overlap.

Anti-goal: in a wide composer where both fit, the placeholder and hint stay where they are today; read from a 1600x900 screenshot before and after.

## Verify

- `cd packages/desktop-gpui && CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-composer` -> pass.
- Screen check: zoom 100%, 1200x800, right panel open, composer empty and unfocused -> placeholder ends in "…" before the hint, no overlap.
- Screen check: focus the composer (`cmd+l`) -> the hint hides and the placeholder takes the full width.
- Screen check: a new-thread draft (`min_lines` above 1) -> the row sits on the first line, no overlap.
- Anti-goal: zoom 125%, 1600x900 -> placeholder and hint as before.
