# TASK-303 Outcome

## Status

DONE. The Unicode PNG look is `DEFERRED (UI checker)`.

## Changed

- `packages/desktop-gpui/crates/diagram/src/svg.rs` (new): `PeleTheme`, `pele_theme`, `resolve_vars`, `rasterize`, `load_fonts`
- `packages/desktop-gpui/crates/diagram/src/pele.rs` (new): `PeleEngine`
- `packages/desktop-gpui/crates/diagram/src/lib.rs`: `mod pele;` and `mod svg;` only (TASK-304 edits the rest)
- `packages/desktop-gpui/crates/diagram/Cargo.toml`: `resvg 0.46` (features `text`, `system-fonts`, `memmap-fonts`), `bachuc-theme`
- `packages/desktop-gpui/Cargo.lock`: 2 lines (`bachuc-theme`, `resvg 0.46.0` under `bachuc-diagram`)

## Contract

- Produces: `pub(crate) struct PeleTheme` (`Clone + PartialEq + Debug`; fields as in the spec; `series: [String; 8]`).
- Produces: `pub(crate) fn pele_theme(tokens: &bachuc_theme::Tokens) -> PeleTheme`.
- Produces: `pub(crate) struct PeleEngine` with `pub(crate) fn spawn() -> PeleEngine`; it is also `Clone`.
- Produces: `pub(crate) fn render_png(&self, source: String, theme: PeleTheme) -> futures::channel::oneshot::Receiver<Result<Vec<u8>, String>>`.
- Also: `resolve_vars(&str, &PeleTheme) -> String`, `rasterize(&str) -> Result<Vec<u8>, String>`, `load_fonts()` (forces the one fontdb on the engine thread).
- The engine thread creates `Runtime`, `Context`, and the fontdb itself. It ends when every handle drops. A stopped engine replies `Err`.

## Verified

- Red: the spec tests were added first. They failed to build: missing `PeleTheme`, `pele_theme`, `resolve_vars`, `rasterize`, `PeleEngine`, `PELE_SOURCE_LIMIT`.
- `CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-diagram` -> 17 passed, 0 failed (6 new red tests, 9 old tests, 2 new cache tests from TASK-304). Group run after TASK-304.
- `./cargo.sh tree -p bachuc-diagram -i resvg@0.46.0` -> `resvg v0.46.0` is a dependency of `bachuc-diagram`; `-i resvg@0.45.1` -> nothing printed. The lock has no third resvg or usvg.
- `./cargo.sh clippy -p bachuc-diagram --no-deps --all-targets -- -D warnings` -> clean (run after TASK-304, so no `dead_code` allow was left).
- `./cargo.sh fmt --check -p bachuc-diagram` -> exit 0.
- Unicode check, code part: a temporary test rendered `Tiếng Việt`, `Xin chào, thế giới`, `Đúng không?` to `/tmp/pele-unicode.png`; I viewed it as an image: the labels show with diacritics, in a sans face, with no boxes. The temporary test is removed.
- UI checker: `DEFERRED (UI checker)`: look at `/tmp/pele-unicode.png` (dark theme, transparent background, so view on a light backdrop).

## Anti-goal

Tripwire: `bachuc-diagram` depends on `resvg 0.46.0` only. Source: `./cargo.sh tree -p bachuc-diagram -i resvg@0.46.0` and `-i resvg@0.45.1`, 2026-10-25.

- After adding `resvg`: the first `tree -i resvg` said the name is ambiguous (0.45.1 and 0.46.0 are both in the lock through GPUI), so I did not read a version-pinned result at that step. UNVERIFIED at that checkpoint.
- At completion: `-i resvg@0.46.0` lists `bachuc-diagram`; `-i resvg@0.45.1` prints nothing. The lock diff is the 2 lines above. PASS.

## Lessons

- The spec's red tests did not compile as written: `r#"fill="#e6e6e6""#` ends at `"#` inside the text. I changed only the raw-string delimiters of 4 assertions and the rasterize input to `r##"..."##`. No assertion text changed. The spec should use `r##` there.
- usvg stops reading a `style` attribute at the first `--_x:` custom property, so Pele's `font-family` (after them) was lost and text drew in a serif face. `resolve_vars` drops every `--_x:value;` declaration after it resolves them. Found by viewing the Unicode PNG.
- All `--_x` definitions sit in the root `<svg style>`, so one global map is enough. Unknown `--pele-*` names (`radius`, `heading-weight`) take their own fallback.
- The `Hsla` to `Rgba` conversion is gpui's `Rgba::from(Hsla)`; both come from `gpui_kit`.
- Pele's source limit is 50000 characters (`Y1=5e4` in the vendored script); the error text is "Diagram source is longer than the limit of 50000 characters."

Lead re-check (2026-10-25): `cargo.sh test -p bachuc-diagram` -> 17 passed; fmt and clippy `--no-deps --all-targets` clean; Produces names found with rg; `crates/timeline` has no diff. Committed in 4cb92d2a2.
