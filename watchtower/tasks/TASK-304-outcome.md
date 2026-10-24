# TASK-304 Outcome

## Status

DONE. Every screen check is `DEFERRED (UI checker)`.

## Changed

- `packages/desktop-gpui/crates/diagram/src/lib.rs`: `DiagramRenderer` holds a `PeleEngine` and the last `PeleTheme`; `start_next` sends a Mermaid job to `render_png` and a task turns the reply into `Event::Snapshot`; `render` clears the cache when `pele_theme(&tokens(cx))` or `dark` changes; `WEBVIEW_WIDTH` is a fixed 1000.0.
- `packages/desktop-gpui/crates/diagram/src/cache.rs`: `next_job(&mut self, webview_ready: bool)`; two new tests; old tests call `next_job(true)`.
- `packages/desktop-gpui/crates/diagram/src/page.rs`: mermaid script tag, `renderMermaid`, `#d.mermaid` style, `MERMAID_WIDTH`, `mermaid_js`, and the `/mermaid.min.js` route removed; `serves_only_the_page_and_its_files` loses its mermaid assertion.
- `pele.rs` and `svg.rs`: no `TODO(TASK-304)` allow was ever committed. TASK-303 added `#![allow(dead_code)]` with the TODO comment, and this TASK removed it.
- `crates/timeline` and `examples/diagrams.rs`: unchanged.

## Contract

- Consumes: `PeleTheme`, `pele_theme`, `PeleEngine::spawn`, `render_png` from TASK-303, with the names and types of the spec.
- Public API unchanged: `DiagramRenderer::new`, `render`, `DiagramState`, `DiagramReady`, `DiagramKind`, `image_size`.
- Internal: `RenderCache::next_job(&mut self, webview_ready: bool) -> Option<Job>`. With `webview_ready == false` it takes the first queued Mermaid job and leaves Math jobs queued. Callers: `start_next` and the cache tests only (GitNexus impact said HIGH, but every caller is in this crate; checked with `rg`).

## Verified

- Red: the two cache tests plus the call updates failed to build against `next_job()` (E0061, signature mismatch).
- `CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-diagram -p bachuc-timeline` -> bachuc-diagram 17 passed, 0 failed; bachuc-timeline all suites ok, 0 failed. Includes `mermaid_job_starts_before_the_webview_is_ready_and_math_stays_queued` and `cleared_mermaid_job_renders_again_before_the_webview_is_ready`.
- `rg -n "mermaid.min.js|renderMermaid|MERMAID_WIDTH" packages/desktop-gpui/crates/diagram/src` -> no match.
- `git diff --stat packages/desktop-gpui/crates/timeline` -> empty.
- `./cargo.sh clippy -p bachuc-diagram --no-deps --all-targets -- -D warnings` -> clean. `-p bachuc-timeline` fails on 3 old lines (`plugin_view.rs:185`, `lib.rs:376`, `lib.rs:3355`); it is a Known Failure and `crates/timeline` has no diff.
- `./cargo.sh fmt --check -p bachuc-diagram` -> exit 0.
- UI checker: `DEFERRED (UI checker)`: `./cargo.sh run -p bachuc-diagram --example diagrams`: flowchart, sequence, state show with sharp labels in their boxes; the bad source shows a code block with Pele's error; the KaTeX formula shows; switch light and dark and see the diagrams render again in the new colors.

## Anti-goal

Tripwire: each `DiagramRenderer::render` call on the main thread takes 1 ms or less. Source: a temporary `Instant` around `renderer.render(..)` in `examples/diagrams.rs`, printed to stderr (debug build, 2026-10-25). `cx.activate(true)` was commented out for the runs, so the window did not take focus. I killed the process after the log printed. The log is removed and the example is restored (`git status` shows no change to it).

- Before changes (WebView path): 14 calls, max 0.003 ms, all `Pending`.
- After the wiring (Pele path): 14 calls, max 0.060 ms, all `Pending`. The first call of a run, a new Mermaid source, includes `pele_theme` and the channel send.
- At completion: not read again. After the wiring read I changed only a comment in `pele.rs`; `render` is untouched. UNVERIFIED as a separate read.
- The unfocused window drew only 2 frames, so no call hit the `Ready` cache path and no Pele reply was seen on screen. That path is a hash lookup plus `pele_theme`. Left to the UI checker: `DEFERRED (UI checker)` for a log of a `Ready` call.

## Lessons

- `pele_theme` runs on every `render` call (about 20 small strings). It costs microseconds here. If the timeline calls `render` for many rows per frame, compare `Tokens` first and build the theme only on a change.
- While the WebView is not ready, a Math job waits and a Mermaid job starts. `finish` then calls `start_next`, so queued Mermaid jobs run back to back.
- The theme check clears the Math cache too, because both kinds share the cache and `clear`. Cost: a custom-theme change re-renders formulas once.
- A cargo run of the example never gets a visible frame without focus here; screen timings need the UI checker.

Lead re-check (2026-10-25): `cargo.sh test -p bachuc-diagram` -> 17 passed; fmt and clippy `--no-deps --all-targets` clean; Produces names found with rg; `crates/timeline` has no diff. Committed in 4cb92d2a2.
