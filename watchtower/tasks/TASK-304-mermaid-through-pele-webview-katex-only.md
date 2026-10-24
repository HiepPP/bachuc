# TASK-304 Mermaid through Pele, WebView for KaTeX only

Group: A (writes `crates/diagram/src/lib.rs`, shared with TASK-303)
Class: risky

## Brief

Goal: Render every Mermaid block through `PeleEngine`, and keep the hidden WebView only for KaTeX.

Change: Mermaid jobs run mermaid.js in the WebView and take a snapshot -> Mermaid jobs go to `PeleEngine` and finish through the same `finish` path; `mermaid.min.js` is gone.

Boundaries:

- Follows ADR-0067 from TASK-302.
- Keep the public API: `DiagramRenderer::new`, `DiagramRenderer::render`, `DiagramState`, `DiagramReady`, `DiagramKind`, and `image_size`. The timeline in `markdown.rs` and the `diagrams` example must not change.
- When Pele fails, finish the job with `Err`, so the row shows a code block. No mermaid.js fallback.
- Theme: compute `pele_theme(&bachuc_theme::tokens(cx))` in `render`. When it differs from the stored theme, call `cache.clear()`. This replaces the `dark` check for Mermaid and also covers custom themes. KaTeX still needs `dark`.
- Trap: `start_next` in `lib.rs` returns early while `ready` is false. A Mermaid job must not wait for the WebView.
- Trap: `next_job` in `cache.rs` pops the queue front, whatever its kind. Before the WebView is ready, a Math job at the front must not block a Mermaid job behind it, and must not be popped and lost.
- Trap: `WEBVIEW_WIDTH` in `lib.rs` uses `page::MERMAID_WIDTH`. When you remove `MERMAID_WIDTH`, give the WebView a fixed width for KaTeX that keeps formulas unclipped, and keep `hidden_bounds` working.
- Trap: `Event::Rendered` and the tall-WebView resize in `snapshot` stay for Math. Do not delete them.
- Planner call: keep one running job at a time for both kinds, as `running: Option<Job>` does today. A Pele render takes milliseconds, so a second slot is not worth it. The only change: while the WebView is not ready, the cache may start a Mermaid job from behind queued Math jobs.
- Planner call: a theme change while a Mermaid job runs uses the existing stale path (`running_is_stale`), so that job renders again. Add a cache test for a Mermaid job.
- The prototype crate has its own mermaid code. Do not touch it.

How:

- In `DiagramRenderer`, hold a `PeleEngine` (spawned in `new`) and the last `PeleTheme`.
- In `start_next`, send a Mermaid job to `render_png`. Spawn a task that awaits the reply and sends `Event::Snapshot { id, result }` on `events`.
- Change `RenderCache` so a Mermaid job can start while the WebView is not ready, with a test for it.
- In `page.rs`, remove the mermaid script tag, `renderMermaid`, the `#d.mermaid` style, `MERMAID_WIDTH`, `mermaid_js`, and the `/mermaid.min.js` route; update the `serves_only_the_page_and_its_files` test.

Files:

- [packages/desktop-gpui/crates/diagram/src/lib.rs](packages/desktop-gpui/crates/diagram/src/lib.rs) (route Mermaid jobs to `PeleEngine`, theme check)
- [packages/desktop-gpui/crates/diagram/src/cache.rs](packages/desktop-gpui/crates/diagram/src/cache.rs) (start a Mermaid job before the WebView is ready)
- [packages/desktop-gpui/crates/diagram/src/page.rs](packages/desktop-gpui/crates/diagram/src/page.rs) (remove mermaid)
- [packages/desktop-gpui/crates/diagram/src/pele.rs](packages/desktop-gpui/crates/diagram/src/pele.rs) and [packages/desktop-gpui/crates/diagram/src/svg.rs](packages/desktop-gpui/crates/diagram/src/svg.rs) (remove `TODO(TASK-304)` allows only)

Interfaces:

- Consumes: `pub(crate) struct PeleTheme` with `Clone + PartialEq + Debug` and `pub(crate)` fields `bg, fg, muted, line, surface, surface_alt, border, accent, font, font_mono: String` and `series: [String; 8]` from TASK-303
- Consumes: `pub(crate) fn pele_theme(tokens: &bachuc_theme::Tokens) -> PeleTheme` from TASK-303
- Consumes: `pub(crate) struct PeleEngine` with `pub(crate) fn spawn() -> PeleEngine` from TASK-303
- Consumes: `pub(crate) fn render_png(&self, source: String, theme: PeleTheme) -> futures::channel::oneshot::Receiver<Result<Vec<u8>, String>>` on `PeleEngine` from TASK-303

Expected result:

- WHEN the timeline asks for a Mermaid source THE renderer SHALL return `Pending`, then emit `DiagramReady` with a `Ready` PNG made by Pele.
- WHILE the WebView is not ready, WHEN a Mermaid job is queued behind a Math job THE cache SHALL start the Mermaid job and keep the Math job queued.
- IF Pele fails on a source THEN THE renderer SHALL store `Failed` with Pele's error text.
- WHEN the theme tokens change THE renderer SHALL drop cached results and render each visible diagram again in the new colors.
- WHEN the timeline asks for a KaTeX source THE renderer SHALL render it in the WebView as before.

Red tests:

File: [packages/desktop-gpui/crates/diagram/src/cache.rs](packages/desktop-gpui/crates/diagram/src/cache.rs)

Planner call: change `next_job` to `next_job(&mut self, webview_ready: bool) -> Option<Job>`, and update its callers and tests. Write these tests:

- Queue Math `x` then Mermaid `A`. `next_job(false)` returns the Mermaid job. After `finish`, `next_job(false)` returns `None`, and `next_job(true)` returns the Math job.
- Queue Mermaid `A`, start it, call `clear`, then `finish` it: the result is `None`, and the next `next_job(false)` returns Mermaid `A` again.

Anti-goal: each `DiagramRenderer::render` call on the main thread takes 1 ms or less, also for a new Mermaid source; tripwire; read with a temporary `Instant` log in `examples/diagrams.rs` before changes, after the wiring, and at completion; remove the log before the commit.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-diagram -p bachuc-timeline` -> all pass, including the new cache test.
- `rg -n "mermaid.min.js|renderMermaid|MERMAID_WIDTH" packages/desktop-gpui/crates/diagram/src` -> no match.
- `git diff --stat packages/desktop-gpui/crates/timeline` -> empty.
- Screen check (UI checker): `./cargo.sh run -p bachuc-diagram --example diagrams` -> the flowchart, sequence, and state diagrams in `BLOCKS` show with sharp labels inside their boxes; a bad source shows the code block with Pele's error; the KaTeX formula shows. Switch light and dark -> diagrams render again in the new colors.
- Anti-goal log in the example -> every `render` call is 1 ms or less; values go in the outcome sidecar.
