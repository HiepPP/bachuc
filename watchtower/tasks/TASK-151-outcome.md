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

## UI Checks

Run by the UI checker on 2026-10-16, working tree on HEAD 5afebcedd, bundle `BachucExplorer` rebuilt before the run, light theme. Screenshots: `/tmp/gpui-ui-check/G/`.

Setup: the live daemon on 6768 was not listening and AUTORUN forbids starting it, so the example ran with `--url ws://127.0.0.1:6770/ws` against the reference daemon (the `/tmp/gpui-bench` workspace exists there). The checker made `/tmp/gpui-bench` a git repo with one commit, then edited `README.md` and `src/main.rs` and added `src/extra.rs`. It also made `/tmp/gpui-plain` (a text file, a 2 KB random `blob.bin`, a 48 px `pic.png`; not a git repo). Both folders stay in `/tmp` as scratch.

### Live edit shows in Changes -> PASS (on 6770, not live)

Changes tab at start: `src/extra.rs +3 -0`, `README.md +2 -0`. After an edit to `src/main.rs` on disk, within 3 s the list showed `src/main.rs +1 -0` and the `src` group read `+4 -0`. Limit: this ran on the reference daemon, so it does not prove the live daemon.

### Non-git folder, PNG, binary -> PASS

- `/tmp/gpui-plain` Changes: "Not a git repository", centered.
- `pic.png`: the image renders, scaled up to fill the pane with smoothing (it extends to the bottom edge of the 832 px window; no placeholder).
- `blob.bin`: "Binary preview unavailable" and "2.0 KB", centered.

### Large folder (the repo root) -> PASS with a limit

Root `/Users/hiep/Projects/bachuc` (6458 tracked files, plus untracked and `node_modules`). The root listing opened with 26 rows and only the expanded folders listed. Expanding `packages`, `app`, then `src` (about 70 entries) showed the children at once: the click call returned in 666 ms and the first AX read after it already held the children (1075 ms from the start, including the automation overhead of both calls). A screenshot confirms the expanded folder. Limit: the automation overhead is inside the 1 s, so the real expand time is lower than the number, but it was not isolated. Smooth scrolling was not measured; one scroll step and later reads gave no stall.

### Parity (light) -> FAIL, 5 mismatches. Dark theme not run.

| Element                      | Reference (Electron)                                                    | GPUI                                                                        | Result                                          |
| ---------------------------- | ----------------------------------------------------------------------- | --------------------------------------------------------------------------- | ----------------------------------------------- |
| Tab rail                     | Files and Changes tabs, active chip fill, 36 px                         | same tabs, active chip fill                                                 | match                                           |
| Files row pitch, icons       | 29 px, chevron or material icon                                         | 29 px, same icons                                                           | match                                           |
| Changes row pitch, counts    | 32 px, green `+N` and red `-N`                                          | 32 px, same colors                                                          | match                                           |
| Files toolbar                | `Name` sort menu, new file, new folder, hidden toggle, refresh          | none                                                                        | MISMATCH (not built, listed in the Parity list) |
| Changes header               | branch `main`, `Uncommitted` menu with total `+6 -0`, refresh, more     | none                                                                        | MISMATCH                                        |
| Changes footer               | `Commits` section                                                       | none                                                                        | MISMATCH                                        |
| New file mark                | green "added" mark on `extra.rs`                                        | modified mark on every row                                                  | MISMATCH (known: no new flag in `ChangedFile`)  |
| Diff view                    | one scroll with a header per file, full-width added tint, syntax colors | one file, added tint only as wide as the text, no syntax colors in the diff | MISMATCH (tint width, syntax)                   |
| Added line tint color        | `#e0ece2`                                                               | `#e1ebe3`                                                                   | match                                           |
| Diff line pitch, hunk header | 22 px, `@@ -1,3 +1,5 @@`                                                | 22 px, same header text                                                     | match                                           |
| File view                    | not captured in the reference                                           | line numbers, tree-sitter colors, size line                                 | NOT COMPARED                                    |

Defect found (not in the spec list): the Files tab opens with "not connected" and a Retry button when it is the first tab (the repo-root run) or when switching to it from Changes (two runs). Retry loads the tree. Changes loaded correctly in the same sessions, so the daemon was up. The Files tab does not load after `Connected`.

### Accessibility tree -> PASS

`tab group Explorer views` with `tab Files` and `tab Changes` (settable, boolean value); `outline Files` and `outline Changes` with `row (selectable)` or `row (selected)`; change rows carry counts in the label (`src/extra.rs, +3, -0`); the diff is `Diff README.md` with `Heading README.md, +2, -0` and `hunk line`, `added line N`, `unchanged line N` containers; the file view is `File src/main.rs` with `116 B, 6 lines` and `Line N: text` containers; the PNG is `image pic.png`; the binary file shows `container Binary preview unavailable`.

### Idle CPU with Changes shown -> PASS

`measure.sh 60 ex="pids:66743"` with Changes and a diff shown, window kept in front by an `open` loop every 3 s: 0.6 CPU% (limit 1.0), 99 MB RSS, 44 MB footprint. A screenshot after the read shows the full window, no cut.

### Cleanup

The example and the reference app were quit; the lock was released; 6767 untouched. The checker created no agent or terminal. `/tmp/gpui-bench` (now a git repo with edits) and `/tmp/gpui-plain` remain in `/tmp`.
