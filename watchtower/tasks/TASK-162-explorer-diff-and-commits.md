# TASK-162 Explorer multi-file diff and commits

Group: F (writes `crates/explorer`)
Class: risky

## Brief

Goal: Make the explorer's Changes and diff match Electron: one scroll with every changed file, the Files toolbar, the Commits footer with commit diffs, new and deleted marks, the non-git state for a real non-git folder, and the selected syntax theme (Q-075, Q-081, Q-082, audit items 4 and 5).

Change: one file per diff tab, no toolbar or footer, every file marked modified, a detached HEAD shown as non-git, and fixed diff colors -> the Electron layout and states.

Design: [packages/app/src/git/diff-pane.tsx](packages/app/src/git/diff-pane.tsx) (toolbar types from line 344, `DiffLayoutToggle` line 237, `DiffModeMenu` line 288, `CommitsSection` line 1375), [packages/app/src/git/commits-section/commits-section.tsx](packages/app/src/git/commits-section/commits-section.tsx) (line 88) with `commit-row.tsx`, and [packages/app/src/git/diff-document/surface.web.tsx](packages/app/src/git/diff-document/surface.web.tsx).

Boundaries:

- Load the multi-file diff with one `checkout_diff` call and reload it on `DirectoryEvent::CheckoutChanged` (owner). Drop a response that arrives after a newer request. Each `too_large` or `binary` file shows its placeholder; the others still render.
- Commits footer: list the branch commits; a click on a commit file opens that file's commit diff (owner: list plus commit diffs).
- Use `checkout_details` for the Changes list: `is_git` for the non-git state, `is_new`/`is_deleted` for the marks, `old_path` for renames.
- Diff syntax colors come from `bachuc_theme::syntax_colors`. The gpui-kit `tree-sitter-markdown` feature exists; enable it for markdown files if it builds. `MEMORY.md` and the archived plan context name `css`, `html`, `toml`, and `yaml` as breaking `cocoa`; if markdown breaks the build too, leave it out and say so.
- Keep `ExplorerEvent` unchanged, because `crates/app/src/shell.rs:563-570` matches it exhaustively. Emit the new open requests as a second event type, `ExplorerDiffEvent`, through a second `EventEmitter` impl, as `Client` does with `DirectoryEvent` (Planner call from the Builder Check). Keep `Explorer::new`, `set_cwd`, `set_visible`, `FileView::new`, and `DiffView::new` working as today.
- A layout toggle or mode menu that needs a client mode the daemon lacks stays out; report it.
- Rows render lazily (`uniform_list` or `list`), never one element per diff line up front.

Files:

- [packages/desktop-gpui/crates/explorer](packages/desktop-gpui/crates/explorer) (`Cargo.toml`, `src`, `tests`, `examples/explorer.rs`)

Interfaces:

- Consumes: `bachuc_client::CheckoutDetails` (`is_git: bool`, `branch: Option<String>`, `files: Vec<ChangedFileDetails>`), `bachuc_client::ChangedFileDetails` (`path: String`, `old_path: Option<String>`, `is_new: bool`, `is_deleted: bool`, `status: String`, `additions: u32`, `deletions: u32`), and `Client::checkout_details(&mut self, cwd: &str, cx) -> Task<anyhow::Result<CheckoutDetails>>` from TASK-157
- Consumes: `bachuc_client::FileDiff` (`path: String`, `old_path: Option<String>`, `is_new: bool`, `is_deleted: bool`, `status: FileDiffStatus`, `additions: u32`, `deletions: u32`, `text: String` as `file_diff` renders it), `bachuc_client::FileDiffStatus` (enum `Ok`, `TooLarge`, `Binary`), and `Client::checkout_diff(&mut self, cwd: &str, cx) -> Task<anyhow::Result<Vec<FileDiff>>>` from TASK-157
- Consumes: `bachuc_client::CommitList` (`base_ref: Option<String>`, `commits: Vec<CommitSummary>`), `bachuc_client::CommitSummary` (`sha`, `short_sha`, `subject`, `author_name`, `author_date: String`, `is_on_remote: bool`, `is_on_base: Option<bool>`, `files: Vec<CommitFile>`), `bachuc_client::CommitFile` (`path`, `additions: u32`, `deletions: u32`, `status: Option<String>`), `Client::commits(&mut self, cwd: &str, cx) -> Task<anyhow::Result<CommitList>>`, and `Client::commit_file_diff(&mut self, cwd: &str, sha: &str, path: &str, cx) -> Task<anyhow::Result<Option<FileDiff>>>` from TASK-157
- Consumes: `bachuc_theme::SyntaxThemeId` (enum with the eight ids of `themes.ts`, `id() -> &'static str`, `label() -> &'static str`, `from_id(&str) -> Option<Self>`, `ALL`), `bachuc_theme::set_syntax_theme(id: SyntaxThemeId, cx: &mut gpui_kit::App)`, `bachuc_theme::syntax_theme(cx: &gpui_kit::App) -> SyntaxThemeId`, and `bachuc_theme::syntax_colors(cx: &gpui_kit::App) -> bachuc_theme::SyntaxColors` (one `Hsla` field per highlight role of `colors.ts`) from TASK-159
- Produces: `bachuc_explorer::ExplorerDiffEvent` (enum `OpenAllDiffs { cwd: String, focus_path: Option<String> }`, `OpenCommitDiff { cwd: String, sha: String, path: String }`) emitted by `Explorer` through a second `EventEmitter`, `bachuc_explorer::DiffView::new_all(client: Entity<Client>, cwd: String, focus_path: Option<String>, window: &mut Window, cx: &mut Context<DiffView>) -> DiffView`, and `bachuc_explorer::DiffView::new_commit(client: Entity<Client>, cwd: String, sha: String, path: String, window: &mut Window, cx: &mut Context<DiffView>) -> DiffView` for TASK-167

Expected result:

- Changes opens one scroll with a header per file, in the daemon's file order, focused on the clicked file.
- The Commits footer lists commits and opens a commit file diff.

Anti-goal: a 20,000-line synthetic diff of 50 files parses and lays out its row model in under 200 ms in a debug test, and render builds rows only for the visible range; tripwire; read when the multi-file view lands and at completion.

## Verify

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-explorer` -> pass, including tests for the multi-file row model with `too_large` and `binary` files, an empty file list, a renamed file, a `focus_path` that is not in the list, a stale response dropped after a newer reload, new and deleted marks, `is_git: false`, a detached HEAD that stays a git state, a commit list with an error and an empty list, the commit-diff event, and the 20,000-line timing test
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-explorer` -> exit 0
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> exit 0
