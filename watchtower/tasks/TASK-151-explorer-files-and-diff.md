# TASK-151 Explorer with files, a file viewer, and git changes

Group: G (writes the new `crates/explorer`)
Class: code

## Brief

Goal: Add the crate `bachuc-explorer` with the Electron explorer sidebar (files and changes), a file viewer pane, and a diff viewer pane.

Change: no file or git UI in the GPUI client -> a files tree, a changes list, a read-only file viewer with syntax colors, and a unified diff viewer, as in Electron.

Design: [docs/explorer-sidebar.md](docs/explorer-sidebar.md), [packages/app/src/screens/workspace/explorer-sidebar.tsx](packages/app/src/screens/workspace/explorer-sidebar.tsx), [packages/app/src/screens/workspace/explorer-sidebar-tab-rail.tsx](packages/app/src/screens/workspace/explorer-sidebar-tab-rail.tsx), [packages/app/src/components/file-explorer-pane.tsx](packages/app/src/components/file-explorer-pane.tsx), [packages/app/src/panels/diff-panel.tsx](packages/app/src/panels/diff-panel.tsx), [packages/app/src/components/diff-viewer.tsx](packages/app/src/components/diff-viewer.tsx), [packages/app/src/file-icons](packages/app/src/file-icons)

Boundaries:

- The explorer sidebar has the Electron tab rail with Files and Changes.
- Files: a lazy tree from `list_directory`, folders first, the Electron row size and file icons ([docs/file-icons.md](docs/file-icons.md)); a click emits `ExplorerEvent::OpenFile`.
- Changes: the files from `checkout_status` as a compressed folder tree with collapsible folders, as [packages/app/src/git/changed-files-tree.tsx](packages/app/src/git/changed-files-tree.tsx) lines 36-40 draws it, with `+N -M` counts. It refreshes on `DirectoryEvent::CheckoutChanged` while shown (`watch_checkout` on show, `unwatch_checkout` on hide); a click emits `ExplorerEvent::OpenDiff`. A folder that is not a git repo (`branch: None`) shows the Electron non-git state; a clean checkout shows the "no changes" state.
- File viewer: read-only, with line numbers, syntax colors through the gpui-kit highlighter (only the tree-sitter languages that resolve: see [watchtower/CONTEXT.md](watchtower/CONTEXT.md)), and scroll to a given line. `FileKind::Image` shows the image; `FileKind::Binary` and a file over the read limit show the Electron placeholder. No editing.
- Diff viewer: unified diff with the Electron colors (`diff_addition`, `diff_deletion`, `surface_diff_empty`), line numbers, and a 22 px line height. A `too_large` or `binary` file shows the Electron placeholder.
- Out of scope: commit, push, pull, PR panel, discard, and branch switching.

How:

- Write the tree and changes models with tests.
- Build the explorer sidebar, the file viewer, and the diff viewer.
- Add `examples/explorer.rs` that shows the sidebar and opens a file and a diff on a live workspace.

Files:

- [packages/desktop-gpui/crates/explorer](packages/desktop-gpui/crates/explorer) (new crate `bachuc-explorer`: `Cargo.toml`, `src`, `tests`, `examples/explorer.rs`)

Interfaces:

- Consumes: `bachuc_theme::tokens(cx: &gpui_kit::App) -> Tokens` from TASK-145
- Consumes: `bachuc_theme::metrics` (module of `pub const` `f32` values: `SPACE_0_5`, `SPACE_1`, `SPACE_1_5`, `SPACE_2`, `SPACE_3`, `SPACE_4`, `SPACE_6`, `SPACE_8`, `RADIUS_SM`, `RADIUS_BASE`, `RADIUS_MD`, `RADIUS_LG`, `RADIUS_XL`, `RADIUS_2XL`, `FONT_CODE`, `FONT_SM`, `FONT_BASE`, `FONT_CONTENT`, `FONT_LG`, `FONT_XL`, `FONT_2XL`, `ICON_XS`, `ICON_SM`, `ICON_MD`, `ICON_LG`, `CONTROL_XS`, `CONTROL_SM`, `CONTROL_MD`, `HEADER_HEIGHT`, `TAB_BAR_HEIGHT`, `MAX_CONTENT_WIDTH`, `SIDEBAR_WIDTH`, `SIDEBAR_MIN_WIDTH`, `SIDEBAR_MAX_WIDTH`, `TRAFFIC_LIGHT_WIDTH`, `TRAFFIC_LIGHT_HEIGHT`, `SETTINGS_SIDEBAR_WIDTH`, `SETTINGS_DETAIL_MIN_WIDTH`, `SETTINGS_CONTENT_WIDTH`) from TASK-145
- Consumes: `bachuc_icons::IconName` (enum with one variant per generated icon, named as the lucide import, for example `FolderPlus`, `GitBranch`, `X`), `bachuc_icons::icon(name: IconName) -> gpui_kit::component::Icon` (sized with `ICON_MD`; change it with `with_size` and `text_color`), and `bachuc_icons::Assets` (a GPUI `AssetSource` that serves the SVGs and falls through to the gpui-kit assets) from TASK-145
- Consumes: `bachuc_client::DirectoryEvent` (enum `ProjectsChanged`, `CheckoutChanged { cwd: String }`) emitted by `Client` from TASK-146
- Consumes: `Client::list_directory(&mut self, cwd: &str, path: &str, cx: &mut gpui_kit::Context<Client>) -> gpui_kit::Task<anyhow::Result<Vec<FileEntry>>>` with `FileEntry { name: String, path: String, is_dir: bool, size: u64 }` from TASK-146
- Consumes: `Client::read_file(&mut self, cwd: &str, path: &str, cx: &mut gpui_kit::Context<Client>) -> gpui_kit::Task<anyhow::Result<FileContent>>` with `FileContent { path: String, kind: FileKind, text: Option<String>, bytes: Option<Vec<u8>>, mime_type: Option<String>, size: u64 }` and `bachuc_client::FileKind` (enum `Text`, `Image`, `Binary`) from TASK-146
- Consumes: `Client::checkout_status(&mut self, cwd: &str, cx: &mut gpui_kit::Context<Client>) -> gpui_kit::Task<anyhow::Result<CheckoutStatus>>` with `CheckoutStatus { branch: Option<String>, files: Vec<ChangedFile> }` and `ChangedFile { path: String, status: String, added: u32, removed: u32 }`, `Client::watch_checkout(&mut self, cwd: &str, cx: &mut gpui_kit::Context<Client>)`, `Client::unwatch_checkout(&mut self, cwd: &str, cx: &mut gpui_kit::Context<Client>)`, and `Client::file_diff(&mut self, cwd: &str, path: &str, cx: &mut gpui_kit::Context<Client>) -> gpui_kit::Task<anyhow::Result<String>>` (unified diff text) from TASK-146
- Produces: `bachuc_explorer::Explorer` (a GPUI view) with `Explorer::new(client: gpui_kit::Entity<bachuc_client::Client>, cwd: String, window: &mut gpui_kit::Window, cx: &mut gpui_kit::Context<Explorer>) -> Explorer` and `Explorer::set_cwd(&mut self, cwd: String, cx: &mut gpui_kit::Context<Explorer>)` for TASK-155
- Produces: `bachuc_explorer::ExplorerEvent` (enum `OpenFile { cwd: String, path: String }`, `OpenDiff { cwd: String, path: String }`) emitted by `Explorer` for TASK-155
- Produces: `bachuc_explorer::FileView::new(client: gpui_kit::Entity<bachuc_client::Client>, cwd: String, path: String, line: Option<u32>, window: &mut gpui_kit::Window, cx: &mut gpui_kit::Context<FileView>) -> FileView` and `bachuc_explorer::DiffView::new(client: gpui_kit::Entity<bachuc_client::Client>, cwd: String, path: String, window: &mut gpui_kit::Window, cx: &mut gpui_kit::Context<DiffView>) -> DiffView` for TASK-155

Expected result:

- The explorer, file viewer, and diff viewer match the reference app on the same workspace.
- Editing a file on disk updates the Changes list within the daemon's watch delay.
- Cases:
  - An empty folder shows the Electron empty row.
  - A clean checkout shows the Electron "no changes" state.
  - A file larger than the read limit shows the placeholder, not a hang.

Anti-goal: the explorer example stays at or below 1% CPU while idle with Changes shown and a watched checkout; tripwire; read with `measure.sh 60` after the changes list lands and at completion.

## Verify

- `packages/desktop-gpui/cargo.sh test -p bachuc-explorer` -> pass, including tree order, lazy expand, changes grouping, and diff parse tests
- Agent check: bundle `packages/desktop-gpui/bundle.sh example explorer BachucExplorer io.github.hieppp.bachuc-explorer`; compare Files, Changes, a file view, and a diff view with the reference app on 6770 using `cua_repl` screenshots -> the parity list in the outcome shows match or records each mismatch
- Agent check on live: edit a file in `/tmp/gpui-bench` (a git repo the check creates there) -> the Changes list shows it with counts
- Agent check: open the explorer on a folder that is not a git repo, a PNG, and a binary file -> Changes shows the non-git state, the PNG renders, and the binary file shows the placeholder
- Agent check: `cua_repl` `getAXState` -> tree rows, the tab rail, change rows, and viewer lines have roles and labels
- Agent check: open the explorer on the repo root (more than 10,000 files) and expand `packages/app/src` -> the folder opens in under 1 second and scrolling stays smooth; only expanded folders are listed
- Anti-goal: `measure.sh 60 ex="pids:<pid>"` idle with Changes shown -> CPU at most 1.0
