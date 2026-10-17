# TASK-179 Explorer scroll to file, long lines, markdown, diff mode

Group: I (writes `crates/explorer`)
Class: risky

## Brief

Goal: Close the explorer gaps TASK-162 left: a Changes click scrolls an open all-files diff to the file, long diff lines scroll sideways or wrap, markdown files get syntax colors, and the diff mode menu switches between Uncommitted and Committed.

Change: a second Changes click only focuses the open tab, long lines are cut at the pane edge, `.md` is plain, and the mode is a fixed "Uncommitted" label -> a focus request the open view follows, per-file sideways scroll with a wrap toggle, markdown colors, and a working mode menu.

Design: Electron:

- Scroll to file: `handleSelectTreeFile` in [packages/app/src/git/diff-pane.tsx](packages/app/src/git/diff-pane.tsx) (lines 1673-1693) and the focus effect in [packages/app/src/git/diff-document/surface.web.tsx](packages/app/src/git/diff-document/surface.web.tsx) (lines 475-487).
- Long lines: `HorizontalScroll` per file and the wrap switch in `surface.web.tsx` (lines 166-175, 533-535, 839-869); toolbar toggle `changes-toggle-wrap-lines` in `diff-pane.tsx` (lines 851-854), labels "Wrap long lines" and "Scroll long lines" (`packages/app/src/i18n/resources/en.ts` lines 943-944).
- Mode menu: `DiffModeMenu` in `diff-pane.tsx` (lines 281-340), labels "Uncommitted", "Committed", and "Diff mode" (`en.ts` lines 957-959); the default and expiry in [packages/app/src/git/working-diff-comparison/state.ts](packages/app/src/git/working-diff-comparison/state.ts) (lines 55-77); the request in [packages/app/src/git/use-diff-query.ts](packages/app/src/git/use-diff-query.ts) (lines 86-90).

Boundaries:

- Scroll to file (Planner call): the app's `open_diff_tab` only activates an open tab and drops the new `focus_path` (`crates/app/src/shell.rs:1309-1322`). Fix it inside this crate, with no app edit: `Explorer::open_all_diffs` also writes a GPUI global focus request `{cwd, path, revision}` with a growing revision, as Electron's `focusRequestId`. A `DiffView` with `Source::All` for that cwd observes the global and scrolls to the file's header on a revision it has not seen, expanding the file when collapsed. A request that arrives before the first diff answer applies after it. A new view takes the current revision at creation, so it does not scroll twice.
- Keep `ExplorerEvent`, `ExplorerDiffEvent`, `Explorer::new`, `set_cwd`, `set_visible`, `FileView::new`, `DiffView::new`, `new_all`, and `new_commit` as they are (`shell.rs:607-622` matches the events).
- Long lines: rows keep one height, `LINE_HEIGHT` 22, and stay lazy (`diff_view.rs:243`). The default is sideways scroll: each file keeps its own horizontal offset, moved by a horizontal trackpad or shift-wheel delta over that file's rows, and clamped to `0..=(longest line width - code area width)`. The gutter stays fixed; only the code moves. Width is the expanded char count times the mono advance width. Wrap mode wraps code to the pane width; then rows have their own heights and the list is reset without uniform height. The toggle sits in the all-files toolbar (`diff_view.rs:397-452`) with the Electron labels, icon `WrapText`.
- Planner call: the wrap choice is an in-memory global for the app session. Electron persists it in `@paseo:changes-preferences`; this crate has no settings file, and adding one belongs to `crates/settings`.
- Markdown: TASK-170 tries the gpui-kit `tree-sitter-markdown` feature. When TASK-170's outcome reports it enabled, map `md`, `markdown`, and `mdx` to `"markdown"` in `syntax::language_for` (`syntax.rs:19-31`); the file view and the diff both use it. When it was left out, keep `.md` plain, keep `languages_resolve_for_the_enabled_grammars_only` asserting `None`, and report it. Headings and links already have roles in `role_of` (`syntax.rs:166-209`). Inline spans (emphasis, code spans) need the separate `tree-sitter-markdown-inline` feature, which TASK-170 does not add; report that, do not edit the workspace `Cargo.toml`.
- Mode menu: replace the fixed "Uncommitted" labels in the all-files toolbar (`diff_view.rs:425-431`) and the Changes header (`explorer.rs:1202`) with a menu: "Uncommitted", a separator, "Committed" with the description `<branch> -> <base>` (`computeCommittedDiffDescription`, `diff-pane.tsx:1138-1146`; ref prefixes dropped as in `base_labels_drop_ref_prefixes`; none when either is missing or both are equal). The menu reloads with `checkout_diff_with`; `Base` sends `base_ref` from the loaded `CommitList.base_ref` when present, else none. `ignore_whitespace` follows the hide-whitespace toggle (Q-101), off by default.
- Mode state, as Electron: an in-memory override per cwd, stored with the dirty state at selection. With no override, or when the dirty state changed since, the mode is `Uncommitted` for a dirty tree and `Base` for a clean one. Dirty means `CheckoutDetails.files` is not empty. The Changes header and every all-files view of that cwd share the mode. A response for an older mode or request is dropped (`Latest`, `diff_doc.rs:193-212`).
- Left out, with reasons:
  - Split layout: it needs a paired-row model and two-column rendering on top of the lazy list, a separate feature outside the second scope.
  - Hide whitespace: Planner call (Q-101): add the Electron toggle to the diff toolbar; it sets `DiffCompare::ignore_whitespace` for the next `checkout_diff` call, and a test checks that the request carries it.
  - New file and New folder: the client has no `fs.entry.create.request` call, and TASK-169 adds none.
  - Modified sort: `FileEntry` has no modified time. The daemon sends `modifiedAt`, but a new field on the public `FileEntry` breaks struct literals in other crates (TASK-157 Boundaries), and no client accessor is planned.
- New controls use `bachuc_theme` tokens and `bachuc_icons` icons, with roles and labels. Keep each rule in a pure function and test it (MEMORY 2026-10-16).

Files:

- [packages/desktop-gpui/crates/explorer](packages/desktop-gpui/crates/explorer) (`src`, `tests`, `examples/explorer.rs` if it no longer builds)

Interfaces:

- Consumes: `bachuc_client::DiffCompare` (`mode: DiffMode`, `base_ref: Option<String>`, `ignore_whitespace: bool`), `bachuc_client::DiffMode` (enum `Uncommitted`, `Base`), and `Client::checkout_diff_with(&mut self, cwd: &str, compare: DiffCompare, cx) -> Task<anyhow::Result<Vec<FileDiff>>>` from TASK-169
- Consumes: the gpui-kit `tree-sitter-markdown` workspace feature, when TASK-170's outcome reports it enabled, from TASK-170
- Consumes: the same `CheckoutDetails`, `checkout_details`, `CommitList`, and `commits` API from TASK-157

Expected result:

- With the Changes tab open on file A, a click on file B in Changes scrolls the open tab to B's header.
- A 400-character line scrolls sideways inside its file; the gutter stays; wrap mode wraps it.
- The mode menu switches to Committed and the diff shows the branch changes against the base.
- Cases:
  - Focus request for a path not in the diff -> no scroll, no panic.
  - Focus request for a collapsed file -> the file expands, then scrolls.
  - Focus request before the first answer -> applies after the answer.
  - Offset clamp: longest line 1,000 px, code area 600 px -> offsets clamp to 0 and 400; a file with no long line never moves.
  - Clean tree with no override -> `Base`; dirty tree -> `Uncommitted`; override `Base` set while dirty, then the tree goes clean -> the default applies again.
  - `README.md` -> `"markdown"` when the feature is on; `None` when it is off.

Anti-goal: the 20,000-line, 50-file diff still builds its row model in under 200 ms in a debug test (`a_20000_line_diff_of_50_files_builds_its_rows_in_under_200_ms`), and sideways scroll changes no row model, only offsets; tripwire; read before the first change, after sideways scroll lands, and at completion.

## Verify

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-explorer` -> pass, including every existing model and diff test unchanged and new pure tests for each expected case: the focus request revision rule (new, seen, before the answer, collapsed, missing path), the per-file offset clamp, the wrap toggle row heights, the mode default and override expiry, the `Base` request with and without a base ref, the stale mode answer dropped, the hide-whitespace toggle setting `ignore_whitespace` on the next request, Committed picked before the `CommitList` loads (no base ref), and `language_for` for `.md` matching the workspace feature, plus the 20,000-line timing test
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-explorer` -> exit 0
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> exit 0 with no edit outside this group's files
- UI check: with live running, open the GPUI build with `open -g`, open Changes on a repo with a long line and a markdown file, take background screenshots with no pointer and no foreground, and compare with the Electron 6770 build -> the mode menu labels, the long line cut at the pane edge with the gutter in place, and markdown heading colors match; a stale or blank frame is UNVERIFIED
