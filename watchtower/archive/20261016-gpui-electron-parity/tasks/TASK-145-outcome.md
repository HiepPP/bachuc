# TASK-145 Outcome

## Status

DONE. Screen checks are deferred to the UI checker.

## Changed

- `packages/desktop-gpui/crates/theme/Cargo.toml` (dev-dependency on `bachuc-icons`)
- `packages/desktop-gpui/crates/theme/src/lib.rs` (`ThemeChoice`, `set_theme`, `set_font_scale`, `tokens`, shadows, gpui-kit color mapping)
- `packages/desktop-gpui/crates/theme/src/tokens.rs` (new: `Tokens`, `light()`, `claude_dark()`, comparison tests)
- `packages/desktop-gpui/crates/theme/src/metrics.rs` (new)
- `packages/desktop-gpui/crates/theme/examples/tokens.rs` (new)
- `packages/desktop-gpui/crates/icons/` (new crate `bachuc-icons`: `Cargo.toml`, `src/lib.rs`, generated `src/names.rs`, `codegen/generate-icons.mjs`, 184 `assets/*.svg`)
- `packages/desktop-gpui/Cargo.lock` (generated)

## Contract

- `bachuc_theme::Tokens`: every field in the spec, plus nothing else. `Tokens::light()` and `Tokens::claude_dark()` build it. `ansi` order: black, red, green, yellow, blue, magenta, cyan, white, then the eight bright colors.
- `bachuc_theme::tokens(cx: &App) -> Tokens`: tokens of the theme on screen. Works before `init` (falls back to the constructors).
- `bachuc_theme::ThemeChoice { Light, ClaudeDark, System }` (default `System`), `set_theme(choice, cx)`, `set_font_scale(interface, code, cx)`.
  - The two scales are factors of the defaults (1.0 = 14 px interface, 12 px code). A settings screen that stores pixels passes `px / 14.0` and `px / 12.0`.
  - `ui_font_size(base, cx)` and `code_font_size(base, cx)` return the scaled `Pixels`. A view should call them for text sizes it takes from `metrics::FONT_*`.
- `bachuc_theme::init(cx)` still works: it applies `System` and follows window appearance changes only while the choice is `System`.
- `bachuc_theme::metrics`: every `pub const` f32 in the spec. Extras: `WEIGHT_*`, `BORDER_WIDTH_1/2`, `Shadow`, `LIGHT_SHADOWS`, `DARK_SHADOWS`.
- `bachuc_theme::shadow(ShadowSize::{Sm,Md,Lg}, cx) -> BoxShadow`; `mono_font_family(cx)` picks `SFMono-Regular`, else `SF Mono`, else `Menlo`.
- `Scheme`, `scheme`, `TurnLabel`, `turn_label_colors` unchanged. `Scheme::Dark` is `claude` dark.
- `bachuc_icons::IconName`: 190 variants named as the lucide import, with `IconName::ALL` and `path()` (`lucide/<file>.svg`).
- `bachuc_icons::icon(name) -> gpui_kit::component::Icon`, sized `ICON_MD`.
- `bachuc_icons::Assets`: an `AssetSource`. A caller must register it: `gpui_kit::application().with_assets(bachuc_icons::Assets)`. Without it no icon draws (no existing example or `crates/app` registers assets yet; TASK-155 owns `crates/app`).
- Icons need `bachuc-icons = { path = "../icons" }` in the consumer `Cargo.toml`.

## Verified

- `cargo.sh test -p bachuc-theme` -> PASS: 4 tests, including `light_tokens_match_theme_ts` and `claude_dark_tokens_match_theme_ts`, which read `packages/app/src/styles/theme.ts` at test time and compare within 1 unit per channel and 0.01 alpha.
- `cargo.sh test -p bachuc-icons` -> PASS: 5 tests (every `IconName` loads an SVG, `circle` and `line` nodes survive, a gpui-kit path falls through, sorted table).
- Generator twice -> PASS: second run `changed: 0`; 190 imported names, 184 SVG files, 0 missing.
- `cargo.sh test --workspace` -> PASS: exit 0, 22 result lines, 0 failures.
- `cargo.sh fmt --check -p bachuc-theme -p bachuc-icons` -> PASS.
- `npm run lint -- .../generate-icons.mjs` and `format:check:files` -> PASS.
- Agent check (`bundle.sh example tokens ...`, screenshots in both themes) -> DEFERRED (UI checker). A debug build of the example compiles.

## Anti-goal

- Tripwire: no Electron token changed or invented.
- Checkpoint `token-comparison`: PASS after the themes landed, and PASS at completion (2026-10-16, `cargo.sh test -p bachuc-theme`, source `theme.ts`).
- One value is not in `theme.ts`: `tint_merged`. The spec lists it, `statusTints` has no merged entry, so it is `status_merged` at the alpha of the other tints (`1f` light, `29` dark). The test checks it against that rule.

## Parity list

Not asked by the spec.

## Lessons

- Icons: `crates/icons/src/names.rs` is generated and skipped by rustfmt (`#[rustfmt::skip] mod names;`). Do not edit it. Re-run `node packages/desktop-gpui/crates/icons/codegen/generate-icons.mjs` when the app imports a new lucide icon. Aliases such as `Loader2` and `LoaderCircle` share one SVG.
- `npm run lint -- <file>` through the rtk wrapper can print "ESLint output (JSON parse failed)". Redirect the output to a file and read the exit code; `oxlint` then reports "0 warnings and 0 errors".
- `Theme::change` reloads gpui-kit defaults, so colors must be written in a second `Theme::update` (`apply` in `lib.rs` does this). Any code that calls `Theme::change` or `sync_system_appearance` itself drops the Electron colors; call `bachuc_theme::set_theme` instead.
- The theme crate dev-depends on `bachuc-icons`, which depends on the theme crate (a dev-dependency cycle that cargo allows).

## UI Checks

- Line: `bundle.sh example tokens BachucTokens io.github.hieppp.bachuc-tokens`, open, `cua_repl` screenshots in both themes -> PASS (2026-10-16, UI checker).
  - The example window shows the light panel and the claude dark panel side by side. Each has all 41 swatches plus 16 ANSI colors, 7 type sizes, 40 icons at 14 px, 40 icons at 16 px, and the accent button. Light accent is green, dark accent is orange.
  - Icons draw with clean 1 px strokes and no blur at 14 and 16 px in both panels. The screenshot is at window scale (1180 px wide), so sharpness is judged at that scale, not on a retina zoom.
  - Limit: the System, Light, and Claude dark buttons took a click, but the toolbar and window chrome did not repaint. The example view does not re-render on `set_theme`. This does not affect the swatch and icon check, which uses `Tokens::light()` and `Tokens::claude_dark()` directly. A real screen must notify its view on a theme change.
  - Screenshots: `/tmp/gpui-ui-check/A/tokens-system.png`, `/tmp/gpui-ui-check/A/tokens-after-light.png`, `/tmp/gpui-ui-check/A/tokens-after-dark.png`, `/tmp/gpui-ui-check/A/tokens-dark-selected.png`.
