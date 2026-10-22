# TASK-289 Outcome

## Status

DONE (code checks). Screen checks are DEFERRED (UI checker).

## Changed

- `packages/desktop-gpui/crates/explorer/src/file_view.rs`: `rendered` field, toolbar toggle, `render_markdown` body, `scroll_to_line` switches to Source.

## Contract

- `FileView::new` sets `rendered = is_markdown_path(&path) && line.is_none()`.
- `scroll_to_line` sets `rendered = false`, then scrolls through `scroll_list_to`. `apply` (load and reload) calls `scroll_list_to`, so a reload keeps the current mode.
- `TextDoc.markdown: Option<SharedString>` holds the text for a markdown path, so a frame does not clone the file.
- `render_toolbar` takes `&mut Context<Self>`. The `file-view-toggle` button (Role::Button, label "Show source" / "Show rendered") shows only when `doc.markdown` is set. The `file-view-bar` aria label is unchanged.
- Rendered body: `file-view-text` (Role::Document) with `overflow_y_scroll`, a column `max_w(MAX_CONTENT_WIDTH)` centered, `TextView::markdown` with the default link handler.

## Verified

- `./cargo.sh test -p bachuc-explorer` -> all tests pass (56 + 38 + 31).
- `./cargo.sh fmt --check -p bachuc-explorer` -> exit 0.
- `./cargo.sh clippy -p bachuc-explorer --no-deps -- -D warnings` -> one error, the Known Failure `large_enum_variant` at `enum State` (`State` untouched). `--no-deps` used.
- GitNexus impact `scroll_to_line`: LOW (caller `apply`; `shell.rs:2799` outside the index).
- Group-level run: this is the only TASK in the group.
- DEFERRED (UI checker): README.md renders with "Show source"; toggle via AX action; `.md` at a line shows Source scrolled to the line.

## Anti-goal

Tripwire not crossed in code: the `.rs` path has no `markdown` (so no toggle child), and the `file-view-bar` aria label and `render_lines` are unchanged. The AX reads of `file-view-bar` and `file-view-text` before and after are DEFERRED (UI checker); no checkpoint was measured by this builder.

## Lessons

- `UniformListScrollHandle::scroll_to_item_strict` keeps its target until the list draws, so it can run while the list is unmounted.
