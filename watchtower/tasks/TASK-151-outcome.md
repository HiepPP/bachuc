# TASK-151 Outcome

## Status

DONE for code checks. Every screen check is DEFERRED (UI checker). The Anti-goal CPU read is not measured yet.

## Changed

- New crate `packages/desktop-gpui/crates/explorer` (`bachuc-explorer`): `src` (`explorer.rs`, `file_view.rs`, `diff_view.rs`, `tree.rs`, `changes.rs`, `diff.rs`, `syntax.rs`, `code.rs`, `ui.rs`, `file_icons.rs`, generated `file_icon_data.rs`), `tests/models.rs`, `examples/explorer.rs`, `codegen/generate-file-icons.mjs`.
- A restart: the first builder died before it verified anything. This builder read all its files, kept them, and fixed two things: a diff for a path that holds the word "binary" showed the binary placeholder (now `diff::no_diff_reason` reads the end of the client message), and a `too_large` file diff now shows a header plus the `Diff too large to display` status line of the Electron diff document instead of a full-pane state.

## Contract

- `Explorer::new(client: Entity<Client>, cwd: String, window: &mut Window, cx: &mut Context<Explorer>) -> Explorer` and `Explorer::set_cwd(&mut self, cwd: String, cx: &mut Context<Explorer>)`.
- `ExplorerEvent::{OpenFile { cwd, path }, OpenDiff { cwd, path }}`, emitted by `Explorer`.
- `FileView::new(client, cwd, path, line: Option<u32>, window, cx) -> FileView` and `DiffView::new(client, cwd, path, window, cx) -> DiffView`.
- Extra public API for TASK-155: `Explorer::select_tab(ExplorerTab, cx)`, `Explorer::tab()`, `Explorer::set_visible(bool, cx)` (call it when the sidebar shows or hides: a hidden Explorer runs `unwatch_checkout`), `Explorer::refresh(cx)`, `FileView::reload`, `FileView::scroll_to_line`, `DiffView::reload`, `format_file_size`.
- A `FileView` path may be absolute under `cwd` or relative to it. Both views keep their own scroll handle, so an inactive tab keeps its position.
- Views read `bachuc_theme::tokens`, `metrics`, and `bachuc_icons`. The example registers `bachuc_icons::Assets`. The app must register it too.

## Verified

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-explorer` -> PASS: 21 tests (tree order, lazy expand, 10,000-entry tree lists only open folders, changes grouping and compression, diff parse, no-diff reason, highlighter lines, tabs, icon lookup).
- `cargo.sh clippy -p bachuc-explorer --all-targets` -> PASS, no warning in the crate. `cargo.sh fmt --check -p bachuc-explorer` -> PASS.
- Strings checked against `packages/app/src/i18n/resources/en.ts`: "Loading files...", "No files", "Not a git repository", "Checking repository...", "No changes to display", "This file is too large to display", "Binary preview unavailable", "Binary file", "Diff too large to display".
- Screen checks (Files, Changes, file view, diff view versus the reference on 6770; live Changes edit in `/tmp/gpui-bench`; non-git folder, PNG, binary file; `getAXState`; repo root with more than 10,000 files and expand `packages/app/src`) -> DEFERRED (UI checker).

## Anti-goal

Idle CPU at most 1.0% with Changes shown and a watched checkout. Checkpoints: none measured -> DEFERRED (UI checker). Nothing in the crate animates or polls: the only refresh trigger is `DirectoryEvent::CheckoutChanged`, and `unwatch_checkout` runs on hide, on a tab switch to Files, on `set_cwd`, and on release.

## Parity list

Rows below are from source, not from a screenshot; the UI checker confirms each.

| Element                                                                                | Built as                                                                             | State      |
| -------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------ | ---------- |
| Tab rail                                                                               | `TAB_BAR_HEIGHT` bar, 26 px tabs, `Files` and `FileDiff` icons, 1 px bottom border   | unverified |
| Files row                                                                              | 29 px, 12 px indent per level, chevron or colored material icon, `foreground` at 76% | unverified |
| Changes row                                                                            | 32 px, compressed folders, `+N -M` compact counts, modified mark                     | unverified |
| Changes toolbar and Files toolbar (sort, new file, new folder, hidden toggle, refresh) | not built; the spec lists none. `refresh()` exists                                   | mismatch   |
| Changes mark                                                                           | always the modified mark: `ChangedFile` has no new or deleted flag                   | mismatch   |
| File view                                                                              | line numbers, 1.45 line height, gutter, tree-sitter colors, size and line bar        | unverified |
| Diff view                                                                              | 22 px lines, addition and deletion tints, header with counts                         | unverified |

## Lessons

- A git repo with a detached HEAD gives `CheckoutStatus.branch == None` (client `core.rs`), so the Changes tab shows "Not a git repository" for it. The spec ties the non-git state to `branch: None`; the client would need an `is_git` flag to separate them.
- Highlighting resolves for rust, typescript, tsx, javascript, python, bash, and json only; other files draw in the plain color.
- `Client::watch_checkout` has no reference count: a second view that calls `unwatch_checkout` for the same `cwd` stops the first one's events. Only `Explorer` watches.
- `DiffView` reloads on `CheckoutChanged` but does not watch itself. It updates only while an `Explorer` watches the checkout (Changes tab shown).
- `FileView` finds a too-large file from the `list_directory` listing of its folder before it calls `read_file`, so the daemon never sends a huge file (limits: 10 MB text, 50 MB image).
- During the restart a builder edited one import with `sed -i` by mistake (a one-line change in `diff_view.rs`); the content was checked afterwards.
