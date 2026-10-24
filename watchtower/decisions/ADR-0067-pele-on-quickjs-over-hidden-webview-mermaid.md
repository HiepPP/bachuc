# ADR-0067 Pele on QuickJS over the hidden WebView for Mermaid

Status: accepted
Date: 2026-10-25
Supersedes: ADR-0037 (Mermaid only)
Scope: `packages/desktop-gpui/crates/diagram/**`, `packages/desktop-gpui/crates/timeline/**`

## Context

- [ADR-0037](ADR-0037-hidden-webview-png-snapshots-over-svg-and-inline-webviews.md) renders Mermaid and KaTeX in one hidden WebView and takes a PNG snapshot of each. Mermaid loads `mermaid.min.js` at run time from the repo `node_modules` (`mermaid_js` in `crates/diagram/src/page.rs`).
- Pele (`obsidianmd/pele`, MIT, 1.10.0) renders Mermaid source to SVG. Its SVG has no `foreignObject`, so the ADR-0037 reason against SVG export no longer holds for Mermaid. It colors with CSS custom properties (`var(--_fg)`), which usvg does not support, so the crate must replace each `var()` with a color before resvg parses the SVG.
- `resvg 0.46.0` and `usvg 0.46.0` are already in `packages/desktop-gpui/Cargo.lock` through GPUI. GPUI decodes `ImageFormat::Svg` at scale 1.0 (`gpui-pre-0.3.9/src/platform.rs`), so an SVG image is soft on a retina screen.
- TASK-301 measured Pele 1.10.0 on `rquickjs 0.14` (QuickJS, release build, 3 runs on 2026-10-25, [TASK-301 outcome](watchtower/tasks/TASK-301-outcome.md)): runtime plus Pele load 117 to 142 ms, overall render p95 7.44 to 9.58 ms over 500 renders. All 10 sample diagrams rendered. Verdict: `GO`.
- The gate set by the owner on 2026-10-25 was a warm render p95 of 16 ms or less and a load of 300 ms or less. Both hold.
- The 40-node flowchart sets the overall p95. Its own p95 was 11.2 to 22.6 ms, and its p50 7.4 to 9.5 ms. A larger real diagram can cross 16 ms.
- KaTeX outputs HTML, not SVG, and has no Pele equivalent. It still needs the hidden WebView.
- [ADR-0045](ADR-0045-native-rust-plugin-clients-over-webview-panes-or-shared-logic.md) bans a hidden WebView or embedded JS engine for plugin clients. `bachuc-diagram` is a host crate, not a plugin client, so ADR-0045 does not apply.

## Decision

- Mermaid renders through Pele on QuickJS (`rquickjs 0.14`) inside `bachuc-diagram`. The crate replaces each `var()` with a color, rasterizes the SVG with resvg 0.46 to a PNG at 2x, and hands the PNG to the timeline as before. `DiagramState`, `image_size`, and the timeline code stay as they are.
- `pele.global.js` 1.10.0 is vendored in `packages/desktop-gpui/crates/diagram/assets/pele/` and embedded with `include_str!`. There is no npm dependency. The folder `crates/diagram/assets/**` is excluded from oxlint, oxfmt, and the lint hook (commit `447a1b36a`).
- When Pele fails on a diagram, the state is `DiagramState::Failed` and the row shows a code block. There is no mermaid.js fallback, and `mermaid.min.js` is removed.
- KaTeX keeps the hidden WebView PNG snapshot of ADR-0037. Only the Mermaid part of ADR-0037 is superseded.
- Hiep chose this on 2026-10-25 in the watchtower interview, after TASK-301 reported `GO`.

## Consequences

- Mermaid no longer needs the WebView, so it draws without the WebView startup and without `mermaid.min.js` in `node_modules`.
- The WebView stays for KaTeX, so the objc2 snapshot code stays.
- A Mermaid syntax Pele does not accept shows as a code block until Pele supports it.
- The crate adds `rquickjs` and a 709 KB vendored script. Updating Pele means replacing the vendored file by hand.
- Rendering must run off the UI thread, because one large diagram can take longer than a frame.

## Options Considered

- mermaid.js in the hidden WebView (ADR-0037): rejected for Mermaid. It needs a WebView and a script from `node_modules` at run time, while Pele on QuickJS loaded in 117 to 142 ms and rendered at an overall p95 of 7.4 to 9.6 ms. Kept for KaTeX.
- Pele in the hidden WebView: rejected. It keeps the WebView and its snapshot path for Mermaid, which Pele's `foreignObject`-free SVG makes unnecessary.
- Pele on the daemon over a new RPC: rejected. It adds a protocol message and a daemon dependency for a render that runs locally in 0.3 to 12 ms, and the daemon may be remote.
- A native Rust Mermaid layout: rejected. It means porting layout for every diagram type that agents write. Pele already covers flowchart, sequence, class, state, ER, gantt, pie, xychart, and mindmap.

## Revisit If

- Pele stops releasing, or breaks Mermaid syntax that agents write.
- QuickJS render p95 grows past 16 ms.
- GPUI's image path gains a scale-aware SVG render.
