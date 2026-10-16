# TASK-158 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/icons/codegen/generate-provider-logos.mjs` (new): reads the Electron glyph components and the GLM SVG, writes `assets/providers/*.svg`.
- `packages/desktop-gpui/crates/icons/assets/providers/{claude,codex,copilot,glm,minimax,omp,opencode,pi}.svg` (new, generated).
- `packages/desktop-gpui/crates/icons/src/providers.rs` (new): the logo table, cache, fallback icon, and tests.
- `packages/desktop-gpui/crates/icons/src/lib.rs`: `mod providers` and a `pub use` of the three functions. Every earlier public item is unchanged.
- `crates/composer/assets/claude.svg` is untouched; TASK-163 deletes it.

## Contract

- `bachuc_icons::provider_logo_svg(provider: &str) -> Option<&'static str>`: the SVG template with `{FILL}` and `{OPACITY}` placeholders. `None` for `kiro` and unknown ids.
- `bachuc_icons::provider_logo(provider: &str, fill: gpui_kit::Hsla) -> Option<Arc<gpui_kit::Image>>`: fill written into the SVG text (`#rrggbb` plus `fill-opacity`), `ImageFormat::Svg`, cached per provider and color (the same `Arc` for equal calls).
- `bachuc_icons::provider_fallback_icon(provider: &str) -> IconName`: `PackagePlus` for `kiro`, else `Bot`.
- Logo ids: claude, codex, copilot, glm, minimax, omp, opencode, pi. The templates carry `width="24" height="24"` and the source `viewBox`; the opencode back rectangle keeps `opacity="0.4"`.

## Verified

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-icons` -> 12 passed, 0 failed.
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-icons` -> exit 0.
- `node .../generate-provider-logos.mjs` twice -> first run changed 8, second run changed 0.
- `npm run lint -- packages/desktop-gpui/crates/icons/codegen/generate-provider-logos.mjs` -> exit 0, 0 warnings, 0 errors.
- Drawing the images in a window: UNVERIFIED (unattended): code tests only.

## Anti-goal

- Provider assets under 200 KB: `du -sh packages/desktop-gpui/crates/icons/assets/providers` -> 32K (2026-10-16 16:02, local).

## Lessons

- Electron's `OpenCodeIcon` draws a 0.4-opacity back rectangle; the root `fill-opacity` placeholder multiplies with it, so a translucent fill stays correct.
- `copilot` uses a non-square `viewBox` (512x416); the templates keep each source `viewBox`.
- The generator parses the `.tsx` components with regexes; a component that adds a non-`Path` element makes it fail loudly only if it has no `<Path>`, so extend it when one appears.
- The cache test uses `Arc::ptr_eq`; the cache is process-global, so tests use distinct colors per case.
