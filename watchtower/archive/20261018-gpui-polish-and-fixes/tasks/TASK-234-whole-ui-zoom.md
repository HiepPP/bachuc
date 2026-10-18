# TASK-234 Whole-UI zoom with Cmd+=, Cmd-, Cmd+0

Group: S (app shell, chrome, workspace, settings; shared with TASK-235, TASK-236, TASK-237)
Class: risky

## Brief

Goal: Cmd+= (and Cmd++) zooms the whole GPUI window in, Cmd+- zooms out, and Cmd+0 resets to 100%. The level survives a restart.

Change: no zoom -> whole-UI zoom of text, padding, and icons, saved in `appearance.json`.

Boundaries:

- The owner chose whole-UI zoom, saved, with Cmd+0, on 2026-10-18. Font-only zoom was rejected.
- Today only font scale exists: `set_font_scale`, `ui_font_size`, `code_font_size` in [packages/desktop-gpui/crates/theme/src/lib.rs](packages/desktop-gpui/crates/theme/src/lib.rs) about line 147. `Appearance` in [packages/desktop-gpui/crates/settings/src/appearance.rs](packages/desktop-gpui/crates/settings/src/appearance.rs) saves to `appearance.json` and applies at `apply()` about line 224.
- No `rem_size` or `ui_scale` use exists under `crates/` today. Probe the vendored GPUI for rem size or window scale first. Planner call: if neither exists, apply a theme-level multiplier to the `bachuc-theme` sizes, and record the method in the outcome. List fixed `px` sizes that do not scale in the outcome; convert the ones that break the layout.
- Steps: 50%, 67%, 75%, 80%, 90%, 100%, 110%, 125%, 150%, 175%, 200%. Planner call: match Electron and Chrome zoom steps.
- `RunAction(String)` is defined in [packages/desktop-gpui/crates/app/src/actions.rs](packages/desktop-gpui/crates/app/src/actions.rs) line 18. Add three action ids and handle them in `Shell::run_action` in [packages/desktop-gpui/crates/app/src/shell.rs](packages/desktop-gpui/crates/app/src/shell.rs) about line 3234. Planner call: bind both Cmd+= and Cmd+Shift+= (Cmd++) to zoom in. Add default chords in `default_shortcuts()` in [packages/desktop-gpui/crates/command/src/shortcuts.rs](packages/desktop-gpui/crates/command/src/shortcuts.rs) so the owner can rebind them.
- Release state has unassigned shortcut overrides with an empty `parsedChord` (CLAUDE.md Debugging). An empty override must not crash or bind a blank key.

How:

- Add a pure `zoom_step(current, direction) -> f32` with clamping and tests.
- Store the level in `Appearance` and apply it in `apply()`.
- Add three actions and default chords.

Files:

- [packages/desktop-gpui/crates/theme/src/lib.rs](packages/desktop-gpui/crates/theme/src/lib.rs) (zoom level applied as rem or scale)
- [packages/desktop-gpui/crates/settings/src/appearance.rs](packages/desktop-gpui/crates/settings/src/appearance.rs) (saved zoom level, pure step function)
- [packages/desktop-gpui/crates/app/src/actions.rs](packages/desktop-gpui/crates/app/src/actions.rs) (zoom action ids)
- [packages/desktop-gpui/crates/app/src/shell.rs](packages/desktop-gpui/crates/app/src/shell.rs) (`run_action` handlers)
- [packages/desktop-gpui/crates/command/src/shortcuts.rs](packages/desktop-gpui/crates/command/src/shortcuts.rs) (default chords)

Expected result:

- Cmd+= and Cmd+- change the size of all text, padding, and icons in steps.
- Cmd+0 returns to 100%. The level stays after a restart.

Anti-goal: a zoom step redraws in under 1 frame of extra work; no per-frame font query is added (MEMORY 2026-10-17 `all_font_names` lesson); checked by reading the render path at the end.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-settings -p bachuc-command` -> all pass, including step clamp tests at 50% and 200%, an empty-chord override test, and a test that a saved value off-step, out of range, NaN, or missing reads as the nearest valid step or 100%.
- UI checker: send Cmd+= twice, screenshot -> text and padding larger; send Cmd+0 -> back to 100%.
- Owner hand test: zoom to 150%, quit, reopen -> still 150%.
- Read the render path -> no font enumeration added per frame.
