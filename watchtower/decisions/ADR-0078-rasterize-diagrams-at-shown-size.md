# ADR-0078 Rasterize Mermaid diagrams again at the size they show

Status: accepted
Date: 2026-10-30
Scope: `packages/desktop-gpui/crates/diagram/**`, `packages/desktop-gpui/crates/timeline/**`

## Context

- [ADR-0067](ADR-0067-pele-on-quickjs-over-hidden-webview-mermaid.md) rasterizes each Mermaid diagram once, to a PNG at 2x of the content font size.
- A wide diagram shrinks to the thread width, and the chart viewer opens at Fit, often 50% or less. The GPU then draws the PNG smaller than its pixel size.
- GPUI samples images with `min_filter::linear` and no mipmaps (`gpui-pre-apple-0.3.9/src/shaders.metal`). A downscale of more than about 2:1 skips texels, so text and 1 px lines look thin and broken. The PNG itself is sharp.
- resvg rasterizes the resolved SVG of a typical diagram in a few milliseconds, so a second raster at the shown width is cheap.

## Decision

- `bachuc-diagram` keeps the resolved SVG of each ready Mermaid diagram. `DiagramRenderer::fitted(source, width)` returns a PNG rasterized to the shown width in device pixels. It renders on the background executor.
- The thread measures the shown width of each diagram with a canvas. The chart viewer computes it from the natural size, the zoom, and the window scale factor, so a zoom above 100% is also sharp.
- Widths round up to 32 px. Each diagram rasterizes one fitted PNG at a time. The cache holds at most 16 fitted PNGs and clears on a theme change. A PNG over 32M pixels is not made, and the cached 2x PNG shows instead.
- KaTeX keeps its WebView snapshot. It has no SVG to rasterize again.
- Hiep checked the result by eye on the release build on 2026-10-30.

## Consequences

- Diagram text stays sharp at every thread width and viewer zoom. While a new width rasterizes, the cached PNG shows for a frame or two.
- Each ready Mermaid diagram keeps its SVG string in memory.
- A resize or a zoom triggers new rasters, one per 32 px step.

## Options Considered

- Mipmaps in the GPUI image sampler: rejected. It needs a patch to vendored GPUI shaders and atlas code, and a mipmapped downscale is still softer than a raster at the right size.
- Precomputed PNGs at 1x, 0.5x, and 0.25x: rejected. Sizes between the steps still skip texels, and zoom above 100% stays blurry.
- Thicker text, such as font weight 500 or a text stroke: rejected. It does not fix lines, and Pele measures labels with Helvetica widths, so a wider weight overflows the node boxes.

## Revisit If

- GPUI gains mipmapped or area-filtered image sampling.
- Fitted rasters cost more than a frame on large diagrams.
