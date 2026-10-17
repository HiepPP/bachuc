# TASK-179 Outcome

## Status

BLOCKED

Blocked: BUILT, UNVERIFIED (Q-109). The code and its tests pass; trackpad sideways scroll, wrap, and scroll-to-file need the owner's hand check on screen.

## Changed

- `packages/desktop-gpui/crates/explorer/src/diff_prefs.rs` (new): the pure model of the shared diff choices. It has `DiffPrefs` (a GPUI `Global`), `CheckoutPrefs`, `ModeOverride`, `FocusRequest`, `FocusTracker`, `resolve_mode`, `compare_for`, `ref_label`, `committed_description`, `clamp_offset`, and `uniform_row_height`.
- `src/lib.rs`: `pub mod diff_prefs`.
- `src/ui.rs`: `read_prefs` and `update_prefs`. `update_prefs` sets the global only when the change returns true. Also a `MenuTrigger` dropdown trigger and `diff_mode_menu`: "Uncommitted", a separator, then "Committed" with the `<branch> -> <base>` description. The trigger has `Role::Button` and the label "Diff mode"; the menu is 260 px wide.
- `src/diff_view.rs`:
  - A `FocusTracker` replaces the `scrolled` flag. The view observes `DiffPrefs`.
  - An all-files view requests with `checkout_diff_with(prefs.compare(cwd))` and reloads when the compare changes.
  - Each file keeps its own sideways offset, moved by a horizontal or shift-wheel delta over its lines and clamped at render. Only the code moves; the gutter stays.
  - In wrap mode, rows wrap and the list resets without a uniform height.
  - The toolbar has the mode menu and two toggles: `changes-toggle-whitespace` (Pilcrow, "Hide whitespace"/"Show whitespace") and `changes-toggle-wrap-lines` (WrapText, "Wrap long lines"/"Scroll long lines").
- `src/explorer.rs`:
  - `open_all_diffs` writes a focus request before it emits.
  - A status answer records the dirty state and branch. A commit list answer records `base_ref`.
  - In `Base` mode, the Changes tree lists the files of a `checkout_diff_with` Base answer. A separate `Latest` drops stale answers.
  - The mode menu replaces the fixed "Uncommitted" label. The explorer observes `DiffPrefs`.
- `src/diff_doc.rs`: `reveal(path)` opens a collapsed file and returns its header row. `longest_columns(file)` gives the drawn column count of the widest line per file.
- `src/code.rs`: `display_columns` (tabs expanded, the long-line cut and ellipsis counted).
- `src/changes.rs`: `changed_file_of(&FileDiff)` and `CheckoutView::from_diffs`.
- `src/syntax.rs`: `md`, `markdown`, and `mdx` map to `"markdown"`.
- `tests/diff_and_commits.rs`: 18 new tests. `tests/models.rs`: markdown asserts in `languages_resolve_for_the_enabled_grammars_only`.

## Contract

- Unchanged: `ExplorerEvent`, `ExplorerDiffEvent`, `Explorer::new`, `set_cwd`, `set_visible`, `FileView::new`, `DiffView::new`, `new_all`, `new_commit`, and `LINE_HEIGHT` 22. `build -p bachuc-app` passes with no app edit.
- Consumes `DiffCompare`, `DiffMode`, and `Client::checkout_diff_with` from TASK-169 by those names. Consumes `CheckoutDetails`, `checkout_details`, `CommitList`, and `commits` from TASK-157.
- Markdown: TASK-170 enabled `tree-sitter-markdown`, so `README.md` gives `"markdown"`. TASK-170 also enabled `tree-sitter-markdown-inline`; the spec expected it to be absent. Gpui-component 0.7.3 builds the inline grammar under `tree-sitter-markdown` too. Its captures `emphasis`, `emphasis.strong`, `link_text`, `link_uri`, and `text.code.span` have no role in `role_of`, so they keep the text color. Only headings (`title`), link destinations (`text.uri`), and link labels (`text.reference`) take colors. `role_of` is unchanged.
- Focus: a view takes the focus revision current at its creation, plus its own `focus_path`. Revisions for other folders, or revisions the view has seen, do nothing. A request that comes before a `Ready` diff waits for it. A missing path is dropped with no scroll.
- Mode: each folder can hold an override, stored with its dirty state at selection. A status answer with another dirty state drops it. A folder with no status yet counts as dirty, so it shows `Uncommitted` (safe default, same as before). `Base` sends `base_ref` only from a loaded commit list.
- Wrap and hide-whitespace: in-memory session values in `DiffPrefs`, both off by default. The wrap choice applies to every diff view. The toggles show only in the all-files toolbar.
- Not done:
  - Hide whitespace does not change the Uncommitted Changes tree: `checkout_details` takes no whitespace option.
  - The empty-state actions "See uncommitted/committed changes" were not ported.
  - The spec's other out-of-scope items (split layout, New file and New folder, Modified sort) stay out.
- GitNexus impact: `language_for` gave CRITICAL (14 impacted, all in the explorer crate). The change only adds extensions. `rg` found no caller outside `crates/explorer`. The other edited symbols gave LOW, or UNKNOWN with explorer-only callers by `rg`.

## Verified

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-explorer` -> exit 0; `diff_and_commits` 37 passed, `models` 21 passed, 0 failed. Every earlier test passes unchanged. The new tests cover:
  - Focus: a new revision, a seen revision, another folder, a view created by the click, a request before the answer, a collapsed file, and a missing path.
  - Offset clamp: 1,000 px in a 600 px area clamps to 0 and 400, and a short file never moves. Per-file columns with tabs and a 400-character line. Sideways scroll leaves the rows unchanged.
  - Wrap row heights: `Some(22)` and `None`.
  - Mode default and expiry, and a stale override masked.
  - `Base` with and without a base ref, and Committed picked before the `CommitList` loads.
  - A stale mode answer dropped, and hide-whitespace setting `ignore_whitespace` on the next request.
  - The committed description, the Committed tree from base diffs, the `.md` language and a markdown heading color, and the 20,000-line timing test.
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-explorer` -> exit 0.
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-explorer --all-targets` -> exit 0 (the example builds unchanged).
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> exit 0. Its one warning, `next_theme` never used in `crates/app/src/state.rs`, is outside these files.
- UI check (mode menu labels, long-line cut with the gutter in place, markdown heading colors against Electron 6770) -> DEFERRED (UI checker).
- Menu open and pick, trackpad sideways scroll, wrap layout, and the scroll to a file in an open tab -> UNVERIFIED (unattended): code tests only. These paths need `App` and were checked by reading only.

## Anti-goal

- Checkpoint 0, before any change: 20,000-line rows built in 3.57 ms (limit 200 ms); source `/tmp/wt-i179-cp0.txt`; 2026-10-17 02:21.
- Checkpoint 1, after sideways scroll and per-file columns landed: 3.73 ms; source `/tmp/wt-i179-cp1.txt`; 2026-10-17 02:32.
- Checkpoint 2, at completion, with the whole suite running in parallel: 8.69 ms; source `/tmp/wt-i179-final.txt`; 2026-10-17 02:32.
- Sideways scroll changes only `DiffView::offsets`. The row model gained one per-file column count, computed in `DiffDoc::new`. The test `sideways_scroll_leaves_the_row_model_unchanged` checks the rows. The tripwire did not trip.

## UI Checks

Checked 2026-10-17 by the UI checker with `open -g`, AX reads, AX presses, and window screenshots (no pointer, no foreground).

- UI check (mode menu labels, long-line cut with the gutter in place, markdown heading colors against Electron 6770) -> UNVERIFIED (unattended): needs pointer. The Changes tab of a workspace with a long line and a markdown file cannot open: AX press on the GPUI sidebar workspace rows changed nothing (`/tmp/wt-ui/gpui-t144.jpg`, `/tmp/wt-ui/gpui-tsc.jpg`).

## Lessons

- `App::set_global` and `default_global` always notify observers. A shared global that status answers write should be set only on a real change, or every observing view reloads on each answer.
- The gpui-component popup menu gives an `ElementItem` no accessible label. A custom row, such as one with a description, must label itself.
- The gpui-component `tree-sitter-markdown` feature also builds the inline grammar. The separate gpui-kit `tree-sitter-markdown-inline` feature adds nothing for highlighting.

## UI Checks, round 2

Checked 2026-10-17 by the UI checker with `open -g`, AX reads, AX presses, and window screenshots (no pointer, no foreground). Fixture: the scratch repo `/tmp/gpui-bench` (workspace `gpui-bench / main`, opened from the command center). The checker edited its `README.md` to add two markdown headings and a 330-column line; `src/main.rs`, `src/extra.rs`, and `shot.png` were already changed.

- UI check (Changes diff with a long line and a markdown file, mode menu labels, against Electron 6770) -> FAIL. The Changes tab lists `src/extra.rs`, `src/main.rs`, `README.md`, and `shot.png` (`/tmp/wt-ui2/gpui-changes-list.jpg`). AX press on the `README.md` row, and again on the `src/main.rs` row after a relaunch, aborts the GPUI app within a second (2 of 2 runs). Crash reports: `/tmp/wt-ui2/bachuc-desktop-2026-10-17-183001.ips` and `/tmp/wt-ui2/bachuc-desktop-2026-10-17-183056.ips`. Both show `panic_already_mutably_borrowed` in `gpui::elements::list::ListState::viewport_bounds`, called from `DiffView::render_row` (`bachuc_explorer::diff_view`) while the list lays out its items. From the source: `render_line` calls `code_widths` (`crates/explorer/src/diff_view.rs:759`), which calls `self.list.viewport_bounds()` (`:330`) inside the list's own layout borrow. The mode menu and the long-line cut in Changes could not be reached. This is a new defect the check found; round 1 never opened a diff.
- UI check (long line cut with the gutter in place, markdown heading colors in the file viewer) -> UNVERIFIED: the Files tab view of `README.md` draws the line numbers in a gutter, cuts the 330-column line at the pane edge, and colors the heading text blue (`/tmp/wt-ui2/gpui-readme-file.jpg`), but no Electron frame of the same file was taken, so the match is not compared.
