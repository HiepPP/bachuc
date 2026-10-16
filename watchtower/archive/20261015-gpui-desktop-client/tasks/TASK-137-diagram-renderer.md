# TASK-137 Diagram renderer with one hidden WebView

Group: C (writes only `crates/diagram`)
Class: risky

## Brief

Goal: Add the crate `bachuc-diagram`, which renders mermaid and KaTeX in one hidden WebView, takes a PNG snapshot of each result, and gives GPUI an image to show.

Change: one visible mermaid WebView in the prototype -> one hidden shared renderer that returns cached PNG images.

Boundaries:

- Follows [ADR-0037](watchtower/decisions/ADR-0037-hidden-webview-png-snapshots-over-svg-and-inline-webviews.md): PNG snapshots of one hidden WebView while `gpui-wry` views do not clip in scroll lists and GPUI's usvg drops `foreignObject` labels and KaTeX HTML.
- Keep mermaid's default HTML labels. Render math with KaTeX from the repo `node_modules/katex/dist`. Load `mermaid.min.js` as the prototype does with `BACHUC_MERMAID_JS`. Planner call: add `BACHUC_KATEX_DIR` in the same way. Bundling these files into the app is a later plan.
- "Hidden" means placed outside the visible content, not `isHidden`, so WebKit paints it. Prove this works first: if snapshots come back blank, stop and record it, because the ADR `Revisit If` then holds.
- Take snapshots with `WKWebView` `takeSnapshotWithConfiguration` through objc2 (`objc2-web-kit`), using the wry macOS extension to reach the `WKWebView`. Snapshot at 2x scale, cropped to the rendered element's bounds.
- Render requests run one at a time. Cache results by kind and source text. A source that fails gives `Failed` with the error text, and the timeline shows the code block instead.

How:

- Prove a snapshot of an off-content WebView returns a non-blank PNG.
- Create the hidden WebView with an init script that defines `renderMermaid` and `renderMath` and reports the element bounds through an IPC handler.
- Snapshot the bounds, build a GPUI image from the PNG bytes, and cache it.
- Add `examples/diagrams.rs` that shows 3 mermaid diagrams and 3 math blocks in a scroll list.

Files:

- [packages/desktop-gpui/crates/diagram/Cargo.toml](packages/desktop-gpui/crates/diagram/Cargo.toml) (new crate `bachuc-diagram`)
- [packages/desktop-gpui/crates/diagram/src](packages/desktop-gpui/crates/diagram/src) (renderer, snapshot, cache)
- [packages/desktop-gpui/crates/diagram/examples/diagrams.rs](packages/desktop-gpui/crates/diagram/examples/diagrams.rs) (scroll list example)

Interfaces:

- Produces: `bachuc_diagram::DiagramRenderer` (a GPUI entity) with `DiagramRenderer::new(window: &mut gpui_kit::Window, cx: &mut gpui_kit::Context<DiagramRenderer>) -> DiagramRenderer` for TASK-138, TASK-141
- Produces: `bachuc_diagram::DiagramKind` (enum `Mermaid`, `Math`) and `DiagramRenderer::render(&mut self, kind: DiagramKind, source: &str, cx: &mut gpui_kit::Context<DiagramRenderer>) -> DiagramState` with `DiagramState` (enum `Pending`, `Ready(std::sync::Arc<gpui_kit::Image>)`, `Failed(String)`) for TASK-138
- Produces: `DiagramRenderer` emits `DiagramReady { kind: DiagramKind, source: String }` when a pending render finishes, for TASK-138

Expected result:

- The example shows 3 diagrams with their labels and 3 formulas as images. They scroll and clip with the list.
- A broken mermaid source gives `Failed`, and the example shows the error text.
- A repeated source returns `Ready` at once from the cache, with no new WebView call.

Anti-goal: the example stays within 10% CPU and 250 MB phys_footprint, WebKit helpers included, while scrolling the list for 60 seconds; tripwire; read with `measure.sh 60` after the first snapshot works and at completion.

## Verify

- `packages/desktop-gpui/cargo.sh test -p bachuc-diagram` -> cache and failure tests pass
- Agent check: `packages/desktop-gpui/bundle.sh example diagrams BachucDiagrams io.github.hieppp.bachuc-diagrams`, open it, scroll with `cua_repl`, and take screenshots -> diagrams show their node labels and formulas show math symbols; they scroll and clip inside the list; no WebView shows over the content
- Agent check: `cua_repl` `getAXState` on the example -> each diagram image has a label with its kind
- Anti-goal: `measure.sh 60 diag="BachucDiagrams.app" wk="pids:<WebKit pids>"` while scrolling -> footprint sum at most 250, and CPU at most 1.0 point above a control run of the same scroll with each image replaced by an empty box. Lead change in the run of 2026-10-15 (Q-058): the old "CPU at most 10.0" line was UNSATISFIABLE, because the 60 fps scroll without any image used 9.6%. The 10% limit stays on the stream scenario of TASK-141.
