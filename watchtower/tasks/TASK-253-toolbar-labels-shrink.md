# TASK-253 Toolbar labels shrink like Electron

Group: C (composer crate; shares `composer.rs` with TASK-254)
Class: code

## Brief

Goal: when the composer toolbar lacks room, the model, thinking, and mode labels get shorter with "…" before any pill is cut, as in Electron.

Change: the controls row clips a whole pill when it overflows -> the row and its labels can shrink, so text truncates first.

Boundaries:

- Q-162 (owner, 2026-10-19): port Electron's shrink rule; keep `resolve_density` and its thresholds and tests unchanged.
- Electron source: `packages/app/src/composer/agent-controls/index.tsx` styles `container` (`minWidth: 0`, `flexShrink: 1`, `overflow: hidden`), `modelControl` (`minWidth: 0`, `flexShrink: 1`), `modeBadgeText` (`minWidth: 0`, `flexShrink: 1`), and fixed parts with `flexShrink: 0` (icons, carets, `modeIconBadge`).
- Icons, carets, and icon-only badges keep their size; only label text shrinks and ends in "…".
- Keep the density switch (Full, Condensed, Tight) as it is.
- Code facts: `density_box` (`composer.rs:2257`) puts the pills in a bare `h_flex()` with no `min_w_0` or `overflow_hidden`; pills are `flex_none` (`control.rs:33`, `:83`); the value label truncates at a fixed `max_w(VALUE_MAX_WIDTH)` (`control.rs:107`).
- Planner call: keep `VALUE_MAX_WIDTH` as the maximum; give the label `min_w_0` and `flex_shrink`, the pill `flex_shrink` with `min_w_0` around its label, and the `density_box` row `min_w_0` and `overflow_hidden`.

Files:

- [packages/desktop-gpui/crates/composer/src/agent_controls/control.rs](packages/desktop-gpui/crates/composer/src/agent_controls/control.rs)
- [packages/desktop-gpui/crates/composer/src/agent_controls/mod.rs](packages/desktop-gpui/crates/composer/src/agent_controls/mod.rs)
- [packages/desktop-gpui/crates/composer/src/composer.rs](packages/desktop-gpui/crates/composer/src/composer.rs) (`density_box` and the toolbar row only)

Interfaces:

- Produces: no public API.

Expected result:

- At zoom 150% in a 1200x800 window, with text typed so the send button shows, no toolbar pill is cut; long labels end in "…".

Anti-goal: at zoom 100% in a 1600x900 window, the toolbar looks the same as before (no label truncates when there is room); read from screenshots before and after.

## Verify

- `cd packages/desktop-gpui && CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-composer` -> pass; the existing `resolve_density` tests are unchanged.
- `cd packages/desktop-gpui && CARGO_BUILD_JOBS=4 ./cargo.sh build -p bachuc-app` -> builds.
- Screen check: zoom 150%, 1200x800, `mock TASK-252 outline`, type "/" so the send button shows -> no pill is cut; a long label ends in "…".
- Screen check: zoom 150% with the right panel open (narrow composer) -> no pill is cut; the model, thinking, and mode labels each truncate with "…" when needed.
- Screen check: widen and narrow the pane across a density floor (open and close the right panel) -> labels re-expand and no pill stays cut.
- Anti-goal: zoom 100%, 1600x900 -> labels are whole, same as before.
