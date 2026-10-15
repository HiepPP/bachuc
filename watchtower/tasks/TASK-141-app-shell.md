# TASK-141 App shell with host switcher, sidebar, and panes

Group: G (writes only `crates/app`)
Class: code

## Brief

Goal: Add the crate `bachuc-app`, the real desktop app. It joins the timeline, composer, terminal, and diagram parts into one window with a host switcher, a workspace and agent sidebar, tabs, a new-agent dialog, notifications, and a dock badge.

Change: separate crate examples -> one app binary `bachuc-desktop` for daily use.

Boundaries:

- One host at a time. The host switcher lists `load_hosts()` and reconnects on change. Switching hosts closes the old host's tabs.
- The sidebar groups agents by workspace name from `workspaces()` and shows status. Archived agents stay hidden.
- Tabs: each open agent is a tab with its timeline and composer. A terminal tab opens a terminal in the agent's `cwd`. Closing a terminal tab calls `close`.
- New agent: pick a provider and a model from `providers()`, pick a folder, type a prompt, and call `create_agent`. Providers with `available: false` are shown disabled.
- Notify with `App::show_system_notification` on `PermissionRequested` and `AgentFinished` when the window is not focused. If macOS refuses notifications for the unsigned bundle, record it in the outcome as a gap; do not sign the app in this TASK.
- The dock badge shows the sum of `pending_permissions` over all agents, and clears at zero. GPUI has no dock badge API, so set `NSDockTile` `badgeLabel` through `objc2-app-kit`.
- Light and dark follow the system appearance through `bachuc_theme::init`.
- Shortcuts: Cmd+N new agent, Cmd+T new terminal tab, Cmd+W close tab, Cmd+1 to Cmd+9 switch tabs.
- No daemon start, stop, update, CLI install, or deep links (see [watchtower/CONTEXT.md](watchtower/CONTEXT.md)).
- Give the sidebar rows, tabs, dialogs, and the host switcher a role and a label.

How:

- Build the window layout: sidebar, tab bar, pane area, and status line.
- Wire the client, the diagram renderer, and the views from the other crates.
- Add the new-agent dialog, notifications, dock badge, and shortcuts.
- Bundle as `packages/desktop-gpui/bundle.sh bin bachuc-desktop "Bachuc GPUI" io.github.hieppp.bachuc-desktop`.

Files:

- [packages/desktop-gpui/crates/app/Cargo.toml](packages/desktop-gpui/crates/app/Cargo.toml) (new crate `bachuc-app`, binary `bachuc-desktop`)
- [packages/desktop-gpui/crates/app/src](packages/desktop-gpui/crates/app/src) (window, sidebar, tabs, dialog, notifications, dock badge)

Interfaces:

- Consumes: `bachuc_theme::init(cx: &mut gpui_kit::App)` from TASK-134
- Consumes: `bachuc_theme::Scheme` (enum `Light`, `Dark`) and `bachuc_theme::scheme(cx: &gpui_kit::App) -> Scheme` from TASK-134
- Consumes: `bachuc_client::HostConfig { name: String, url: String, password: Option<String> }` and `bachuc_client::load_hosts() -> Vec<HostConfig>` from TASK-136
- Consumes: `bachuc_client::Client` (a GPUI entity) with `Client::connect(host: HostConfig, cx: &mut gpui_kit::App) -> gpui_kit::Entity<Client>` from TASK-136
- Consumes: `bachuc_client::ClientEvent` (enum `Connected`, `Disconnected { reason: String }`, `AgentsChanged`, `TimelineChanged { agent_id: String }`, `PermissionRequested { agent_id: String }`, `AgentFinished { agent_id: String }`) emitted by `Client` from TASK-136
- Consumes: `Client::agents(&self) -> &[AgentSummary]` with `AgentSummary { id: String, title: String, status: String, cwd: String, workspace_id: Option<String>, provider: String, model: Option<String>, pending_permissions: usize }` from TASK-136
- Consumes: `Client::workspaces(&self) -> &[WorkspaceSummary]` with `WorkspaceSummary { id: String, name: String, cwd: String }` from TASK-136
- Consumes: `Client::providers(&self) -> &[ProviderSummary]` with `ProviderSummary { id: String, label: String, available: bool, models: Vec<(String, String)> }` from TASK-136
- Consumes: `Client::create_agent(&mut self, provider: &str, model: Option<&str>, cwd: &str, prompt: &str, cx: &mut gpui_kit::Context<Client>) -> gpui_kit::Task<anyhow::Result<String>>` from TASK-136
- Consumes: `bachuc_diagram::DiagramRenderer` (a GPUI entity) with `DiagramRenderer::new(window: &mut gpui_kit::Window, cx: &mut gpui_kit::Context<DiagramRenderer>) -> DiagramRenderer` from TASK-137
- Consumes: `bachuc_timeline::TimelineView` (a GPUI view) with `TimelineView::new(client: gpui_kit::Entity<bachuc_client::Client>, agent_id: String, diagrams: gpui_kit::Entity<bachuc_diagram::DiagramRenderer>, window: &mut gpui_kit::Window, cx: &mut gpui_kit::Context<TimelineView>) -> TimelineView` from TASK-138
- Consumes: `bachuc_composer::Composer` (a GPUI view) with `Composer::new(client: gpui_kit::Entity<bachuc_client::Client>, agent_id: String, window: &mut gpui_kit::Window, cx: &mut gpui_kit::Context<Composer>) -> Composer` from TASK-139
- Consumes: `bachuc_terminal::TerminalView` (a GPUI view) with `TerminalView::new(client: gpui_kit::Entity<bachuc_client::Client>, cwd: String, window: &mut gpui_kit::Window, cx: &mut gpui_kit::Context<TerminalView>) -> TerminalView` and `TerminalView::close(&mut self, cx: &mut gpui_kit::Context<TerminalView>)` from TASK-140
- Produces: the bundle `target/bundles/Bachuc GPUI.app` built by `bundle.sh bin bachuc-desktop "Bachuc GPUI" io.github.hieppp.bachuc-desktop` for TASK-142, TASK-143

Expected result:

- The app opens on live, lists workspaces and agents, and opens an agent in a tab with its timeline and composer.
- Switching to release lists the release agents. The check sends no prompt and creates nothing on release.
- Cmd+N creates a `mock` agent on live that streams in a new tab.
- A permission request on a background agent shows a notification and a dock badge of 1. Resolving it clears the badge.
- Cases:
  - The host is down at start: the app shows a disconnected state and retries; it does not crash.
  - Zero agents: the sidebar shows an empty state.
  - A wrong host password shows the rejection reason.
  - Switching hosts with tabs open closes those tabs and shows the new host's sidebar.

Anti-goal: the app stays within 10% CPU and 250 MB phys_footprint, WebKit helpers included, with one mock stream tab and one terminal tab open, in the foreground for 60 seconds; tripwire; read with `measure.sh 60` after the layout works and at completion.

## Verify

- `packages/desktop-gpui/cargo.sh build --release -p bachuc-app` -> exit 0
- `packages/desktop-gpui/cargo.sh test -p bachuc-app` -> layout state, badge count, host switch, and shortcut tests pass
- Agent check: bundle and open the app, then use `cua_repl` `getAXState` -> sidebar rows, tabs, the host switcher, and the composer appear with labels
- Agent check: Cmd+N on live, provider `mock`, model `thirty-minute-stream`, folder `/tmp/gpui-bench` -> a new tab streams; `npm run cli -- ls -a -g` lists the agent
- Agent check: point the host list at a closed port -> the app shows disconnected and keeps retrying with backoff
- Agent check: switch from live to release with two tabs open -> the tabs close and the release agents show; nothing is sent to release
- Agent check: start a `claude` agent in `default` mode on live that asks for a tool approval, focus another app -> a dock badge of 1 shows and a notification shows or its refusal is recorded; approve it -> the badge clears
- Anti-goal: `measure.sh 60 app="Bachuc GPUI.app" wk="pids:<WebKit pids>"` -> CPU at most 10.0 and footprint sum at most 250
