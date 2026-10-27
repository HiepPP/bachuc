# TASK-318 Code citation card in the GPUI answer

Group: C (timeline `lib.rs` and `links.rs`; ordered after TASK-317 only, no shared file)
Class: code

## Brief

Goal: A code block whose fence meta names a file and lines shows a Cursor-style header, "lib.rs Ln 2331-2334", and a click opens that file at that line.

Change: a code block shows only the code and a hover copy button -> a citation block also shows a header with the file icon, the file name, and the line range.

Design: Cursor answer screenshots in this thread, 2026-10-27: a bordered card, file icon and name, muted "Ln a-b", then the highlighted code.

Boundaries:

- Q-197 took the default fence format: ` ```<lang> <path>:<start>-<end> ` or ` ```<lang> <path>:<line> `. The path is repo-relative or absolute. CommonMark renderers ignore the meta, so Electron still shows a plain code block.
- Reuse `parse_inline_path_token` in `links.rs` to read the meta. Do not write a second path parser.
- A click emits the existing `TimelineEvent::OpenFile` through `open_link`-style resolution with the thread `cwd`, like a file link in the text.
- Keep the hover copy button from `code_block_actions`.
- Trap: `render_markdown` in `lib.rs` builds the `RichText` for every markdown block of every message. Do the meta parse inside the hook per block; do not add a pass over all messages.
- Trap: a meta that is not a path, for example ` ```js title="x" `, must give no header, so the block looks as before.
- Trap: `parse_inline_path_token` in `links.rs` also accepts `path line 5`, `path(12)`, and reads `title="a:1"` as the path `title="a` with line 1. `code_citation` accepts only one token `path:N` or `path:N-M`, with no whitespace, `=`, or quote.

How:

- Add `pub fn code_citation(meta: &str) -> Option<FileTarget>` in `links.rs` on top of `parse_inline_path_token`; it needs a line number.
- In `render_markdown` in `lib.rs`, set `code_block_header` to return a header row for a block whose `meta()` gives a citation, else `None`.
- Show the file name with `bachuc_explorer::file_icon_element(file_name, cx)` from `explorer/src/ui.rs` (the timeline already depends on `bachuc-explorer`), and "Ln a-b" in the muted color.

Files:

- [packages/desktop-gpui/crates/timeline/src/links.rs](packages/desktop-gpui/crates/timeline/src/links.rs) (`code_citation`)
- [packages/desktop-gpui/crates/timeline/src/lib.rs](packages/desktop-gpui/crates/timeline/src/lib.rs) (header hook in `render_markdown`)
- [packages/desktop-gpui/crates/timeline/tests/links.rs](packages/desktop-gpui/crates/timeline/tests/links.rs) (tests)

Interfaces:

- Consumes: `CodeBlock::meta(&self) -> Option<SharedString>` from TASK-317
- Consumes: `TextView::code_block_header<F>(self, f: F) -> Self` where `F: Fn(&CodeBlock, &mut Window, &mut App) -> Option<AnyElement> + Send + Sync + 'static` from TASK-317

Expected result:

- WHEN the meta is `src/lib.rs:2331-2334` THE `code_citation` function SHALL return path `src/lib.rs`, start 2331, end 2334.
- WHEN the meta is `src/lib.rs:12` THE `code_citation` function SHALL return start 12 and no end.
- IF the meta is `title="x"`, `title="a:1"`, `src/lib.rs line 5`, `src/lib.rs(12)`, `src/lib.rs:10 title="x"`, a bare path with no line, or empty THEN THE `code_citation` function SHALL return `None`.
- WHEN an answer has a citation fence THE timeline SHALL draw the header "lib.rs Ln 2331-2334" above the code.
- WHEN Hiep clicks the header THE timeline SHALL emit `OpenFile` with the cited path and line 2331.

Red tests:

File: [packages/desktop-gpui/crates/timeline/tests/links.rs](packages/desktop-gpui/crates/timeline/tests/links.rs)

```rust
#[test]
fn a_fence_meta_with_a_path_and_lines_is_a_code_citation() {
    let range = bachuc_timeline::links::code_citation("src/lib.rs:2331-2334").unwrap();
    assert_eq!(range.path, "src/lib.rs");
    assert_eq!(range.line_start, Some(2331));
    assert_eq!(range.line_end, Some(2334));

    let line = bachuc_timeline::links::code_citation("src/lib.rs:12").unwrap();
    assert_eq!(line.line_start, Some(12));
    assert_eq!(line.line_end, None);

    for meta in [
        "title=\"x\"",
        "title=\"a:1\"",
        "src/lib.rs line 5",
        "src/lib.rs(12)",
        "src/lib.rs:10 title=\"x\"",
        "src/lib.rs",
        "",
        "  ",
    ] {
        assert_eq!(bachuc_timeline::links::code_citation(meta), None, "{meta:?}");
    }
}
```

Anti-goal: a message with no citation fence renders the same rows; tripwire; read with `./cargo.sh test -p bachuc-timeline` (same pass count plus the new test) before the change and at completion.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-timeline --test links` -> pass, including the new test.
- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-timeline` -> pass; count is the old count plus 1 (anti-goal).
- Screen check on live (human or UI checker): send a mock or real reply with a ` ```rust packages/desktop-gpui/crates/timeline/src/lib.rs:10-20 ` fence -> the header shows "lib.rs Ln 10-20"; a click opens the file at line 10; a plain ` ```rust ` block looks as before.
- After the screen check, quit the GPUI app and run `./cargo.sh clean --profile dev` when no other cargo or rustc process runs.
