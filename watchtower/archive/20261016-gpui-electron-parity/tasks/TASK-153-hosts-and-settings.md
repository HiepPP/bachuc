# TASK-153 Hosts and settings

Group: I (writes the new `crates/settings`)
Class: code

## Brief

Goal: Add the crate `bachuc-settings` with the Electron settings screen for the sections in this plan: hosts, appearance, keyboard shortcuts, providers, and project settings.

Change: hosts only from a seeded file and no settings -> an Electron-style settings view that edits hosts, theme and font sizes, shortcuts, and project config, and shows providers.

Design: [packages/app/src/screens/settings-screen.tsx](packages/app/src/screens/settings-screen.tsx), [packages/app/src/screens/settings/host-page.tsx](packages/app/src/screens/settings/host-page.tsx), [packages/app/src/components/add-host-modal.tsx](packages/app/src/components/add-host-modal.tsx), [packages/app/src/screens/settings/appearance/appearance-section.tsx](packages/app/src/screens/settings/appearance/appearance-section.tsx), [packages/app/src/screens/settings/keyboard-shortcuts-section.tsx](packages/app/src/screens/settings/keyboard-shortcuts-section.tsx), [packages/app/src/screens/settings/providers-section.tsx](packages/app/src/screens/settings/providers-section.tsx), [packages/app/src/screens/project-settings-screen.tsx](packages/app/src/screens/project-settings-screen.tsx)

Boundaries:

- Owner answer (2026-10-16): sections are hosts, appearance, keyboard shortcuts, providers, and project settings.
- Layout: `SETTINGS_SIDEBAR_WIDTH` 320 and `SETTINGS_DETAIL_MIN_WIDTH` 400 ([packages/app/src/constants/layout.ts](packages/app/src/constants/layout.ts) lines 20-21), `SETTINGS_CONTENT_WIDTH` 720 ([packages/app/src/screens/settings-screen.tsx](packages/app/src/screens/settings-screen.tsx) line 1550). Rows follow [docs/design.md](docs/design.md) density and border rules.
- Hosts: add, edit, and remove `ws://` hosts with a name, URL, and optional password ([ADR-0040](watchtower/decisions/ADR-0040-one-host-plain-ws-over-merged-hosts-and-relay.md)); save with `save_hosts`.
- Appearance: theme `Light`, `Claude dark`, or `System` ([ADR-0043](watchtower/decisions/ADR-0043-electron-tokens-and-lucide-icons-over-gpui-kit-defaults.md)), and the interface and code font sizes. Persist in `~/Library/Application Support/BachucGpui/appearance.json` and apply at once.
- Shortcuts: list, rebind by recording a chord, clear to unassigned, and reset; save with `save_overrides`.
- Providers: read-only availability and models from `providers()`, as the Electron section shows them.
- Project settings: read and write the project config with `read_project_config` and `write_project_config`, with the fields the Electron screen edits. Writing goes to the live daemon only in checks.
- No daemon settings, plugins page, or updates (ADR-0038, ADR-0041).

How:

- Build the settings view with a section list and detail panes.
- Add the host form, appearance controls, shortcut recorder, providers list, and project config form.
- Add `examples/settings.rs`.

Files:

- [packages/desktop-gpui/crates/settings](packages/desktop-gpui/crates/settings) (new crate `bachuc-settings`: `Cargo.toml`, `src`, `tests`, `examples/settings.rs`)

Interfaces:

- Consumes: `bachuc_theme::ThemeChoice` (enum `Light`, `ClaudeDark`, `System`), `bachuc_theme::set_theme(choice: ThemeChoice, cx: &mut gpui_kit::App)`, and `bachuc_theme::set_font_scale(interface: f32, code: f32, cx: &mut gpui_kit::App)` from TASK-145
- Consumes: `bachuc_theme::tokens(cx: &gpui_kit::App) -> Tokens` from TASK-145
- Consumes: `bachuc_theme::metrics` (module of `pub const` `f32` values: `SPACE_0_5`, `SPACE_1`, `SPACE_1_5`, `SPACE_2`, `SPACE_3`, `SPACE_4`, `SPACE_6`, `SPACE_8`, `RADIUS_SM`, `RADIUS_BASE`, `RADIUS_MD`, `RADIUS_LG`, `RADIUS_XL`, `RADIUS_2XL`, `FONT_CODE`, `FONT_SM`, `FONT_BASE`, `FONT_CONTENT`, `FONT_LG`, `FONT_XL`, `FONT_2XL`, `ICON_XS`, `ICON_SM`, `ICON_MD`, `ICON_LG`, `CONTROL_XS`, `CONTROL_SM`, `CONTROL_MD`, `HEADER_HEIGHT`, `TAB_BAR_HEIGHT`, `MAX_CONTENT_WIDTH`, `SIDEBAR_WIDTH`, `SIDEBAR_MIN_WIDTH`, `SIDEBAR_MAX_WIDTH`, `TRAFFIC_LIGHT_WIDTH`, `TRAFFIC_LIGHT_HEIGHT`, `SETTINGS_SIDEBAR_WIDTH`, `SETTINGS_DETAIL_MIN_WIDTH`, `SETTINGS_CONTENT_WIDTH`) from TASK-145
- Consumes: `bachuc_icons::IconName` (enum with one variant per generated icon, named as the lucide import, for example `FolderPlus`, `GitBranch`, `X`), `bachuc_icons::icon(name: IconName) -> gpui_kit::component::Icon` (sized with `ICON_MD`; change it with `with_size` and `text_color`), and `bachuc_icons::Assets` (a GPUI `AssetSource` that serves the SVGs and falls through to the gpui-kit assets) from TASK-145
- Consumes: `bachuc_client::save_hosts(hosts: &[HostConfig]) -> anyhow::Result<()>` from TASK-146
- Consumes: `Client::read_project_config(&mut self, root: &str, cx: &mut gpui_kit::Context<Client>) -> gpui_kit::Task<anyhow::Result<serde_json::Value>>` and `Client::write_project_config(&mut self, root: &str, config: serde_json::Value, cx: &mut gpui_kit::Context<Client>) -> gpui_kit::Task<anyhow::Result<()>>` from TASK-146
- Consumes: `Client::projects(&self) -> &[ProjectNode]` with `ProjectNode { id: String, name: String, root: String, workspaces: Vec<WorkspaceNode> }` and `WorkspaceNode { id: String, name: String, cwd: String, branch: Option<String>, pinned: bool, agent_ids: Vec<String> }` from TASK-146
- Consumes: `bachuc_command::Shortcut { action: String, label: String, chord: Option<String> }`, `bachuc_command::default_shortcuts() -> Vec<Shortcut>`, `bachuc_command::load_shortcuts() -> Vec<Shortcut>` (defaults plus overrides), and `bachuc_command::save_overrides(shortcuts: &[Shortcut]) -> anyhow::Result<()>` from TASK-152
- Produces: `bachuc_settings::Settings` (a GPUI view) with `Settings::new(client: gpui_kit::Entity<bachuc_client::Client>, window: &mut gpui_kit::Window, cx: &mut gpui_kit::Context<Settings>) -> Settings` for TASK-155
- Produces: `bachuc_settings::SettingsEvent` (enum `HostsChanged`, `ShortcutsChanged`, `Closed`) emitted by `Settings` for TASK-155
- Produces: `bachuc_settings::load_appearance(cx: &mut gpui_kit::App)` (reads `appearance.json` and applies it) for TASK-155

Expected result:

- Each section matches the reference app's settings page.
- A host added in settings appears in `hosts.json`; a theme change applies at once and survives a restart.
- Cases:
  - Removing the connected host asks for a confirm.
  - A shortcut chord that clashes with another action shows the Electron conflict state.
  - A project with no config shows the Electron empty form.

Anti-goal: settings changes never write `~/.bachuc` or `.dev/paseo-home` files directly; tripwire; read with `git status --short` and a `find ~/.bachuc .dev/paseo-home -newer <start marker> -name '*.json'` check at completion (only daemon-written files may change).

## Verify

- `packages/desktop-gpui/cargo.sh test -p bachuc-settings` -> pass, including host validation, appearance persistence, the shortcut conflict and clear, and project config round-trip against a fake server
- Agent check: bundle `packages/desktop-gpui/bundle.sh example settings BachucSettings io.github.hieppp.bachuc-settings`; compare each section with the reference app on 6770 using `cua_repl` screenshots -> the parity list in the outcome shows match or records each mismatch
- Agent check: add a host, switch the theme to Claude dark, rebind one shortcut, then relaunch -> all three changes persist; restore `hosts.json`, `appearance.json`, and `shortcuts.json` after the check
- Agent check: remove the connected host and confirm -> the host leaves `hosts.json` and the view shows the disconnected state; restore `hosts.json`
- Agent check: `cua_repl` `getAXState` -> section rows, inputs, switches, and buttons have roles, labels, and selected state
- Anti-goal: the `find` check above -> no file written by the settings view under `~/.bachuc` or `.dev/paseo-home`
