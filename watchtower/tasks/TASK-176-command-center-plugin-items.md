# TASK-176 Command Center and slash plugin items

Group: F (writes `crates/command`; the composer slash list is read by TASK-172 from the plugin host, so no composer file changes here)
Class: code

## Brief

Goal: Show the Command Center items of the active plugins with Electron's look and ranking, run them in place, and fix where plugin slash commands live so no TASK edits the composer for them.

Change: the Command Center lists only the app's actions, workspaces, and agents -> plugin items come first, one section per plugin, matched by title and hidden keywords, gated by context and `visible`, and run by the Command Center itself.

Design: Electron builds the plugin rows in [packages/app/src/plugins/command-center/contributions.ts](packages/app/src/plugins/command-center/contributions.ts) (`buildPluginCommandCenterContributions` line 20, context gate lines 26-27, row fields lines 80-94), registers them in [packages/app/src/plugins/command-center/registration.tsx](packages/app/src/plugins/command-center/registration.tsx) (workspace lines 21-23, enable rule line 83), puts contribution sections first in [packages/app/src/command-center/command-center.tsx](packages/app/src/command-center/command-center.tsx) (lines 287-297), and ranks them in [packages/app/src/command-center/results.ts](packages/app/src/command-center/results.ts) (visible match over keyword match line 39, section order line 187, empty query line 280). Slash commands: [packages/app/src/plugins/client-slash-commands/index.ts](packages/app/src/plugins/client-slash-commands/index.ts) (line 17) and [packages/app/src/plugins/client-slash-commands/model.ts](packages/app/src/plugins/client-slash-commands/model.ts) (merge line 15, resolve line 54).

Boundaries:

- Follows [ADR-0045](watchtower/decisions/ADR-0045-native-rust-plugin-clients-over-webview-panes-or-shared-logic.md): read items only from `PluginHost::index().command_items()`; a plugin the host does not report active adds nothing.
- Keep `CommandItem`, `CommandEvent`, `CommandCenter::new`, `set_items`, and `SearchIndex::new` as they are. `crates/app/src/state.rs:317-355` builds `CommandItem` with struct literals, and `shell.rs:1513-1518` matches `CommandEvent` exhaustively. Keep plugin keywords in a private side table, and add keyword search as a new `SearchIndex` constructor.
- `CommandCenter::new` appends nothing to the caller's items. It reads the host global, builds plugin rows, and puts them before the caller's items, one section per plugin titled with the plugin id, in index order. Row id: `plugin:<plugin_id>:<item_id>`. Rows show with an empty query.
- Context gate: `Global` always; `Workspace` only with a workspace id; `Agent` only with a workspace id and an agent id; then `visible`, when set, must return true. The ids come from `set_plugin_context`, which the app calls after it creates the Command Center (TASK-191). Before that call, only `Global` items show (Planner call: the host gives no getter for its active ids).
- Ranking: title matches use today's tiers (`search.rs:21-26`). A keyword match ranks below every title or group match, and keywords are never drawn. Two rows with equal score keep index order.
- Run (Planner call): on Enter or click of a plugin row, the Command Center calls the item's `on_select` with `CommandInput`, then emits `CommandEvent::Run` with the `plugin:` id. The shell already ignores an unknown prefix (`shell.rs:1526-1534`), closes the palette, and does not refocus, so `on_select` may move focus. No app edit is needed.
- While open, rebuild the plugin rows on `PluginHostEvent::ContributionsChanged` and on `PluginStateChanged` (a `visible` result may change), keeping the query and the selected row id when it still exists.
- Rows use today's row look (`center.rs:208-316`): icon slot 16 by 20 with the item `icon` at 16 px in `foreground_muted`, title `FONT_BASE`, row height 36. Section titles and dividers use the existing `Section` rows of `search.rs`.
- Slash commands (Planner call): TASK-172 draws `ContributionIndex::slash_commands` in the composer slash list and resolves `/name args` on send, as `model.ts:15-60` does. This TASK adds no slash code and edits no composer file. Plugin slash commands do not appear in the Command Center, as in Electron.
- Keep each rule in a pure function and test it: GPUI view crates have no headless test context here (MEMORY 2026-10-16).

Files:

- [packages/desktop-gpui/crates/command](packages/desktop-gpui/crates/command) (`Cargo.toml` adds `bachuc-plugin-host`, `src`, `tests`, `examples/command.rs` if it no longer builds)

Interfaces:

- Consumes: `bachuc_plugin_host::init(plugins: Vec<Box<dyn NativePlugin>>, cx: &mut App) -> Entity<PluginHost>` (stores the host as a GPUI global), `bachuc_plugin_host::host(cx: &App) -> Option<Entity<PluginHost>>`, `PluginHost::set_client(&mut self, client: Entity<Client>, cx: &mut Context<PluginHost>)`, `PluginHost::set_active(&mut self, workspace_id: Option<String>, agent_id: Option<String>, cx: &mut Context<PluginHost>)`, `PluginHost::is_active(&self, plugin_id: &str) -> bool`, `PluginHost::index(&self) -> &ContributionIndex`, `PluginHost::timeline_revision(&self) -> u64`, and `bachuc_plugin_host::active_plugin_ids(listed: &[PluginListItem], registered: &[&'static str]) -> Vec<&'static str>` from TASK-170
- Consumes: `bachuc_plugin_host::PluginHostEvent` (`#[non_exhaustive]` enum, emitted by `PluginHost`: `ContributionsChanged`, `PluginStateChanged { plugin_id: &'static str }`, `TimelineTransformsChanged`, `SidebarFilterChanged { direction: Option<SwipeDirection> }`, `OpenSurface { plugin_id: String, surface_id: String, presentation: SurfacePresentation, params: std::collections::BTreeMap<String, String> }`, `OpenPanel { plugin_id: &'static str, panel_id: String, target: PanelTarget }`, `OpenSettings { plugin_id: &'static str, settings_id: String }`, `SetComposerText { agent_id: String, text: String }`, `AddComposerAttachment(ComposerAttachment)`, `RevealTimelinePassage(TimelinePassage)`, `OpenNewWorkspace(NewWorkspaceRequest)`, `OpenPluginsPage`, `OpenAgent { agent_id: String }`, `OpenWorkspace { workspace_id: String }`), `bachuc_plugin_host::PanelTarget` (`workspace_id: String`, `agent_id: Option<String>`, `location: Option<PanelLocation>`), `bachuc_plugin_host::TimelinePassage` (`agent_id: String`, `message_id: String`, `text: Option<String>`), and `bachuc_plugin_host::NewWorkspaceRequest` (`cwd: String`, `project_id: Option<String>`, `name: Option<String>`) from TASK-170
- Consumes: `bachuc_plugin_host::AgentAnchor` (enum `Latest`, `LatestPrompt`) and `bachuc_plugin_host::MessageLevel` (enum `Info`, `Error`); the host shows a `show_message` as a gpui-kit notification, routes `open_agent_at` like `open_agent` plus the anchor, and `PluginContext::notify` emits `PluginHostEvent::PluginStateChanged { plugin_id }` so host views redraw that plugin's contributions, from TASK-170
- Consumes: `bachuc_plugin_host::Contributed<T>` (`plugin_id: &'static str`, `value: T`) and `bachuc_plugin_host::ContributionIndex` (getters that each return `&[Contributed<T>]`: `composer_pills`, `composer_interceptors`, `stop_buttons`, `attachment_sources`, `sidebar_items`, `sidebar_sections`, `project_filters`, `project_menus`, `command_items`, `slash_commands`, `settings_screens`, `surfaces`, `workspace_panels`, `new_workspace_panels`, `shortcuts`, `header_subtitles`, `selection_actions`, `themes`; plus `transformers_for(&self, item_type: &str) -> &[Contributed<TimelineTransformer>]`, `renderer_for(&self, kind: &str, version: u32) -> Option<&Contributed<TimelineRenderer>>`, and `surface(&self, plugin_id: &str, surface_id: &str) -> Option<&Contributed<Surface>>`) from TASK-170
- Consumes: command kinds `bachuc_plugin_host::CommandCenterItem` (`id`, `title`, `icon`, `keywords: Vec<SharedString>`, `context: CommandContext`, `visible: Option<std::rc::Rc<dyn Fn(&App) -> bool>>`, `on_select: Handler<CommandInput>`), `CommandContext` (enum `Global`, `Workspace`, `Agent`), `CommandInput` (`workspace_id: Option<String>`, `agent_id: Option<String>`), `PluginSlashCommand` (`name: &'static str`, `description: SharedString`, `argument_hint: SharedString`, `context: CommandContext`, `on_submit: Handler<SlashInput>`), and `SlashInput` (`workspace_id: Option<String>`, `agent_id: Option<String>`, `args: String`) from TASK-170
- Produces: `CommandCenter::set_plugin_context(&mut self, workspace_id: Option<String>, agent_id: Option<String>, cx: &mut Context<CommandCenter>)` (rebuilds the plugin rows for the new context) and the rule that a plugin row emits `CommandEvent::Run { id }` with an id that starts with `plugin:` after it ran, which the app ignores, for TASK-191

Expected result:

- With an active plugin that adds a global item, the Command Center shows the plugin's section first with the item, and Enter runs it and closes the palette.
- Cases:
  - No active plugin -> the list equals today's list.
  - `Workspace` item with no workspace id -> hidden; with one -> shown.
  - `Agent` item with a workspace id but no agent id -> hidden.
  - `visible` returns false -> hidden.
  - Query "diff" matches item A by title and item B only by keyword "diff" -> A ranks above B; the keyword is not drawn.
  - Two plugins -> two sections in index order, before "Actions".
  - `ContributionsChanged` while open with query "ed" -> the query stays and the rows rebuild.
  - A plugin row run -> `on_select` gets the context ids, then `Run { id: "plugin:<plugin>:<item>" }` is emitted once.

Anti-goal: keystroke search stays fast: `search_500_items_stays_under_16_ms_per_keystroke` passes unchanged, and a new test with 500 items plus 100 keyword items stays under 16 ms per keystroke; tripwire; read before the first change, after keyword search lands, and at completion.

## Verify

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-command` -> pass, including the existing search and shortcut tests unchanged and new pure tests for each expected case: plugin rows first, the context gate, `visible`, keyword ranking below title and group matches, section order across two plugins, the run order (handler, then one `Run` with the `plugin:` id), the rebuild that keeps the query, and the 600-item keystroke timing test
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-command` -> exit 0
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> exit 0 with no edit outside this group's files
- No UI check here: no plugin crate is registered before TASK-191, so the Command Center looks as today. Plugin rows get their screen check after TASK-191.
