# TASK-288 One-file diff toolbar with wrap

Group: A (writes diff_view.rs; shared file with TASK-291)
Class: code

## Brief

Goal: Give the one-file diff and the commit diff a toolbar with the wrap toggle, so long lines can wrap there as in the all-files view.

Change: only `Source::All` shows `render_toolbar` -> `Source::File` and `Source::Commit` show a small toolbar with the wrap toggle.

Boundaries:

- The wrap toggle reads and writes the shared `DiffPrefs::wrap_lines` in [packages/desktop-gpui/crates/explorer/src/diff_prefs.rs](packages/desktop-gpui/crates/explorer/src/diff_prefs.rs) through `toggle_wrap` in `diff_view.rs`. One click changes wrap in every diff view, as it does now.
- The one-file toolbar has no mode menu, collapse, whitespace, or refresh control. `file_diff` always sends the uncommitted compare without `ignoreWhitespace`, so a whitespace toggle would do nothing there.
- Keep the all-files toolbar controls and their order unchanged.
- Use the same toolbar height (`TOOLBAR_HEIGHT`), colors, and `toolbar_button` helper as the all-files toolbar. Put the wrap button at the right end.
- Planner call: the one-file toolbar shows no file name; the tab already shows it.
- Trap: `render` in `diff_view.rs` adds the toolbar only `when(matches!(self.source, Source::All { .. }))`. Do not change `render_toolbar`; add a second, smaller toolbar function so TASK-291 can add its toggle there.

How:

- Add `render_file_toolbar` in `diff_view.rs` that holds a spacer and the wrap button with the same labels as the all-files wrap button ("Wrap long lines" or "Scroll long lines").
- In `render`, show `render_toolbar` for `Source::All` and `render_file_toolbar` for `Source::File` and `Source::Commit`.

Files:

- [packages/desktop-gpui/crates/explorer/src/diff_view.rs](packages/desktop-gpui/crates/explorer/src/diff_view.rs) (add `render_file_toolbar`; choose the toolbar per source in `render`)

Interfaces:

- Produces: `fn render_file_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement + use<>` in `DiffView`, with element id `diff-file-toolbar` and the wrap button id `diff-file-wrap`, for TASK-291

Expected result:

- WHEN a one-file diff or a commit diff opens THE `DiffView` SHALL show a toolbar with one wrap button.
- WHEN the owner clicks the wrap button THE `DiffView` SHALL wrap long lines, and its label SHALL change to "Scroll long lines".
- WHEN wrap changes in one diff view THE other open diff views SHALL follow it.
- WHILE the pane is 300 px wide, THE one-file toolbar SHALL keep the wrap button visible.

Anti-goal: the all-files toolbar keeps its 5 controls (mode menu, collapse all, whitespace, wrap, refresh) in the same order; tripwire; read from the AX tree of the `diff-view-toolbar` element before the change and at completion.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-explorer` -> all tests pass.
- `cd packages/desktop-gpui && ./cargo.sh clippy -p bachuc-explorer -- -D warnings` -> no warnings.
- Screen check (UI checker, `cua_repl`, live GPUI build): open a changed file from Changes -> the AX tree has `diff-file-toolbar` with a button labeled "Wrap long lines"; press it with the AX action -> the label becomes "Scroll long lines" and a long line wraps.
- Screen check: open a commit file diff from the commits list -> the same toolbar and wrap button show.
- Screen check: resize the pane to 300 px wide -> the wrap button stays inside the pane bounds.
- Anti-goal: AX tree of `diff-view-toolbar` in the all-files view -> the same 5 controls in the same order as before the change.
