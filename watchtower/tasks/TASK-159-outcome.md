# TASK-159 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/theme/src/lib.rs`: new setters and getters for font families, content size, and syntax theme; `apply` writes the families into the gpui-kit `Theme` global; unit tests for the family fallback and the content size clamp.
- `packages/desktop-gpui/crates/theme/src/syntax.rs` (new): `SyntaxThemeId`, `SyntaxColors`, `syntax_colors_for`, and the eight themes with light and dark palettes.
- `packages/desktop-gpui/crates/theme/tests/syntax_colors.rs` (new): reads `packages/highlight/src/colors.ts` and `themes.ts` and compares every theme and scheme.

## Contract

- `set_font_families(ui: Option<SharedString>, code: Option<SharedString>, cx: &mut App)`, `ui_font_family(cx: &App) -> SharedString`, `code_font_family(cx: &App) -> SharedString`.
- `set_content_font_size(px: f32, cx: &mut App)`, `content_font_size(cx: &App) -> Pixels`. Default 15 (`FONT_CONTENT`); floor, then clamp to 10-21 as `parseClampedFontSize` does; a non-finite value gives the default. Constants `DEFAULT_CONTENT_FONT_SIZE`, `MIN_CONTENT_FONT_SIZE`, `MAX_CONTENT_FONT_SIZE` are public.
- `installed_font_families(cx: &App) -> Vec<SharedString>`: sorted, deduplicated, names that start with `.` removed.
- `SyntaxThemeId` (`ALL`, `id`, `label`, `from_id`, `Default` = `One`), `set_syntax_theme`, `syntax_theme`, `syntax_colors(cx) -> SyntaxColors` (20 `Hsla` fields named as the roles; the `type` role is `r#type`). Extra: `syntax_colors_for(id, scheme)` for pure use, `SyntaxColors::entries()`, `SYNTAX_ROLES`.
- A family that is empty or not installed is stored as unset, so the getter returns the default (system font `.SystemUIFont`, or `mono_font_family`). The check runs when the family is set.
- `mono_font_family` and `set_font_scale` are unchanged. `apply` (private) now uses `code_font_family` and also sets `Theme.font_family`.
- Setters write the `Theme` global through `apply`, so observers redraw. Nothing is persisted.

## Verified

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-theme` -> 7 unit tests and 5 `syntax_colors` tests pass, including the 3 existing token tests and `turn_label_colors_match_the_app`.
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-theme` -> exit 0.
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> Finished, no error.
- UNVERIFIED (unattended): code tests only. No window was opened.

## Anti-goal

Existing token comparison tests (`light_tokens_match_theme_ts`, `claude_dark_tokens_match_theme_ts`, `tint_alphas_are_the_theme_ts_rule`) and `turn_label_colors_match_the_app` pass unchanged. Checked after the syntax port and at completion, 2026-10-16, source: `cargo test -p bachuc-theme`.

## Lessons

- `set_font_families` and the other cx setters have no test: a GPUI `App` needs `test-support`, which breaks workspace resolution. The fallback logic lives in pure functions (`installed_family`, `clamp_content_font_size`) that tests cover.
- The palette test parses the TS sources (`include_str!`), so a change in `themes.ts` or `colors.ts` fails the test until the Rust palettes follow.
- `Theme.font_family` was not set before; `apply` now sets `.SystemUIFont` when no override exists. On Linux desktops that lack the family, gpui-kit swaps it in `Theme::change`, and `apply` would undo that swap.

## Post-run fix (2026-10-17)

- Cause: the owner saw jerky, slow timeline scroll on long release threads. `mono_font_family`, which `code_font_family` falls back to, called `all_font_names` on every call. That builds a CoreText collection of every installed family, and the timeline and plugin rows call it on every row draw.
- Fix: commit `c1887709e` picks the family once per process in a `OnceLock`, with the rule in the pure `pick_mono_family` and the test `mono_family_prefers_sf_mono_and_falls_back_to_menlo`. Signature and result are unchanged; GitNexus rated the symbol CRITICAL (65 dependents) for that reason.
- Verified: `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-theme` -> 19 passed; the owner confirmed smooth scroll in the rebuilt app (2026-10-17).
- Still open: `installed_font_families` enumerates on each call; only the Settings font picker may call it.
