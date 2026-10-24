# Plan Context

## Shared Context

- Pele renders Mermaid source to SVG. It is TypeScript, MIT, by `obsidianmd`, version 1.10.0. Its global build `dist/pele.global.js` (about 690 KB) defines `var pele`, and `pele.render(source)` returns `{ svg }`.
- On Node (V8) on 2026-10-25, the planner measured 0.05 to 0.41 ms per diagram. QuickJS has no JIT, so it is slower. The speed on QuickJS is not measured yet.
- Pele's SVG has no `foreignObject`. It uses CSS custom properties: a root `style` defines `--_bg:var(--pele-bg,#fff)` and similar, and elements use `fill="var(--_fg)"`. usvg does not support `var()`, so the crate must replace each `var()` with a color before usvg parses the SVG. The planner did this with a string replace, and resvg 0.46 drew all four samples correctly in 1.2 to 3.8 ms at 2x.
- Charts use `--pele-series-1` to `--pele-series-8`.
- GPUI decodes `ImageFormat::Svg` at scale 1.0 (`Image::to_image_data` in `gpui-pre-0.3.9/src/platform.rs`), so an SVG image is soft on a retina screen. The crate rasterizes to PNG itself.
- `resvg 0.46.0` and `usvg 0.46.0` are already in [packages/desktop-gpui/Cargo.lock](packages/desktop-gpui/Cargo.lock) through GPUI. Another crate also pulls `resvg 0.45.1`. Use `resvg 0.46` in `bachuc-diagram`, so it reuses the 0.46.0 build.
- Latest `rquickjs` on crates.io is 0.14.0 (2026-09-18).
- Today, `mermaid.min.js` loads at run time from the repo `node_modules` (`mermaid_js` in [packages/desktop-gpui/crates/diagram/src/page.rs](packages/desktop-gpui/crates/diagram/src/page.rs)).
- Run cargo only through `./cargo.sh` in [packages/desktop-gpui](packages/desktop-gpui). Clean with `./cargo.sh clean --profile dev` after tests, unless another cargo process runs.

## Decisions

- Owner, 2026-10-25: vendor `pele.global.js` 1.10.0 into the crate and embed it with `include_str!`. Do not add an npm dependency.
- Owner, 2026-10-25: go only when the warm render p95 is 16 ms or less on QuickJS, and runtime creation plus Pele load is 300 ms or less. Otherwise stop the plan and keep ADR-0037.
- Owner, 2026-10-25: rasterize with resvg to a PNG at 2x. Keep `DiagramState`, `image_size`, and the timeline code as they are.
- Owner, 2026-10-25: when Pele fails, return `DiagramState::Failed`, so the row shows a code block. Remove `mermaid.min.js`; there is no mermaid.js fallback.
- Owner, 2026-10-25: write the ADR that supersedes ADR-0037 only after the spike passes.

## Open Decisions

- None.

## References

- [watchtower/decisions/ADR-0037-hidden-webview-png-snapshots-over-svg-and-inline-webviews.md](watchtower/decisions/ADR-0037-hidden-webview-png-snapshots-over-svg-and-inline-webviews.md)
- [docs/gpui.md](docs/gpui.md)
- https://github.com/obsidianmd/pele
