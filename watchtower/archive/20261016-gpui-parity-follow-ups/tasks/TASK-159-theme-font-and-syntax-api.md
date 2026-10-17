# TASK-159 Theme font and syntax API

Group: C (writes `crates/theme`)
Class: code

## Brief

Goal: Let the app set the interface font family, code font family, content size, and syntax theme, as the Electron appearance settings do.

Change: `bachuc-theme` has `set_font_scale` and a fixed `mono_font_family`, and the timeline and explorer hard-code the `one` syntax colors -> setters and getters for each setting, and the syntax palettes of `packages/highlight`.

Design: [packages/app/src/appearance/apply.ts](packages/app/src/appearance/apply.ts) (`AppearanceInput` lines 14-20, family fallbacks lines 63-64, `resolveSyntaxColors` lines 85 and 93), [packages/app/src/hooks/use-settings/storage.ts](packages/app/src/hooks/use-settings/storage.ts) (defaults lines 78-81 and 137), [packages/highlight/src/themes.ts](packages/highlight/src/themes.ts) and `packages/highlight/src/colors.ts` (the eight syntax themes and their light and dark palettes).

Boundaries:

- Keep every existing public item and default of `bachuc-theme`. `mono_font_family(cx) -> &'static str` (`crates/theme/src/lib.rs:90`) stays as it is; the override goes through the new `code_font_family`, and the crate's own callers (`apply`, line 189) switch to it.
- Content size is a new setting with no GPUI key yet: default to the Electron desktop value of `contentFontSize` ([packages/app/src/hooks/use-settings/storage.ts](packages/app/src/hooks/use-settings/storage.ts) line 79) and clamp it as Electron does.
- Port all eight `SyntaxThemeId` values with their light and dark palettes; a theme with one palette uses it in both app themes, as `themes.ts` says. Default `one`.
- An unknown stored syntax id or a family that is not installed falls back to the default, with no panic.
- Setting a value writes the gpui-kit `Theme` global, so views that observe it (lesson in [watchtower/CONTEXT.md](watchtower/CONTEXT.md)) redraw.

Files:

- [packages/desktop-gpui/crates/theme](packages/desktop-gpui/crates/theme) (`Cargo.toml`, `src`, `tests`)

Interfaces:

- Produces: `bachuc_theme::set_font_families(ui: Option<gpui_kit::SharedString>, code: Option<gpui_kit::SharedString>, cx: &mut gpui_kit::App)`, `bachuc_theme::ui_font_family(cx: &gpui_kit::App) -> gpui_kit::SharedString`, and `bachuc_theme::code_font_family(cx: &gpui_kit::App) -> gpui_kit::SharedString` (the code override, else `mono_font_family`) for TASK-160, TASK-164, and TASK-167
- Produces: `bachuc_theme::set_content_font_size(px: f32, cx: &mut gpui_kit::App)` and `bachuc_theme::content_font_size(cx: &gpui_kit::App) -> gpui_kit::Pixels` for TASK-160, TASK-164
- Produces: `bachuc_theme::installed_font_families(cx: &gpui_kit::App) -> Vec<gpui_kit::SharedString>` for TASK-160
- Produces: `bachuc_theme::SyntaxThemeId` (enum with the eight ids of `themes.ts`, `id() -> &'static str`, `label() -> &'static str`, `from_id(&str) -> Option<Self>`, `ALL`), `bachuc_theme::set_syntax_theme(id: SyntaxThemeId, cx: &mut gpui_kit::App)`, `bachuc_theme::syntax_theme(cx: &gpui_kit::App) -> SyntaxThemeId`, and `bachuc_theme::syntax_colors(cx: &gpui_kit::App) -> bachuc_theme::SyntaxColors` (one `Hsla` field per highlight role of `colors.ts`) for TASK-160, TASK-162, TASK-164

Expected result:

- Each syntax theme in light and Claude dark returns the `colors.ts` values.
- The setters persist nothing; the app and Settings own persistence.

Anti-goal: the existing token comparison test and turn label test still pass unchanged; tripwire; read after the syntax port and at completion.

## Verify

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-theme` -> pass, including the existing token comparison test, a test that compares each syntax theme palette with `packages/highlight/src/colors.ts` within 1 unit per channel, an unknown-id fallback test, and a missing-family fallback test
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-theme` -> exit 0
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> exit 0
