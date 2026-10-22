# TASK-291 outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/explorer/src/diff_view.rs` (toggle, extra file read in `reload`, rendered body, notice)

## Contract

- Consumes TASK-288 `render_file_toolbar` (adds button id `diff-file-markdown` before `diff-file-wrap`, labels "Show rendered" / "Show lines", icons `BookOpen` / `Code`) and TASK-290 `is_markdown_path`, `old_text`, `block_diff`, `RichBlock`, `BlockKind`.
- New state on `DiffView`: `markdown_mode: bool` (starts false), `rendered: Option<Vec<RichBlock>>`, `rendered_unavailable: bool`. `reload` reads the file only when `markdown_mode` is on, under the same `requests.begin()` number; blocks are computed on the background executor in `rich_blocks`.
- The toggle shows only when `markdown_available()`: `Source::File`, markdown path, `State::Ready`, first file not `Change::Deleted`, body is lines.
- Failure (read fails, not text, `old_text` None) -> `markdown_mode` false, notice "Rendered diff unavailable: the file changed since the diff loaded" under the toolbar; the toggle click retries and any later reload clears it.
- Rendered body: `diff-view-rendered` vertical scroll column, max `MAX_CONTENT_WIDTH`, each block a `TextView::markdown` with add/delete tint plus a 3 px left bar; Same blocks have no tint. The line `ListState` is untouched.

## Verified

- Red tests: the spec has none.
- `./cargo.sh test -p bachuc-explorer` -> 56 + 38 + 31 passed, 0 failed (group-level run for TASK-288 and TASK-291).
- `./cargo.sh clippy -p bachuc-explorer --no-deps -- -D warnings` -> only the known `large_enum_variant` at `file_view.rs:94`; with `-A clippy::large_enum_variant` it passes.
- `./cargo.sh fmt --check -p bachuc-explorer` -> exit 0.
- Mismatch check: `old_text` None is covered by TASK-290 test `old_text_rejects_a_patch_that_does_not_match`; code read: `rich_blocks` returns None -> `Rendered::Unavailable` -> `apply` sets Lines and the notice.
- Screen checks (rendered colors, new file, edit while Rendered, placeholder, commit/all/deleted no toggle) -> DEFERRED (UI checker).

## Anti-goal

Lines mode sends 1 request per reload. Code read of `reload`: `read_file` is called only inside `self.markdown_mode.then(...)`; `rg "read_file" diff_view.rs` -> one match, at that call. Before the change `reload` had no `read_file`. Live daemon log lines -> DEFERRED (UI checker). Code-read PASS.

## Lessons

- `gpui::transparent_black` is reachable through `use gpui_kit::*` paths; clippy wants it passed as a function (`redundant_closure`).
- Toolbar hides the markdown toggle for Loading, Failed, Empty and placeholders because `markdown_available` needs `State::Ready` with line rows.
