# TASK-155 App integration and Electron window chrome

Group: K (writes `crates/app`)
Class: risky

## Brief

Goal: Join the parity views into the Bachuc GPUI app, with the Electron window layout and chrome, so the app looks and works like the Electron app for daily use.

Change: the app's own sidebar, tab bar, status line, and new-agent dialog -> the `bachuc-sidebar`, `bachuc-workspace`, `bachuc-explorer`, `bachuc-command`, and `bachuc-settings` views inside the Electron window layout.

Design: [packages/app/src/screens/workspace/workspace-screen.tsx](packages/app/src/screens/workspace/workspace-screen.tsx), [packages/app/src/components/left-sidebar.tsx](packages/app/src/components/left-sidebar.tsx), [packages/app/src/panels/new-tab-panel.tsx](packages/app/src/panels/new-tab-panel.tsx), [packages/app/src/components/desktop/titlebar-drag-region.tsx](packages/app/src/components/desktop/titlebar-drag-region.tsx), [packages/desktop/src/window/window-manager.ts](packages/desktop/src/window/window-manager.ts)

Boundaries:

- Window chrome: Electron uses `titleBarStyle: "hidden"` with `titleBarOverlay: true` and `trafficLightPosition` {16, 14} (`window-manager.ts` lines 17 and 58-60); the app sends a -4 offset (`packages/app/src/app/_layout.tsx` line 695) that `window-manager.ts` line 144 adds, so the lights sit at x 16 and y 10. In GPUI, set `TitlebarOptions` `appears_transparent: true` and `traffic_light_position` to that point (gpui-pre 0.3.9 has both), keep the 78x45 clearance, and make the area above the content drag the window.
- Layout: left `Sidebar`, center `Workspace`, right `Explorer` (toggled with the Electron shortcut), as in `workspace-screen.tsx`. The sidebar and the explorer resize and remember their widths.
- Workspaces, as in Electron: the sidebar selects a workspace, and the center shows that workspace's tabs. Keep one `Workspace` view per opened workspace. Opening a workspace opens tabs for its agents as `workspace-screen.tsx` does; `SidebarEvent::OpenWorkspace` selects it and `Sidebar::set_selected_workspace` marks it. The Electron new-tab panel ([packages/app/src/panels/new-tab-panel.tsx](packages/app/src/panels/new-tab-panel.tsx)) starts a new agent or a terminal in the selected workspace; it replaces the old new-agent dialog.
- Pane kinds the app opens in `Workspace`: agent (timeline and composer), terminal, file (`FileView`), and diff (`DiffView`). Wire `TimelineEvent::OpenFile`, `ExplorerEvent::OpenFile`, and `ExplorerEvent::OpenDiff` to open them. Agent tab menus offer rename and archive, as Electron's do.
- Wire `SidebarEvent`, `PaneEvent`, `CommandEvent`, and `SettingsEvent`; close terminal panes with `TerminalView::close`.
- Shortcuts come from `bachuc_command::load_shortcuts()`; reload them on `SettingsEvent::ShortcutsChanged`. The command center lists the app actions plus open agents and workspaces.
- Apply the saved appearance with `bachuc_settings::load_appearance` at start, after `bachuc_theme::init`.
- Remove the old chrome views (`chrome.rs` sidebar, tab bar, and the new-agent dialog) once the new views replace them. Keep notifications and the dock badge.
- Keep [ADR-0040](watchtower/decisions/ADR-0040-one-host-plain-ws-over-merged-hosts-and-relay.md) one host at a time, and [ADR-0041](watchtower/decisions/ADR-0041-gpui-client-connects-only-over-daemon-management.md) connect only.
- Fix the app accessibility gaps from [packages/desktop-gpui/docs/a11y-audit.md](packages/desktop-gpui/docs/a11y-audit.md): host options get a role and a selected state, the new-agent flow shows the selected provider and model, and every focusable element gets a role. Nobody has captured the `FocusId(5v1)` warning, so watch the log for "has no accessibility node" during the checks and fix each element it names.
- Keep the CPU limit from [watchtower/CONTEXT.md](watchtower/CONTEXT.md). Labeled chrome views stay uncached.

How:

- Replace the shell layout with the new views and the window chrome.
- Add the file and diff pane kinds and the event wiring.
- Wire shortcuts, the command center, settings, and appearance.
- Remove the old chrome code and its tests that no longer apply.

Files:

- [packages/desktop-gpui/crates/app](packages/desktop-gpui/crates/app) (`Cargo.toml`, `src`)
- [packages/desktop-gpui/Cargo.lock](packages/desktop-gpui/Cargo.lock) (generated)

Interfaces:

- Consumes: `bachuc_sidebar::Sidebar` (a GPUI view) with `Sidebar::new(client: gpui_kit::Entity<bachuc_client::Client>, window: &mut gpui_kit::Window, cx: &mut gpui_kit::Context<Sidebar>) -> Sidebar` and `Sidebar::set_selected_workspace(&mut self, workspace_id: Option<String>, cx: &mut gpui_kit::Context<Sidebar>)` from TASK-147
- Consumes: `bachuc_sidebar::SidebarEvent` (enum `OpenWorkspace { workspace_id: String }`, `OpenSettings`, `OpenCommandCenter`) emitted by `Sidebar` from TASK-147
- Consumes: `Client::projects(&self) -> &[ProjectNode]` with `ProjectNode { id: String, name: String, root: String, workspaces: Vec<WorkspaceNode> }` and `WorkspaceNode { id: String, name: String, cwd: String, branch: Option<String>, pinned: bool, agent_ids: Vec<String> }` from TASK-146
- Consumes: `Client::archive_agent(&mut self, agent_id: &str, cx: &mut gpui_kit::Context<Client>)` and `Client::rename_agent(&mut self, agent_id: &str, title: &str, cx: &mut gpui_kit::Context<Client>)` from TASK-146
- Consumes: `bachuc_workspace::Workspace` (a GPUI view) with `Workspace::new(window: &mut gpui_kit::Window, cx: &mut gpui_kit::Context<Workspace>) -> Workspace`, `Workspace::open(&mut self, item: PaneItem, window: &mut gpui_kit::Window, cx: &mut gpui_kit::Context<Workspace>)`, `Workspace::close_active(&mut self, window: &mut gpui_kit::Window, cx: &mut gpui_kit::Context<Workspace>)`, `Workspace::close(&mut self, key: &str, window: &mut gpui_kit::Window, cx: &mut gpui_kit::Context<Workspace>)`, `Workspace::split(&mut self, axis: SplitAxis, window: &mut gpui_kit::Window, cx: &mut gpui_kit::Context<Workspace>)`, `Workspace::activate_index(&mut self, index: usize, window: &mut gpui_kit::Window, cx: &mut gpui_kit::Context<Workspace>)`, and `Workspace::active_key(&self) -> Option<&str>` from TASK-148
- Consumes: `bachuc_workspace::PaneItem { key: String, title: gpui_kit::SharedString, icon: bachuc_icons::IconName, modified: bool, view: gpui_kit::AnyView }` and `bachuc_workspace::SplitAxis` (enum `Right`, `Down`) from TASK-148
- Consumes: `bachuc_workspace::PaneEvent` (enum `Closed { key: String }`, `Activated { key: String }`, `NewTab`) emitted by `Workspace` from TASK-148
- Consumes: `bachuc_timeline::TimelineEvent` (enum `OpenFile { cwd: String, path: String, line: Option<u32> }`) emitted by `TimelineView` from TASK-149
- Consumes: `bachuc_timeline::TimelineView::find(&mut self, window: &mut gpui_kit::Window, cx: &mut gpui_kit::Context<TimelineView>)` (opens the find bar) from TASK-149
- Consumes: the same `bachuc_composer::Composer::new(client: gpui_kit::Entity<bachuc_client::Client>, agent_id: String, window: &mut gpui_kit::Window, cx: &mut gpui_kit::Context<Composer>) -> Composer`, unchanged, from TASK-150
- Consumes: `bachuc_explorer::Explorer` (a GPUI view) with `Explorer::new(client: gpui_kit::Entity<bachuc_client::Client>, cwd: String, window: &mut gpui_kit::Window, cx: &mut gpui_kit::Context<Explorer>) -> Explorer` and `Explorer::set_cwd(&mut self, cwd: String, cx: &mut gpui_kit::Context<Explorer>)` from TASK-151
- Consumes: `bachuc_explorer::ExplorerEvent` (enum `OpenFile { cwd: String, path: String }`, `OpenDiff { cwd: String, path: String }`) emitted by `Explorer` from TASK-151
- Consumes: `bachuc_explorer::FileView::new(client: gpui_kit::Entity<bachuc_client::Client>, cwd: String, path: String, line: Option<u32>, window: &mut gpui_kit::Window, cx: &mut gpui_kit::Context<FileView>) -> FileView` and `bachuc_explorer::DiffView::new(client: gpui_kit::Entity<bachuc_client::Client>, cwd: String, path: String, window: &mut gpui_kit::Window, cx: &mut gpui_kit::Context<DiffView>) -> DiffView` from TASK-151
- Consumes: `bachuc_command::Shortcut { action: String, label: String, chord: Option<String> }`, `bachuc_command::default_shortcuts() -> Vec<Shortcut>`, `bachuc_command::load_shortcuts() -> Vec<Shortcut>` (defaults plus overrides), and `bachuc_command::save_overrides(shortcuts: &[Shortcut]) -> anyhow::Result<()>` from TASK-152
- Consumes: `bachuc_command::CommandItem { id: String, title: gpui_kit::SharedString, group: gpui_kit::SharedString, icon: Option<bachuc_icons::IconName>, shortcut: Option<String> }` and `bachuc_command::CommandCenter` (a GPUI view) with `CommandCenter::new(items: Vec<CommandItem>, window: &mut gpui_kit::Window, cx: &mut gpui_kit::Context<CommandCenter>) -> CommandCenter` from TASK-152
- Consumes: `bachuc_command::CommandEvent` (enum `Run { id: String }`, `Dismissed`) emitted by `CommandCenter` from TASK-152
- Consumes: `bachuc_settings::Settings` (a GPUI view) with `Settings::new(client: gpui_kit::Entity<bachuc_client::Client>, window: &mut gpui_kit::Window, cx: &mut gpui_kit::Context<Settings>) -> Settings` from TASK-153
- Consumes: `bachuc_settings::SettingsEvent` (enum `HostsChanged`, `ShortcutsChanged`, `Closed`) emitted by `Settings` from TASK-153
- Consumes: `bachuc_settings::load_appearance(cx: &mut gpui_kit::App)` (reads `appearance.json` and applies it) from TASK-153
- Consumes: the same `bachuc_terminal::TerminalView` API, unchanged, from TASK-154
- Produces: the bundle `target/bundles/Bachuc GPUI.app` built by `bundle.sh bin bachuc-desktop "Bachuc GPUI" io.github.hieppp.bachuc-desktop` for TASK-156

Expected result:

- The app window matches the reference app's main window on the same workspace: chrome, sidebar, tabs, agent pane, composer, and explorer.
- Daily flows work on live: open a workspace from the sidebar, open its agent tab, start a mock agent from the new-tab panel, send a prompt, split right with a terminal, open a file from a reply and from the explorer, open a diff, run a command from the command center, and change a setting.
- Cases:
  - The host is down at start: the app shows the Electron disconnected state and retries.
  - Zero projects: the sidebar shows the empty state and the center shows the Electron new-tab panel.
  - Switching hosts closes the old host's panes, as before.

Anti-goal: the app stays within 10% CPU and 250 MB, WebKit helpers included, with one mock stream tab and one terminal tab, window in front, at a turn age of about 2 and about 15 minutes; tripwire; read with `measure.sh 60` after the layout works and at completion at both ages.

## Verify

- `packages/desktop-gpui/cargo.sh build --release -p bachuc-app` -> exit 0
- `packages/desktop-gpui/cargo.sh test --workspace` -> pass
- Agent check: bundle the app; open the same mock agent in it and in the reference app on 6770 at the same window size; take `cua_repl` screenshots of the full window -> the parity list in the outcome shows match for chrome, layout, and each region, or records each mismatch
- Agent check on live: run each daily flow in Expected result with `cua_repl` -> each works; `npm run cli -- ls -a -g` shows the agent and the sent prompt
- Agent check: point the host list at a closed port -> the app shows disconnected and retries
- Agent check: with an agent pane and a terminal pane open, switch the theme in settings from Light to Claude dark -> every region recolors at once, with no restart
- Agent check: resize the window to 900 px wide with the sidebar and explorer open -> nothing overlaps; tabs shrink as Electron's do; the sidebar keeps its min width
- Agent check: `cua_repl` `getAXState` -> host options with selected state, the new-agent selection, and every region have roles and labels; the log has no "has no accessibility node" warning during the checks
- Anti-goal: `measure.sh 60 app="pids:<app pid>" wk="pids:<WebKit pids>"` at about 2 and about 15 minutes -> CPU at most 10.0 and footprint sum at most 250 each
