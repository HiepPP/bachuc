# ADR-0036 Hidden WebView SVG diagrams over live inline WebViews

Status: superseded by ADR-0037
Date: 2026-10-15
Supersedes: -
Scope: `packages/desktop-gpui/crates/diagram/**`, `packages/desktop-gpui/crates/timeline/**`

## Context

- A `gpui-wry` WebView is a native view placed over the GPUI surface. Inside a scrolled list it may not clip to the list bounds.
- The GPUI prototype rendered mermaid in one fixed WebView. Its WebKit helper processes used about 56 MB to 58 MB and 0% CPU when idle.
- The timeline can hold many mermaid and math blocks.

## Decision

- One hidden WebView renders mermaid and KaTeX sources to SVG. The timeline shows the result as an image inside the GPUI list.
- The owner chose this in the planning interview on 2026-10-15.

## Consequences

- Diagrams scroll and clip like any other row.
- Diagrams are static images. A user cannot pan or click inside them.
- All diagrams share one WebKit process set, so memory does not grow per diagram.

## Options Considered

- A live WebView in each timeline row: rejected. Each row would need manual clipping and hiding while scrolling, and each diagram would add memory.

## Revisit If

- The owner needs to interact with a diagram, such as pan, zoom, or click a node.
- GPUI gains a clipped embedded web view inside scroll containers.
