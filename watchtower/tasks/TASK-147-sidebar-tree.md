# TASK-147 Sidebar with the project and workspace tree

Group: C (writes the new `crates/sidebar`)
Class: code

## Brief

Goal: Add the crate `bachuc-sidebar` with a sidebar that looks and works like the Electron left sidebar in its default tree mode.

Change: the GPUI sidebar is a flat list of agents grouped by workspace name -> the Electron project > workspace tree with workspace status, hover kebab menus, collapse, and nav rows.

Design: [packages/app/src/components/left-sidebar.tsx](packages/app/src/components/left-sidebar.tsx), [packages/app/src/components/sidebar-workspace-list.tsx](packages/app/src/components/sidebar-workspace-list.tsx), [packages/app/src/components/sidebar/sidebar-workspace-row.tsx](packages/app/src/components/sidebar/sidebar-workspace-row.tsx), [packages/app/src/components/sidebar/sidebar-workspace-menu.tsx](packages/app/src/components/sidebar/sidebar-workspace-menu.tsx), [packages/app/src/components/sidebar/sidebar-nav-rows.tsx](packages/app/src/components/sidebar/sidebar-nav-rows.tsx), [packages/app/src/components/agent-status-dot.tsx](packages/app/src/components/agent-status-dot.tsx)

Boundaries:

- Owner answer (2026-10-16): port the tree mode with status. The status-grouped list and the display options are out of scope.
- Follow Electron exactly: the tree is project > workspace. There are no agent rows; agents open as tabs of their workspace (TASK-155).
- Row sizes from `sidebar-workspace-list.tsx` lines 2541-2700: list padding 8 horizontal, 2 top, 16 bottom; project row min height 36, padding 8, radius 8, gap 8; workspace row min height 36, padding 8 vertical, 8 left, 12 right, radius 8; kebab buttons 24x24, radius 6, margin right -6 (line 2649). Use `bachuc_theme::metrics` and `bachuc_theme::tokens`.
- States: hover `surface_sidebar_hover`, pressed `surface2`, selected `surface_sidebar_selected` (`sidebar-workspace-list.tsx` line 2735), kebab shown on hover.
- Status is per workspace, as Electron's `SidebarStateBucket` computes it from the workspace's agents (running, needs attention, permission waiting, unread, idle). Port that bucket logic and its dot colors.
- Workspace menu, as `sidebar-workspace-menu.tsx` lines 184-245: rename, pin or unpin, copy path, mark as unread, and archive.
- Archive confirm, as [packages/app/src/git/worktree-archive-warning.ts](packages/app/src/git/worktree-archive-warning.ts) lines 116-130 and [packages/app/src/workspace/use-workspace-archive.ts](packages/app/src/workspace/use-workspace-archive.ts) lines 85-108: confirm only for a risky worktree or when the shortcut starts it. Planner call: treat a workspace as risky when `checkout_status` lists changed files; the "ahead of remote" case is a recorded gap, because the client does not expose it.
- Nav rows: port the rows of `sidebar-nav-rows.tsx` whose target is in scope (settings, command center); leave out schedules and sessions (see Decisions in [watchtower/CONTEXT.md](watchtower/CONTEXT.md)) and list them in the outcome.
- The sidebar is resizable between `SIDEBAR_MIN_WIDTH` and `SIDEBAR_MAX_WIDTH` with a drag handle; it starts at `SIDEBAR_WIDTH`.
- The sidebar emits events; it does not open tabs itself. TASK-155 wires the events.

How:

- Build the tree model and the status bucket from `projects()` and `agents()` with tests.
- Build the rows, menu, confirm, collapse state, nav rows, and resize handle.
- Add `examples/sidebar.rs` that connects to a host and shows the sidebar alone.

Files:

- [packages/desktop-gpui/crates/sidebar](packages/desktop-gpui/crates/sidebar) (new crate `bachuc-sidebar`: `Cargo.toml`, `src`, `tests`, `examples/sidebar.rs`)

Interfaces:

- Consumes: `bachuc_client::Client` (a GPUI entity) with `Client::connect(host: HostConfig, cx: &mut gpui_kit::App) -> gpui_kit::Entity<Client>` from TASK-136
- Consumes: `Client::agents(&self) -> &[AgentSummary]` with `AgentSummary { id: String, title: String, status: String, cwd: String, workspace_id: Option<String>, provider: String, model: Option<String>, pending_permissions: usize }` from TASK-136
- Consumes: `bachuc_client::DirectoryEvent` (enum `ProjectsChanged`, `CheckoutChanged { cwd: String }`) emitted by `Client` from TASK-146
- Consumes: `Client::projects(&self) -> &[ProjectNode]` with `ProjectNode { id: String, name: String, root: String, workspaces: Vec<WorkspaceNode> }` and `WorkspaceNode { id: String, name: String, cwd: String, branch: Option<String>, pinned: bool, agent_ids: Vec<String> }` from TASK-146
- Consumes: `Client::rename_workspace(&mut self, workspace_id: &str, title: &str, cx: &mut gpui_kit::Context<Client>)`, `Client::pin_workspace(&mut self, workspace_id: &str, pinned: bool, cx: &mut gpui_kit::Context<Client>)`, `Client::archive_workspace(&mut self, workspace_id: &str, cx: &mut gpui_kit::Context<Client>)`, and `Client::mark_workspace_unread(&mut self, workspace_id: &str, cx: &mut gpui_kit::Context<Client>)` from TASK-146
- Consumes: `Client::checkout_status(&mut self, cwd: &str, cx: &mut gpui_kit::Context<Client>) -> gpui_kit::Task<anyhow::Result<CheckoutStatus>>` with `CheckoutStatus { branch: Option<String>, files: Vec<ChangedFile> }` and `ChangedFile { path: String, status: String, added: u32, removed: u32 }`, `Client::watch_checkout(&mut self, cwd: &str, cx: &mut gpui_kit::Context<Client>)`, `Client::unwatch_checkout(&mut self, cwd: &str, cx: &mut gpui_kit::Context<Client>)`, and `Client::file_diff(&mut self, cwd: &str, path: &str, cx: &mut gpui_kit::Context<Client>) -> gpui_kit::Task<anyhow::Result<String>>` (unified diff text) from TASK-146
- Consumes: `bachuc_theme::tokens(cx: &gpui_kit::App) -> Tokens` from TASK-145
- Consumes: `bachuc_theme::metrics` (module of `pub const` `f32` values: `SPACE_0_5`, `SPACE_1`, `SPACE_1_5`, `SPACE_2`, `SPACE_3`, `SPACE_4`, `SPACE_6`, `SPACE_8`, `RADIUS_SM`, `RADIUS_BASE`, `RADIUS_MD`, `RADIUS_LG`, `RADIUS_XL`, `RADIUS_2XL`, `FONT_CODE`, `FONT_SM`, `FONT_BASE`, `FONT_CONTENT`, `FONT_LG`, `FONT_XL`, `FONT_2XL`, `ICON_XS`, `ICON_SM`, `ICON_MD`, `ICON_LG`, `CONTROL_XS`, `CONTROL_SM`, `CONTROL_MD`, `HEADER_HEIGHT`, `TAB_BAR_HEIGHT`, `MAX_CONTENT_WIDTH`, `SIDEBAR_WIDTH`, `SIDEBAR_MIN_WIDTH`, `SIDEBAR_MAX_WIDTH`, `TRAFFIC_LIGHT_WIDTH`, `TRAFFIC_LIGHT_HEIGHT`, `SETTINGS_SIDEBAR_WIDTH`, `SETTINGS_DETAIL_MIN_WIDTH`, `SETTINGS_CONTENT_WIDTH`) from TASK-145
- Consumes: `bachuc_icons::IconName` (enum with one variant per generated icon, named as the lucide import, for example `FolderPlus`, `GitBranch`, `X`), `bachuc_icons::icon(name: IconName) -> gpui_kit::component::Icon` (sized with `ICON_MD`; change it with `with_size` and `text_color`), and `bachuc_icons::Assets` (a GPUI `AssetSource` that serves the SVGs and falls through to the gpui-kit assets) from TASK-145
- Produces: `bachuc_sidebar::Sidebar` (a GPUI view) with `Sidebar::new(client: gpui_kit::Entity<bachuc_client::Client>, window: &mut gpui_kit::Window, cx: &mut gpui_kit::Context<Sidebar>) -> Sidebar` and `Sidebar::set_selected_workspace(&mut self, workspace_id: Option<String>, cx: &mut gpui_kit::Context<Sidebar>)` for TASK-155
- Produces: `bachuc_sidebar::SidebarEvent` (enum `OpenWorkspace { workspace_id: String }`, `OpenSettings`, `OpenCommandCenter`) emitted by `Sidebar` for TASK-155

Expected result:

- The example shows the projects and workspaces of a live host as a tree, with the Electron sizes, colors, icons, and status.
- Rename, pin, copy path, mark as unread, and archive work on live.
- Cases:
  - Zero projects shows the Electron empty state.
  - A long project or workspace title truncates with an ellipsis and keeps the kebab on its rail.
  - An agent that starts running changes its workspace status without a redraw of the whole list.
  - Archive of a workspace with changed files asks for a confirm; a clean one archives at once.

Anti-goal: the sidebar example stays at or below 1% CPU while idle and at or below 3% while a mock agent streams in a listed workspace; tripwire; read with `measure.sh 60` after the tree renders and at completion.

## Verify

- `packages/desktop-gpui/cargo.sh test -p bachuc-sidebar` -> pass, including empty, collapse, pinned order, the status bucket cases, and the risky-archive rule
- Agent check: bundle `packages/desktop-gpui/bundle.sh example sidebar BachucSidebar io.github.hieppp.bachuc-sidebar`; open it and the reference app at the same size; take `cua_repl` screenshots of both sidebars -> the parity list in the outcome shows match for row heights, paddings, colors, icons, status, hover, and selected states in both themes, or records each mismatch
- Agent check on live: rename a workspace, pin it, mark it unread, and archive a test workspace through the menu -> `npm run cli -- ls -a -g` and the sidebar show the changes
- Agent check: `cua_repl` `getAXState` -> project and workspace rows and kebab buttons have roles and labels
- Anti-goal: `measure.sh 60 sb="pids:<pid>"` idle and with a mock stream -> CPU at most 1.0 idle and at most 3.0 streaming
