# TASK-167 App wiring for the follow-ups

Group: K (writes `crates/app`)
Class: code

## Brief

Goal: Wire the follow-up features into the app: multi-file and commit diff tabs, provider logos on agent tabs, tab menu actions and maximize, and the font and syntax settings at startup.

Change: the app opens one-file diff tabs, agent tabs show a bot, and only theme and sizes apply at startup -> the new explorer and tab events, tab logos, and the full appearance at startup.

Boundaries:

- Subscribe to `ExplorerDiffEvent` and open `DiffView::new_all` and `DiffView::new_commit` tabs with stable identities, as the shell does for `file:` and `diff:`. Keep the existing `ExplorerEvent` handling.
- Set each agent tab's logo with `Workspace::set_item_logo` from the agent provider, with `provider_fallback_icon` when there is no logo; refresh it on a theme change.
- Subscribe to `TabMenuEvent`, and bind maximize to `toggle_maximize`.
- Call `bachuc_settings::load_appearance` at startup as today; it now applies fonts and the syntax theme.
- Keep the test `every_focus_target_in_this_crate_has_a_role` and every existing app test.
- From the run (lead, 2026-10-16): a Changes row click now emits only `ExplorerDiffEvent::OpenAllDiffs` (Q-096), so the app must handle it or Changes opens nothing. Call `Workspace::set_item_kind` with the `TabKind` of each tab after open (Q-097), and bind the workspace `OpenTabMenu` action to the menu key and `shift-f10`. `Explorer` and `Workspace` now have two `EventEmitter` impls, so each `cx.subscribe` names its event type. Read [TASK-162-outcome.md](watchtower/tasks/TASK-162-outcome.md) and [TASK-165-outcome.md](watchtower/tasks/TASK-165-outcome.md) for the exact contracts.

Files:

- [packages/desktop-gpui/crates/app](packages/desktop-gpui/crates/app) (`Cargo.toml`, `src`)
- [packages/desktop-gpui/Cargo.lock](packages/desktop-gpui/Cargo.lock) (generated)

Interfaces:

- Consumes: `bachuc_icons::provider_logo_svg(provider: &str) -> Option<&'static str>`, `bachuc_icons::provider_logo(provider: &str, fill: gpui_kit::Hsla) -> Option<std::sync::Arc<gpui_kit::Image>>`, and `bachuc_icons::provider_fallback_icon(provider: &str) -> bachuc_icons::IconName` from TASK-158
- Consumes: `bachuc_explorer::ExplorerDiffEvent` (enum `OpenAllDiffs { cwd: String, focus_path: Option<String> }`, `OpenCommitDiff { cwd: String, sha: String, path: String }`) emitted by `Explorer` through a second `EventEmitter`, `bachuc_explorer::DiffView::new_all(client: Entity<Client>, cwd: String, focus_path: Option<String>, window: &mut Window, cx: &mut Context<DiffView>) -> DiffView`, and `bachuc_explorer::DiffView::new_commit(client: Entity<Client>, cwd: String, sha: String, path: String, window: &mut Window, cx: &mut Context<DiffView>) -> DiffView` from TASK-162
- Consumes: `bachuc_workspace::Workspace::set_item_logo(&mut self, key: &str, logo: Option<std::sync::Arc<gpui_kit::Image>>, cx: &mut Context<Workspace>)`, `bachuc_workspace::Workspace::toggle_maximize(&mut self, window: &mut Window, cx: &mut Context<Workspace>)`, and `bachuc_workspace::TabMenuEvent` (one variant per `workspace-tab-menu.ts` action that the app owns, each with the tab `key: String`) emitted by `Workspace` through a second `EventEmitter`, from TASK-165
- Consumes: the same `bachuc_settings::Settings`, `SettingsEvent`, and `load_appearance` API from TASK-160
- Consumes: the same `bachuc_sidebar::Sidebar` and `SidebarEvent` API from TASK-161
- Consumes: the same `bachuc_composer::Composer` and `ComposerEvent` API from TASK-163
- Consumes: the same `bachuc_timeline::TimelineView` and `TimelineEvent` API from TASK-164

Expected result:

- Clicking a changed file opens the all-files diff tab focused on it; clicking a commit file opens its commit diff tab.
- A Claude agent tab shows the Claude logo.

Anti-goal: the shell does not redraw on `TimelineChanged`, as today; tripwire: the existing app tests pass and a test asserts the logo refresh runs only on a theme change or a tab open; read at completion.

## Verify

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-app` -> pass, including the existing 26 tests, an identity test for the all-files and commit diff tabs, and the logo-refresh test
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-app` -> exit 0
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build --release -p bachuc-app` -> exit 0
