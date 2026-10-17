# TASK-160 Settings Fonts and Syntax rows

Group: D (writes `crates/settings`)
Class: code

## Brief

Goal: Show the Electron Fonts and Syntax rows in the Appearance page: interface font, code font, content size, and highlight theme (Q-083).

Change: the Appearance page has the theme, interface size, and code size rows (`crates/settings/src/pane_appearance.rs`) -> also the interface font, content size, code font, and syntax theme rows with the syntax preview, stored with the other appearance values and applied at load.

Design: [packages/app/src/screens/settings/appearance/appearance-section.tsx](packages/app/src/screens/settings/appearance/appearance-section.tsx) for order, labels, row layout, and the syntax preview. Electron hides the interface font row only on iOS and Android (line 408); the desktop shows it, so GPUI shows it.

Boundaries:

- Font family rows are pickers of `installed_font_families`, with a "Default" entry first (owner choice; Electron uses a text field).
- The highlight theme row lists the eight syntax themes with their `themes.ts` labels.
- Persist in the existing `appearance.json`; an old file without the new keys loads with the defaults. `load_appearance` applies the new values too.
- Keep `Settings::new`, `SettingsEvent`, and `load_appearance` signatures. Settings writes only BachucGpui files.

Files:

- [packages/desktop-gpui/crates/settings](packages/desktop-gpui/crates/settings) (`src`, `tests`)

Interfaces:

- Consumes: `bachuc_theme::set_font_families(ui: Option<gpui_kit::SharedString>, code: Option<gpui_kit::SharedString>, cx: &mut gpui_kit::App)`, `bachuc_theme::ui_font_family(cx: &gpui_kit::App) -> gpui_kit::SharedString`, and `bachuc_theme::code_font_family(cx: &gpui_kit::App) -> gpui_kit::SharedString` (the code override, else `mono_font_family`) from TASK-159
- Consumes: `bachuc_theme::set_content_font_size(px: f32, cx: &mut gpui_kit::App)` and `bachuc_theme::content_font_size(cx: &gpui_kit::App) -> gpui_kit::Pixels` from TASK-159
- Consumes: `bachuc_theme::installed_font_families(cx: &gpui_kit::App) -> Vec<gpui_kit::SharedString>` from TASK-159
- Consumes: `bachuc_theme::SyntaxThemeId` (enum with the eight ids of `themes.ts`, `id() -> &'static str`, `label() -> &'static str`, `from_id(&str) -> Option<Self>`, `ALL`), `bachuc_theme::set_syntax_theme(id: SyntaxThemeId, cx: &mut gpui_kit::App)`, `bachuc_theme::syntax_theme(cx: &gpui_kit::App) -> SyntaxThemeId`, and `bachuc_theme::syntax_colors(cx: &gpui_kit::App) -> bachuc_theme::SyntaxColors` (one `Hsla` field per highlight role of `colors.ts`) from TASK-159
- Produces: the same `bachuc_settings::Settings`, `SettingsEvent`, and `load_appearance` API for TASK-167

Expected result:

- Picking a font, a size, or a syntax theme applies at once and survives a reload of `appearance.json`.

Anti-goal: no new write path; tripwire: a test asserts that saving writes only `appearance.json` in the settings directory it is given; read at completion.

## Verify

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-settings` -> pass, including a round-trip test for the four new values, an old-file-without-keys test, a corrupt-file test, a content size clamp test, a stored font that is not installed, the "Default" entry, an unknown syntax id test, and the write-path test
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-settings` -> exit 0
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> exit 0
