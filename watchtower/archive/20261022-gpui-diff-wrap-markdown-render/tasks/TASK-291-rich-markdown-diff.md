# TASK-291 Rich markdown diff in one-file diff

Group: A (writes diff_view.rs; shared file with TASK-288, so it runs after it in the same group)
Class: risky

## Brief

Goal: Add a Rendered mode to the one-file diff of a markdown file. It shows the whole new file as rendered markdown, with added blocks tinted green and removed blocks tinted red.

Change: a `.md` one-file diff shows only numbered `+`/`-` lines -> its toolbar has a Rendered/Lines toggle, and Rendered shows a block diff of the whole file.

Boundaries:

- Follows [ADR-0061](watchtower/decisions/ADR-0061-gpui-rich-markdown-diff-from-working-file-over-protocol-full-text.md): read the working file with `Client::read_file`, rebuild the old side with `old_text`, then call `block_diff`. No protocol or server change.
- Owner choice on 2026-10-22: the toggle shows only for `Source::File` of a markdown path. `Source::All` and `Source::Commit` get no toggle; a commit's new side is not the working file.
- Hide the toggle for a deleted file (`is_deleted`): there is no working file to read.
- A new file has an empty old side: `old_text` gives `""`, so every block is Added.
- The mode is per view, starts as Lines, and lives for the view only.
- Lines mode keeps today's request path: one `file_diff` request per reload and no `read_file`. Read the file only while Rendered is on.
- When Rendered is on and the checkout changes, a reload reads the diff and the file again.
- Planner call: tint a block with the add and remove background colors the line rows use now, plus a 3 px left bar in the add or remove foreground color. Same blocks have no tint.
- Planner call: when `read_file` fails, the file is not text, or `old_text` gives None, keep Lines on and show the notice "Rendered diff unavailable: the file changed since the diff loaded" under the toolbar. Clicking the toggle again retries.
- Trap: `use gpui_kit::*` may not bring `TextView` in. Import `gpui_kit::component::text::TextView` as [packages/desktop-gpui/crates/explorer/src/pull_request_view.rs](packages/desktop-gpui/crates/explorer/src/pull_request_view.rs) does.
- Planner call: show the toggle only while the view is Ready with line rows; hide it for Loading, Failed, Empty, and a too-large or binary placeholder. Keep the old blocks on screen while a reload is in flight.
- Planner call: when the file reverts to clean while Rendered is on, show Empty as today.
- Planner call: keep the rendered block drawing in `diff_view.rs`. A new module would need a `lib.rs` line, and TASK-290 owns `lib.rs`.
- Trap: the line list in `DiffView` is a `ListState` with uniform or measured rows (`reset_rows`). Do not put the rendered blocks into that list; draw them in a separate vertical scroll area, so wrap and sideways offsets stay as they are.
- Trap: `reload` in `diff_view.rs` uses `self.requests.begin()` to drop stale answers. The file read must use the same request number, so a late answer for an old reload is dropped.

How:

- Add a markdown toggle button to `render_file_toolbar`, before the wrap button, shown only when the rule above holds. Labels: "Show rendered" in Lines and "Show lines" in Rendered. Use `IconName::BookOpen` and `IconName::Code`.
- Add a `rendered: Option<Vec<RichBlock>>` result and a `markdown_mode: bool` flag to `DiffView`. In `reload`, while `markdown_mode` is on, also run `read_file` and compute the blocks off the UI thread.
- In `render`, while `markdown_mode` is on and the blocks exist, draw each block with `TextView::markdown` in a scroll column no wider than `MAX_CONTENT_WIDTH`, tinted by its `BlockKind`.

Files:

- [packages/desktop-gpui/crates/explorer/src/diff_view.rs](packages/desktop-gpui/crates/explorer/src/diff_view.rs) (the toggle, the extra file read in `reload`, the rendered body)

Interfaces:

- Consumes: `fn render_file_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement + use<>` in `DiffView`, with element id `diff-file-toolbar` and the wrap button id `diff-file-wrap`, from TASK-288
- Consumes: `bachuc_explorer::markdown_diff::is_markdown_path(path: &str) -> bool` from TASK-290
- Consumes: `bachuc_explorer::markdown_diff::old_text(new_text: &str, diff_text: &str) -> Option<String>` from TASK-290
- Consumes: `bachuc_explorer::markdown_diff::block_diff(old_text: &str, new_text: &str) -> Vec<RichBlock>` from TASK-290
- Consumes: `pub struct RichBlock { pub kind: BlockKind, pub markdown: String }` and `pub enum BlockKind { Same, Added, Removed }`, both `Clone, Debug, PartialEq, Eq`, from TASK-290

Expected result:

- WHEN a one-file diff of a changed `.md` file opens THE `DiffView` SHALL show Lines and a "Show rendered" button.
- WHEN the owner clicks "Show rendered" THE `DiffView` SHALL show the whole file as markdown, added blocks green, removed blocks red, and same blocks plain.
- WHILE Rendered is on, WHEN the file changes on disk THE `DiffView` SHALL reload and show the new blocks.
- IF the patch does not match the file THEN THE `DiffView` SHALL stay on Lines and show the "Rendered diff unavailable" notice.
- WHEN the diff is of a new `.md` file THE Rendered mode SHALL mark every block Added.
- IF the file is deleted, not markdown, a commit diff, or in the all-files view THEN THE `DiffView` SHALL show no markdown toggle.

Anti-goal: in Lines mode a one-file diff sends exactly 1 daemon request per reload (`checkout.diff.get.request`, no `file_explorer_request`); tripwire; read from the `reload` code path in `diff_view.rs` and from the live daemon log request lines for one open and one checkout change, before the change and at completion.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-explorer` -> all tests pass.
- `cd packages/desktop-gpui && ./cargo.sh clippy -p bachuc-explorer -- -D warnings` -> no warnings.
- Screen check (UI checker, `cua_repl`, live GPUI build): change one paragraph of a tracked `.md` file, open it from Changes, press "Show rendered" with the AX action -> the AX tree shows the rendered text, and a screenshot shows a red block then a green block for the changed paragraph.
- Screen check: create a new untracked `.md` file, open its diff, press "Show rendered" -> every block has the green tint.
- Screen check: while Rendered is on, append a paragraph to the file on disk -> within 3 s a new green block shows at the end.
- Screen check: open a one-file diff of a too-large or binary `.md` placeholder, or while it loads -> no "Show rendered" button.
- Screen check: open a commit diff of a `.md` file, the all-files view, and a deleted `.md` file -> no "Show rendered" button in the AX tree.
- Mismatch check: owner check, or a unit-level proof that `old_text` returns None for a mismatch (TASK-290 test `old_text_rejects_a_patch_that_does_not_match`) plus a code read that None keeps Lines and sets the notice.
- Anti-goal: open a one-file diff of a `.rs` file and edit it once, then read the live daemon log -> each reload shows one `checkout.diff.get.request` and no `file_explorer_request` with `"mode": "file"` for that path.
