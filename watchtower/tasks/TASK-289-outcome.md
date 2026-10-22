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

## UI Checks

UI checker, 2026-10-22, check bundle at `74d73ff80` on live (6768), zoom 1.25. Screenshots in `/tmp/wt-ui/` (`289-*.png`).

- Screen check, `README.md` from Files renders a heading and has "Show source": PASS. `README.md` opened through the Command Center: heading "Bachuc" draws large, no `#` line; AX `file-view-bar` has `button Show source` (`289-readme-rendered.png`).
- Screen check, press "Show source": PASS. AX: `container Line 1: > Bachuc is a personal fork...` numbered lines, and the button reads `Show rendered` (`289-readme-source.png`).
- Screen check, `.md` at a line shows Source scrolled to that line: UNVERIFIED. The Command Center has no line syntax (`rg` of `crates/command/src`). A timeline file link needs a pointer click, and the AX tree has no link node (the assistant message is one `container (selectable)`). Code read: `FileView::new` sets `rendered = is_markdown_path(&path) && line.is_none()`, and `scroll_to_line` sets `rendered = false` before it scrolls.
- Anti-goal, `.rs` file has no toggle and the same bar text: PASS. After: `diff_prefs.rs` shows `9.5 KB, 266 lines`, no toggle button (`289-rs-antigoal.png`). Before: `git show d05d4aca0` leaves `format_file_size(doc.size)` and `doc.line_count()` unchanged; the toggle is only added when `doc.markdown` is set.
