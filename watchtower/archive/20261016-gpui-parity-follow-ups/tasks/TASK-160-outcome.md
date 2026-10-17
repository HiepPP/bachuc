# TASK-160 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/settings/src/appearance.rs`: `Appearance` gains `content_size`, `ui_font`, `code_font`, and `syntax_theme`; load, save, and `apply` handle them; pure helpers `family_choice`, `family_label`, `clamp_content_size`; new tests.
- `packages/desktop-gpui/crates/settings/src/pane_appearance.rs`: interface font, content size, and code font rows in Fonts (Electron order); new Syntax section with the highlight theme row and the preview.
- `packages/desktop-gpui/crates/settings/src/appearance_preview.rs` (new): the live preview (content line plus a unified TypeScript diff in the code font and syntax colors).
- `packages/desktop-gpui/crates/settings/src/ui.rs`: `select_trigger` takes `Option<IconName>` for the leading icon; the value truncates at 220 px.
- `packages/desktop-gpui/crates/settings/src/lib.rs`: module line and doc.

## Contract

- `Settings::new`, `SettingsEvent`, and `load_appearance` are unchanged. `appearance::load_from`, `save_to`, and `apply` keep their signatures; `apply` also calls `set_font_families`, `set_content_font_size`, and `set_syntax_theme`.
- `Appearance` is no longer `Copy` (the families are `Option<String>`). The only outside user, `theme.cycle` in `crates/app/src/shell.rs`, still compiles.
- Keys in `appearance.json`: `contentSize` (floored, clamped to 10-21, default 15), `uiFont` and `codeFont` (omitted when unset; an empty name loads as unset), `syntaxTheme` (the `themes.ts` id; an unknown or non-string value loads as `one`). An old file without the keys loads the defaults.
- A stored family that is not installed stays in the file, shows "Default" in the picker, and the theme falls back to the default family.
- The family pickers list `installed_font_families` with "Default" first, in a scrollable menu (max 320 px). The syntax picker lists the eight themes with the `themes.ts` labels.

## Verified

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-settings` -> 33 unit tests and 4 `project_config` tests pass, including the round trip of the four new values, an old file without keys, a corrupt file, the content size clamp, a stored font that is not installed with the "Default" label, an unknown syntax id, and the write-path test.
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-settings` -> exit 0.
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> exit 0.
- UNVERIFIED (unattended): code tests only. No window was opened; the pane layout, the pickers, and the preview were not seen.

## Anti-goal

No new write path. Test `saving_writes_only_appearance_json` saves twice into an empty directory and asserts that the directory holds exactly `appearance.json` (the temporary file is renamed). Read at completion, 2026-10-16, source: `cargo test -p bachuc-settings`, pass.

## Lessons

- Setters that need a GPUI `App` (`apply`, the pickers) have no test because `test-support` breaks workspace resolution; the rules live in pure functions that the tests cover.
- `row(first, cx)` and a `&mut Context` dropdown builder conflict on the borrow; build the dropdown first and mark its return type `+ use<>`.
- The preview tokens are a hand-written list of roles, not a tree-sitter parse, so the colors need no async work.
