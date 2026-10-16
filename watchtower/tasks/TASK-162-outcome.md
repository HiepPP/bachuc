# TASK-162 Outcome

## Status

DONE

## Changed

- `crates/explorer/src/diff_doc.rs` (new): `DiffDoc`, the multi-file row model (a header per file in daemon order, then lines or a too-large or binary placeholder, per-file and all-files collapse, `header_row` for the focus file), and `Latest`, the newest-request gate.
- `crates/explorer/src/commits.rs` (new): `CommitsState` (loading, failed, empty with the base label, ready), `commit_rows`, `base_label`, `parse_iso8601`, and `time_ago` (`formatTimeAgo`, date in UTC).
- `crates/explorer/src/diff_view.rs`: one view for three sources (`new`, `new_all`, `new_commit`), drawn with the GPUI `list` element, so only rows in view are built. Header rows are 30 px and line rows 22 px. The all-files view has a toolbar with the diff mode, the totals, collapse or expand all, and refresh. A reload on `CheckoutChanged` keeps the scroll, and an older answer is dropped. The first answer scrolls to the focus file.
- `crates/explorer/src/explorer.rs`: Changes uses `checkout_details`. `is_git` gives the non-git state; a detached HEAD shows the branch "Unknown" and stays a git state. Rows show the New, Deleted, or Modified mark, and labels name renames. A row click emits `ExplorerDiffEvent::OpenAllDiffs` with the file as focus. The header has an Open Diff tab button. The Commits footer opens and closes, loads `commits` when open, reloads on `CheckoutChanged`, opens a commit to its files, and a file click emits `OpenCommitDiff`. The Files tab has the Files toolbar: sort trigger (Name, Size), hidden files toggle, and refresh.
- `crates/explorer/src/syntax.rs`: `ThemeSyntax`, a resolver over `bachuc_theme::syntax_colors`; `highlight` takes any resolver. The old `SyntaxColors` stays as is, because `bachuc-timeline` builds it field by field.
- `crates/explorer/src/file_view.rs`: the file viewer also uses the selected syntax theme.
- `crates/explorer/src/changes.rs`, `tree.rs`, `ui.rs`, `lib.rs`: `Change` moved to `changes` with `of` and `label`; `CheckoutView`; `SortBy`, `rows_with`, `only_hidden`; module exports.
- `crates/explorer/tests/diff_and_commits.rs` (new): 19 tests. `crates/explorer/examples/explorer.rs`: opens `ExplorerDiffEvent` requests.

## Contract

- `bachuc_explorer::ExplorerDiffEvent` (`OpenAllDiffs { cwd, focus_path: Option<String> }`, `OpenCommitDiff { cwd, sha, path }`), emitted through a second `EventEmitter` on `Explorer`. `ExplorerDiffEvent::commit_file(cwd, &CommitSummary, file_index) -> Option<Self>`.
- `DiffView::new_all(client, cwd, focus_path, window, cx)` and `DiffView::new_commit(client, cwd, sha, path, window, cx)`. `DiffView::new`, `Explorer::new`, `set_cwd`, `set_visible`, `FileView::new`, and `ExplorerEvent` keep their signatures.
- Behavior change: a Changes row click no longer emits `ExplorerEvent::OpenDiff`. It emits only `ExplorerDiffEvent::OpenAllDiffs`, so TASK-167 must subscribe to it. Until then, a Changes row click in the app opens no tab.
- Pure models for later use: `diff_doc::{DiffDoc, DiffRow, DiffFile, FileBody, Latest}`, `commits::*`, `changes::{Change, CheckoutView, UNKNOWN_BRANCH}`, `tree::SortBy`, `syntax::ThemeSyntax`.

## Verified

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-explorer` -> exit 0; `diff_and_commits` 19 passed, `models` 21 passed. The new tests cover too-large and binary files, an empty list, a rename, a missing focus path, a stale answer, the marks, `is_git: false`, a detached HEAD, a failed and an empty commit list, the commit-diff event, and the 20,000-line timing.
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-explorer` -> exit 0.
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> exit 0, Finished.
- UNVERIFIED (unattended): code tests only. No window was opened, so the layout, the scroll to the focus file, and hover states are not seen.

## Anti-goal

- Checkpoint: the 20,000-line diff of 50 files builds 20,050 rows in 3.2 ms (limit 200 ms), debug test `a_20000_line_diff_of_50_files_builds_its_rows_in_under_200_ms`, 2026-10-16 16:19.
- Checkpoint: render builds rows only for the visible range. The diff uses the GPUI `list` element with its default visible-only measuring and a 200 px overdraw; no code builds one element per diff line. Source: `crates/explorer/src/diff_view.rs`, 2026-10-16 16:19. Not measured in a window (code tests only).

## Lessons

- `bachuc-timeline` builds `bachuc_explorer::syntax::SyntaxColors` with a struct literal, so that type cannot change shape until TASK-164 moves the timeline to `bachuc_theme`.
- The GPUI `list` element has no horizontal scroll, and `uniform_list` needs one row height. Variable header and line heights won the trade: long diff lines are now cut at the pane edge, where the old one-file view scrolled sideways.

## Questions

- Safe default: Changes rows emit only `OpenAllDiffs`, not also `ExplorerEvent::OpenDiff`, so TASK-167 does not open two tabs per click.
- Left out, reported: markdown highlighting. It needs the gpui-kit `tree-sitter-markdown` feature in the workspace `Cargo.toml`, outside this TASK's files, and a per-crate feature makes a second GPUI build.
- Left out, reported: the diff mode menu (Committed needs a base-mode `checkout_diff` the client lacks), the split layout toggle, hide whitespace (needs a daemon option the client lacks), wrap lines, New file and New folder (no create request in the client), and the Modified sort (`FileEntry` has no modified time).
- Follow-up: horizontal scroll or wrap for long diff lines.
