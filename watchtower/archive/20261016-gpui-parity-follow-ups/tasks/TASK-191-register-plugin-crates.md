# TASK-191 Register every plugin crate in the app

Group: D (writes `crates/app/Cargo.toml` and `crates/app/src/plugins.rs`; runs after TASK-174 in the same group; `Cargo.lock` is generated)
Class: code

## Brief

Goal: Make the app depend on the 15 plugin crates of TASK-181 to TASK-190 and pass them to `bachuc_plugin_host::init`, so each plugin shows when the active host lists it as enabled and running.

Change: `plugins::registered_plugins()` returns an empty list -> it returns the 15 plugin crates in plugin id order.

Design: the daemon catalog lists plugins sorted by id ([packages/server/src/server/plugins/runtime.ts](packages/server/src/server/plugins/runtime.ts) line 359), and the Electron registry keeps that order. The order decides every "first wins" rule of TASK-170 (stop button, renderer, header subtitle).

Boundaries:

- Follows [ADR-0045](watchtower/decisions/ADR-0045-native-rust-plugin-clients-over-webview-panes-or-shared-logic.md): one crate per plugin in `packages/desktop-gpui/plugins/<id>`, named `bachuc-plugin-<id>`. The app needs only `pub fn plugin()` from each.
- The 15 ids, in this order: `assistant-cite`, `board`, `claude-look`, `edit-diffs`, `idle-runtime-closer`, `jev-orchestrator`, `next-prompt-actions`, `plugins-nav`, `prompt-translate`, `skill-pins`, `stop-orb`, `thread-branch`, `thread-context-attach`, `watchtower-board`, `workspace-spaces`. `html-reply` is out of scope. `jev-evaluator`, `jev-permission-gate`, `thread-janitor`, and `pr-watch` have no client part (CONTEXT "Plugin clients").
- Add each crate as a path dependency, for example `bachuc-plugin-board = { path = "../../plugins/board" }`. Add no `features` key to any dependency; one more feature set makes a second GPUI build (CLAUDE.md).
- Change no plugin crate and no other app file, except one call site: the shell calls `CommandCenter::set_plugin_context` (TASK-176) with the active workspace and agent ids whenever they change (lead, from the spec check). If a plugin crate fails to build, stop and report which one; do not patch it from this TASK.
- Also wire, in the same call site, every app-side call that TASK-175 to TASK-178 and TASK-184 name as "the app calls"; list each one in the outcome.
- From TASK-177 (lead): call `bachuc_settings::watch_plugin_themes(cx)` after `bachuc_plugin_host::init`, and call `Settings::open_plugin_screen` on `PluginHostEvent::OpenSettings` (read [TASK-177-outcome.md](watchtower/tasks/TASK-177-outcome.md)). From TASK-178: read [TASK-178-outcome.md](watchtower/tasks/TASK-178-outcome.md) for `TimelineView::reveal_passage`. From TASK-175: subscribe to `SidebarPluginEvent` (`OpenSurface`, `ProjectAdded`) and call `Sidebar::set_active_surface` when the active plugin surface changes; read [TASK-175-outcome.md](watchtower/tasks/TASK-175-outcome.md).
- A test checks the list against the plugin source: every registered id has a folder in `hiep-plugins/plugins`, and each `plugin().id()` equals the id of its crate name.

Files:

- [packages/desktop-gpui/crates/app/Cargo.toml](packages/desktop-gpui/crates/app/Cargo.toml) (15 path dependencies)
- [packages/desktop-gpui/crates/app/src/plugins.rs](packages/desktop-gpui/crates/app/src/plugins.rs) (the list and its tests)
- [packages/desktop-gpui/crates/app/src/shell.rs](packages/desktop-gpui/crates/app/src/shell.rs) (only the `set_plugin_context` call site and the calls the Boundaries name)
- [packages/desktop-gpui/Cargo.lock](packages/desktop-gpui/Cargo.lock) (generated)

Interfaces:

- Consumes: `bachuc_plugin_host::NativePlugin` (object-safe trait: `fn id(&self) -> &'static str`, `fn contribute(&self, ctx: PluginContext, contributions: &mut Contributions, cx: &mut App)`), `bachuc_plugin_host::Contributions` (`Default`; methods `add_composer_pill(ComposerPill)`, `add_composer_interceptor(ComposerInterceptor)`, `add_stop_button(StopButton)`, `add_attachment_source(AttachmentSource)`, `add_sidebar_item(SidebarItem)`, `add_sidebar_section(SidebarSection)`, `add_project_filter(ProjectFilter)`, `add_project_menu(ProjectMenu)`, `add_command_item(CommandCenterItem)`, `add_slash_command(PluginSlashCommand)`, `add_settings_screen(SettingsScreen)`, `add_surface(Surface)`, `add_workspace_panel(WorkspacePanel)`, `add_new_workspace_panel(NewWorkspacePanel)`, `add_shortcut(Shortcut)`, `add_header_subtitle(HeaderSubtitle)`, `add_timeline_transformer(TimelineTransformer)`, `add_timeline_renderer(TimelineRenderer)`, `add_selection_action(SelectionAction)`, `add_theme(ThemeContribution)`), the type aliases `Handler<I>`, `ViewFactory<I>`, and `PopoverClose`, and the crate rule that each plugin crate exports `pub fn plugin() -> Box<dyn bachuc_plugin_host::NativePlugin>` from TASK-170
- Consumes: `pub fn plugin() -> Box<dyn bachuc_plugin_host::NativePlugin>` from each crate `bachuc-plugin-<id>` of TASK-181 to TASK-190
- Consumes (crate-internal): `plugins::registered_plugins() -> Vec<Box<dyn bachuc_plugin_host::NativePlugin>>` in [packages/desktop-gpui/crates/app/src/plugins.rs](packages/desktop-gpui/crates/app/src/plugins.rs), empty here, and passed to `bachuc_plugin_host::init` in `main.rs` from TASK-174
- Produces: the app build that holds every plugin client, for TASK-192

Expected result:

- `registered_plugins()` returns 15 plugins whose ids are the list above, in that order.
- On a host that lists a plugin enabled and running, its pills, rows, and pages show; on a host that does not, nothing of it shows.

Anti-goal: the plugin crates add no second GPUI build: the count of `libgpui_kit-*.rlib` files in `packages/desktop-gpui/target/debug/deps` stays the same before and after this TASK's `cargo.sh build -p bachuc-app`; tripwire; read before the first dependency is added and at completion.

## Verify

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-app` -> pass, including every existing test plus tests that the registered ids equal the 15 ids in order, are unique, each match a folder in `hiep-plugins/plugins`, and each equal the crate id of their `plugin()`
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-app` -> exit 0
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> exit 0
- `packages/desktop-gpui/cargo.sh tree -p bachuc-app -e normal --depth 1 | rg -c "bachuc-plugin-"` -> 16 (the 15 plugin crates and `bachuc-plugin-host`)
- `rg -n "bachuc-(composer|sidebar|timeline|command|settings|workspace|explorer|app)[ .=]" packages/desktop-gpui/plugins/*/Cargo.toml` -> no match
- `ls packages/desktop-gpui/target/debug/deps/libgpui_kit-*.rlib | wc -l` -> the same count as before the first dependency was added
- UI check: only when live already runs on 6768 with these plugins installed; open the GPUI build with the live host selected and the Electron 6770 build, both with `open -g`, never in front and with no pointer; take background screenshots of one agent composer -> the prompt-translate, skill-pins, and watchtower-board pills show in GPUI where Electron shows them; when live does not run, or the frame is stale or blank (Q-069), the check is UNVERIFIED
