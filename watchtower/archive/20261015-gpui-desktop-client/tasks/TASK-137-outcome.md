# TASK-137 Outcome

Status: DONE

Builder status was BLOCKED: one completion read gave 10.2% CPU against the 10.0% limit. A re-read 4 minutes later gave 7.2%. A control run without diagram images gave 9.6%, so continuous 60 fps scrolling in GPUI costs about 9.5% on its own.

Lead decision, 2026-10-15 (Q-058, DEFAULTED): the scroll CPU line was UNSATISFIABLE as written. The spec now checks the diagram cost against the control run. Lead re-check: `cargo.sh test -p bachuc-diagram` 9 passed, `fmt --check` exit 0, and every Produces name exists in `src/lib.rs`.

## Changed

- `packages/desktop-gpui/crates/diagram/Cargo.toml` (new crate `bachuc-diagram`)
- `packages/desktop-gpui/crates/diagram/src/lib.rs` (public API, renderer entity, IPC events)
- `packages/desktop-gpui/crates/diagram/src/cache.rs` (one-at-a-time queue and result cache, with tests)
- `packages/desktop-gpui/crates/diagram/src/page.rs` (hidden page, init script, `bachuc-diagram://` file protocol)
- `packages/desktop-gpui/crates/diagram/src/snapshot.rs` (`WKWebView` `takeSnapshotWithConfiguration` to PNG)
- `packages/desktop-gpui/crates/diagram/examples/diagrams.rs` (scroll list example)
- `packages/desktop-gpui/Cargo.lock` (through cargo)

## Contract

- `bachuc_diagram::DiagramRenderer`, a GPUI entity: `DiagramRenderer::new(window: &mut gpui_kit::Window, cx: &mut gpui_kit::Context<DiagramRenderer>) -> DiagramRenderer`. Create it with `cx.new(|cx| DiagramRenderer::new(window, cx))`.
- `bachuc_diagram::DiagramKind` (`Mermaid`, `Math`) and `DiagramRenderer::render(&mut self, kind: DiagramKind, source: &str, cx: &mut gpui_kit::Context<DiagramRenderer>) -> DiagramState`. `DiagramState` is `Pending`, `Ready(std::sync::Arc<gpui_kit::Image>)`, or `Failed(String)`.
- `DiagramRenderer` emits `DiagramReady { kind: DiagramKind, source: String }` when a pending render finishes, as `Ready` or `Failed`.
- Extra: `bachuc_diagram::image_size(&gpui_kit::Image) -> gpui_kit::Size<gpui_kit::Pixels>` gives the logical size of a `Ready` PNG, which is half its pixel size. GPUI shows an `Image` at 1 logical px per image pixel, so set this size on the `img`.
- Extra: `DiagramKind::label()` returns "Mermaid diagram" or "Math formula" for accessibility labels.
- Env: `BACHUC_MERMAID_JS` (as in the prototype) and `BACHUC_KATEX_DIR` (planner call) override the repo `node_modules` paths. The page loads them through the custom protocol `bachuc-diagram://localhost/`, because KaTeX CSS needs its relative font URLs. The prototype put mermaid into the init script instead.
- The renderer reads `cx.theme().is_dark()` on each `render` call. When it changes, it drops the cache and renders again with mermaid `dark` or `default` and a matching KaTeX text color. Snapshots have a transparent background.

## Verified

- Snapshot proof (first step): a debug run wrote 6 PNG files. Each was non-blank, with RGBA and a transparent background, and at 2x size: the first diagram was 1546x348 px for a 773 CSS px box. Mermaid HTML labels and KaTeX symbols show. ADR-0037 `Revisit If` does not hold.
- `packages/desktop-gpui/cargo.sh test -p bachuc-diagram` -> 9 passed, 0 failed. These include the cache hit with no new job, `Failed` with the error text, one job at a time, the stale job after a theme change, the page file guard, IPC parsing, and `image_size`. PASS.
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-diagram` -> exit 0. PASS.
- `packages/desktop-gpui/cargo.sh clippy -p bachuc-diagram --all-targets` -> exit 0, no warnings from the crate. PASS.
- Agent check: `packages/desktop-gpui/bundle.sh example diagrams BachucDiagrams io.github.hieppp.bachuc-diagrams` -> exit 0. The app was opened with `--auto-scroll`, and `cua_repl` took screenshots at several scroll positions. Diagrams show node and edge labels, and formulas show math symbols. Images clip at the list top while they scroll. No WebView shows over the content. The broken source shows its code and the mermaid parse error. PASS.
- Agent check: `cua_repl` `getAXState` -> "image Mermaid diagram 1", "image Math formula 2", "image Mermaid diagram 3", "image Math formula 4", "image Mermaid diagram 5", "image Math formula 6", and "container Mermaid diagram 7, failed". PASS.
- Resize path: with the WebView height temporarily at 100 px, every diagram resized the WebView and gave the same PNG bytes as the 600 px run. The constant is back to 600.
- Verify Extras: `npm run lint` and `format:check:files` do not apply to Rust. The outcome file was formatted with `npm run format:files`.

## Anti-goal

The limits are 10.0% CPU and 250 MB phys_footprint, WebKit helpers included. Every read used `measure.sh 60 diag="BachucDiagrams.app" wk="pids:<3 WebKit pids matched by start time>"`. The release bundle ran with `--auto-scroll`, which scrolls the list at about 60 fps for the whole read. `cua_repl` raised the window every 2 s, because GPUI stops drawing while the window is covered.

- Checkpoint 1, after the first snapshot worked: CPU 3.3% (app 3.2 + WebKit 0.1), footprint 141 MB (61 + 80). Time: 2026-10-15 17:20:11. Within limits. This read is probably low, because the window was likely covered for part of the 60 s.
- Checkpoint 2, at completion: CPU 10.2% (app 10.2 + WebKit 0.0), footprint 142 MB (62 + 80). Time: 2026-10-15 17:26:00. Tripwire crossed on CPU.
- Control, about 17:28: the same bundle with each image swapped for an empty box of the same size gave CPU 9.6% (9.5 + 0.1) and footprint 105 MB (42 + 63). Diagram images add about 0.7 points of CPU while scrolling.
- Checkpoint 2 re-read, at completion: CPU 7.2% (7.1 + 0.1), footprint 201 MB (118 + 83). Time: 2026-10-15 17:29:49. Within limits.
- Result: FAIL. One completion read crossed the CPU tripwire. Footprint stayed under 250 MB in every read.
- Lead read against the changed Verify line, 2026-10-15: the worst read, 10.2%, is 0.6 points above the 9.6% control, within 1.0. The highest footprint is 201 MB, within 250. Result: PASS.

## Questions

- The scroll Anti-goal measures continuous 60 fps redraws. Without diagram images, GPUI alone used 9.6% CPU in this example. The plan baseline (6.5%) came from a stream at 10 updates/s. Should the scroll check use a higher CPU limit, or measure the stream scenario with diagrams? It should also measure on a window nothing covers.

## Lessons

- GPUI stops drawing while its window is covered. `cua_repl` screenshots then show the last frame, and `cua_repl` scroll events reach the window only when it is on top. Use `app.performSecondaryAction(0, "Raise")` before each screenshot. Drive scrolling from inside the app (the example's `--auto-scroll`), not with `cua_repl` scroll.
- WebKit stops `requestAnimationFrame` in a covered window. The init script does not use it, so renders do not stall.
- KaTeX fonts start to load only at layout. The page forces layout, then waits for `document.fonts.ready`. Without this, the first formula snapshot used fallback fonts.
- Mermaid diagrams render into a box 960 CSS px wide, so wider diagrams scale down. Show a `Ready` image at `image_size` with `max_w_full`, so it scales down further in a narrow column.
- The hidden WebView is 1000 CSS px wide and starts 600 px tall, left of the window. It grows when a result is taller and never shrinks.
- Renders run one at a time. A stuck render, such as a WebKit process that dies, blocks the queue. No timeout exists yet.
- The cache is keyed by kind and source and never evicts. A timeline with many diagrams keeps every PNG in memory.
- Build with `cargo.sh <cmd> -p bachuc-diagram` while another group's crate fails to build. A workspace member with no `src/lib.rs` stops every workspace command, even with `-p`.
