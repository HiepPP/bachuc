# TASK-291 outcome

## Status

BLOCKED: BUILT, UNVERIFIED (Q-189). Code committed; see UI Checks.

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

## UI Checks

UI checker, 2026-10-22, check bundle at `74d73ff80` on live (6768), zoom 1.25. Scratch worktree `wt-ui-check` held a modified `CONTRIBUTING.md`, a new `new-scratch.md`, a 1.8 MB `big-scratch.md`, a binary `binary-scratch.md`, a deleted `README.ko.md`, and a committed `SECURITY.md` change. It is archived.

Main finding: the one-file diff (`Source::File`) is unreachable in the app. A Changes row press opens the all-files view; `ExplorerEvent::OpenDiff` has no emitter (see TASK-288 UI Checks). Every line below that needs a one-file diff is UNVERIFIED for that reason. The Rendered code was not observed.

- Screen check, changed `.md` one-file diff, "Show rendered", red then green block: UNVERIFIED (one-file diff unreachable).
- Screen check, new untracked `.md`, every block green: UNVERIFIED (one-file diff unreachable).
- Screen check, append a paragraph while Rendered is on: UNVERIFIED (one-file diff unreachable).
- Screen check, too-large or binary `.md` placeholder has no "Show rendered": UNVERIFIED (one-file diff unreachable). The all-files view lists `big-scratch.md` and `binary-scratch.md` as New rows with `+0 -0`.
- Screen check, commit diff, all-files view, deleted `.md` have no "Show rendered": PASS for the commit diff of `SECURITY.md` and the all-files view with the deleted `README.ko.md`. The AX tree has no such button in either (`288-commit-diff.png`). A one-file diff of a deleted file is unreachable.
- Anti-goal, Lines mode sends 1 request per reload: UNVERIFIED (one-file diff unreachable, no daemon log line to read). Code read: `rg read_file diff_view.rs` has one match, inside `markdown_mode.then(...)`.

Blocked: BUILT, UNVERIFIED (Q-189)

### Round 2

UI checker, 2026-10-22, check bundle at `04139fc00` on live (6768), via the new "Open file diff" menu item (TASK-297). Scratch worktree workspace `gray-kangaroo`, archived. Cause for every line below: the Changes row menu cannot be opened without the pointer. The AX row lists only "Pick" as a secondary action; `performSecondaryAction` with "ShowMenu", "Show Menu", and "ShowContextMenu" on the row failed with error -10005 (not a valid secondary action). The app registers `AccessibleAction::ShowContextMenu` at `explorer.rs:2106`, but `accesskit_macos` maps no show-menu action (MEMORY 2026-10-18). The menu key and shift-f10 open the tab menu only (`actions.rs:204-205`). A right-click needs the pointer, which is banned.

- Screen check, changed `.md` one-file diff, "Show rendered", red then green block: UNVERIFIED (row menu unreachable without the pointer).
- Screen check, new untracked `.md`, every block green: UNVERIFIED (row menu unreachable without the pointer).
- Screen check, append a paragraph while Rendered is on: UNVERIFIED (row menu unreachable without the pointer).
- Screen check, too-large or binary `.md` placeholder has no "Show rendered": UNVERIFIED (row menu unreachable without the pointer). The Explorer Changes tree lists `big-scratch.md` and `binary-scratch.md` as New rows with `+0 -0`.
- Screen check, one-file diff of a deleted `.md` has no "Show rendered": UNVERIFIED (row menu unreachable without the pointer). The commit diff and all-files parts stay PASS from Round 1.
- Anti-goal, Lines mode: one `checkout.diff.get.request` per reload and no `file_explorer_request` with `"mode": "file"` in `.dev/paseo-home/daemon.log` after one edit of an open `.rs` one-file diff: UNVERIFIED (row menu unreachable without the pointer; no one-file diff was opened, so the log has nothing to read). Code read from Round 1 stands.
