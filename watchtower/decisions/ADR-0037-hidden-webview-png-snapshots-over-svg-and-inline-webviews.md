# ADR-0037 Hidden WebView PNG snapshots over SVG export and live inline WebViews

Status: superseded by ADR-0067 (Mermaid only)
Date: 2026-10-15
Supersedes: ADR-0036
Scope: `packages/desktop-gpui/crates/diagram/**`, `packages/desktop-gpui/crates/timeline/**`

## Context

- A `gpui-wry` WebView is a native view placed over the GPUI surface. Inside a scrolled list it may not clip to the list bounds.
- The timeline can hold many mermaid and math blocks.
- GPUI rasterizes SVG with usvg 0.46, which has no `foreignObject` support. Mermaid's default labels are HTML inside `foreignObject`. KaTeX outputs HTML, not SVG. So the SVG export of [ADR-0036](watchtower/decisions/ADR-0036-hidden-webview-svg-diagrams-over-inline-webviews.md) cannot show labels or math as they are. Found by the plan builder check on 2026-10-15.
- KaTeX is already in the repo `node_modules`. MathJax is not.
- One WebView's WebKit helper processes used about 56 MB to 58 MB and 0% CPU when idle in the prototype.

## Decision

- One hidden WebView renders mermaid, with its default HTML labels, and KaTeX. It takes a PNG snapshot of each result at 2x scale. The timeline shows the PNG as an image inside the GPUI list.
- "Hidden" means placed outside the visible content, not `isHidden`, so WebKit still paints it for the snapshot.
- The owner chose this on 2026-10-15, after the builder check showed the SVG path drops HTML labels and math.

## Consequences

- Diagrams and math look the same as in a browser.
- The images are raster. They can look soft past 2x zoom.
- The diagram crate calls `WKWebView` `takeSnapshot` through objc2 on macOS, because wry has no snapshot API.
- Diagrams scroll and clip like any other row, and all diagrams share one WebKit process set.

## Options Considered

- SVG export with mermaid `htmlLabels: false` and MathJax SVG output: rejected. It needs a new MathJax package, mermaid labels lose markdown, and the owner preferred the browser look.
- A live WebView in each timeline row: rejected. Each row would need manual clipping and hiding while scrolling, and each diagram would add memory.

## Revisit If

- A WebView placed outside the visible content returns blank snapshots on the target macOS version.
- The owner reports soft diagrams at normal zoom.
- GPUI's SVG renderer gains `foreignObject` support.
