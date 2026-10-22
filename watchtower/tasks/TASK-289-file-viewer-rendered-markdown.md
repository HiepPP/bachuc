# TASK-289 File viewer rendered markdown mode

Group: B (writes file_view.rs; no other TASK writes it; starts after TASK-290 for ordering only)
Class: code

## Brief

Goal: Show a markdown file in the GPUI file viewer as rendered markdown, with a toggle back to the source, as the Electron file pane does.

Change: `FileView` always shows numbered source lines -> a markdown file opens Rendered, and a toolbar button switches between Rendered and Source.

Design: [packages/app/src/file-pane/pane.tsx](packages/app/src/file-pane/pane.tsx) (`FilePreviewBody` shows the preview unless the file opens at a line or the mode is `source`)

Boundaries:

- Owner choice on 2026-10-22: match Electron. A markdown file opens Rendered. A file opened with a line (`line: Some(_)` in `FileView::new`) opens Source.
- When `scroll_to_line` runs on a Rendered file, switch to Source first, then scroll. The line target only exists in Source.
- Render with `TextView::markdown` in a vertical scroll area. Center the text in a column no wider than `MAX_CONTENT_WIDTH` from `bachuc_theme::metrics`, as the Electron reading frame does.
- Links use the `TextView` default link handler, which opens the URL. Planner call: relative links and local images are a non-goal.
- Non-goal: no front matter table and no HTML preview.
- The toggle is per view and lives for the view only. A reload keeps the current mode.
- Trap: `use gpui_kit::*` may not bring `TextView` in. Import `gpui_kit::component::text::TextView` as [packages/desktop-gpui/crates/explorer/src/pull_request_view.rs](packages/desktop-gpui/crates/explorer/src/pull_request_view.rs) does.
- Trap: `scroll_to_line` in `file_view.rs` scrolls the `UniformListScrollHandle`, which is not mounted while Rendered. After the switch to Source, make sure the scroll lands once the list draws, for example by keeping `line` and scrolling on the next frame.
- Trap: `render_toolbar` in `file_view.rs` takes `cx: &App` and has `Role::Status`. A button needs a listener from `Context<Self>`; change its parameter to `&mut Context<Self>` rather than adding a second bar.

How:

- Add a `rendered: bool` field to `FileView`, set in `new` from `is_markdown_path(&path) && line.is_none()`.
- Add a toggle button at the right of `render_toolbar`, shown only for a markdown text file, labeled "Show source" while Rendered and "Show rendered" while Source.
- In `render`, for `State::Text` while `rendered` is true, draw the markdown body instead of `render_lines`.

Files:

- [packages/desktop-gpui/crates/explorer/src/file_view.rs](packages/desktop-gpui/crates/explorer/src/file_view.rs) (the `rendered` field, the toolbar toggle, the rendered body)

Interfaces:

- Consumes: `bachuc_explorer::markdown_diff::is_markdown_path(path: &str) -> bool` from TASK-290

Expected result:

- WHEN a `.md` file opens without a line THE `FileView` SHALL show it rendered as markdown.
- WHEN a `.md` file opens with a line THE `FileView` SHALL show Source and scroll to that line.
- WHILE a `.md` file shows Rendered, WHEN `scroll_to_line` runs THE `FileView` SHALL switch to Source and scroll to the line.
- WHEN the owner clicks the toggle THE `FileView` SHALL switch between Rendered and Source.
- IF the file is not markdown THEN THE `FileView` SHALL show no toggle and draw source lines as before.

Anti-goal: a non-markdown text file draws the same toolbar text (size and line count) and the same numbered lines as before; tripwire; read from the AX tree of `file-view-bar` and `file-view-text` for a `.rs` file before the change and at completion.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-explorer` -> all tests pass.
- `cd packages/desktop-gpui && ./cargo.sh clippy -p bachuc-explorer -- -D warnings` -> no warnings.
- Screen check (UI checker, `cua_repl`, live GPUI build): open `README.md` from Files -> a heading renders as large text, not as a line starting with `#`; the toolbar has a button labeled "Show source".
- Screen check: press "Show source" with the AX action -> numbered lines show and the label becomes "Show rendered".
- Screen check: open a `.md` file at a line through the Command Center file search with a line, or a timeline file link with a line -> Source shows, scrolled to that line.
- Anti-goal: open a `.rs` file -> the AX tree has no toggle button, and `file-view-bar` shows the same size and line count text as before.
