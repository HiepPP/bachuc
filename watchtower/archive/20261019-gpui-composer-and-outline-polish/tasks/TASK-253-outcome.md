# TASK-253 Outcome

Status: DONE

## Changed

- `packages/desktop-gpui/crates/composer/src/agent_controls/control.rs`: the pill is `min_w_0` + `flex_shrink(1.)` (icon-only pills stay `flex_none`); the glyph sits in a `flex_none` wrapper; the label gets `min_w_0` + `flex_shrink(1.)` under the same `VALUE_MAX_WIDTH` cap.
- `packages/desktop-gpui/crates/composer/src/agent_controls/mod.rs`: the model and choice `Popover` roots get `min_w_0` when the pill is not icon-only (the Popover root is the flex item in the toolbar row).
- `packages/desktop-gpui/crates/composer/src/composer.rs`: the `density_box` row gets `w_full`, `min_w_0`, `overflow_hidden`.

## Contract

No public API change. `resolve_density`, its thresholds, and its tests are unchanged. Labels shrink with "…" before any pill is cut.

## Verified

- `./cargo.sh test -p bachuc-composer` -> 124 passed.
- `./cargo.sh build -p bachuc-app` -> builds (one pre-existing dead-code warning, `next_theme`).
- `./cargo.sh fmt --check -p bachuc-composer` -> exit 0.
- GitNexus impact on `PillTrigger`, `density_box`: LOW.

## Anti-goal

DEFERRED (UI checker):

- zoom 150%, 1200x800, `mock TASK-252 outline`, type "/" -> no pill cut; long label ends in "…".
- zoom 150% with the right panel open -> model, thinking, mode labels truncate with "…".
- widen/narrow across a density floor -> labels re-expand, no pill stays cut.
- zoom 100%, 1600x900 -> labels whole, same as before.

## Lessons

- A `Popover` root, not the trigger, is the flex item in the row: put `min_w_0` on the Popover (it is `Styled`).
- `flex_shrink` in this GPUI takes an `f32`: `.flex_shrink(1.)`.

## UI Checks

Lead, computer use, 2026-10-19, check bundle `Bachuc GPUI Check` built after `4be6d17f5` code. Screenshots in `/tmp/wt-ui/`.

- Run 1 (after `46ecfd5da`): FAIL. Zoom 150%, 1200x800, "/" typed: the thinking pill was cut and "Ten second stream" stayed whole (`253-z150-1200-slash.png`, first version). Cause: `Styled` on the gpui-component `Popover` styles the menu panel, so `min_w_0` never reached the trigger box.
- Fix round 1 (`4be6d17f5`, lead): `trigger_style(flex + min_w_0)` on the model and choice popovers.
- Zoom 150%, 1200x800, "/" typed: PASS. No pill cut; model reads "Ten sec…" (`253-z150-1200-slash.png`).
- Zoom 150%, right panel open: PASS. No pill cut; model "Te…", the thinking label shrinks to its icon, mode is icon-only (`253-z150-1200-panel.png`).
- Close the panel again: PASS. Labels re-expand to the first state (`253-z150-1200-panel-closed.png`).
- Anti-goal, zoom 100%, 1600x900: PASS. "Ten second stream" and "Low" are whole and at the same x as the baseline (`253-before-z100-1600.png` vs `253-after-z100-1600.png`). The "Load Test" mode pill shows only in the after shot; the baseline shot was taken before the agent reported its modes.
- Note: with little room, a short label such as "Low" shrinks to "." or "..", because flex shrink is shared by basis, as in Electron.
