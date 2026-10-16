# TASK-174 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/app/Cargo.toml`: `bachuc-plugin-host`.
- `packages/desktop-gpui/Cargo.lock` (generated): `bachuc-plugin-host` on `bachuc-app`.
- `packages/desktop-gpui/crates/app/src/plugins.rs` (new): `registered_plugins` (empty) and the pure rules `panel_location`, `panel_identity`, `surface_heading`, `keystroke_text`, `host_takes`, `plugin_shortcut`, `subtitle_input`, `side_panel_available`, `side_panel_index`, with 9 tests.
- `packages/desktop-gpui/crates/app/src/plugin_views.rs` (new): `OpenSurface`, the `PluginViews` cache (surfaces by plugin id, surface id, and params; the header subtitle by input; rebuilt only when the factory `Rc` changes; pruned on `ContributionsChanged`), `surface_header`, `surface_body`, `surface_notice`, `surface_page`, `surface_overlay`, with 1 test.
- `packages/desktop-gpui/crates/app/src/main.rs`: `bachuc_plugin_host::init(plugins::registered_plugins(), cx)` after `bachuc_theme::init`.
- `packages/desktop-gpui/crates/app/src/shell.rs`: `set_client` at start and on each host switch; `set_active` on a workspace or agent tab change; `on_plugin_event` routes; `open_plugin_panel`; the surface page and overlay; `on_key_capture` (Escape and plugin shortcuts); the three tab menu actions with the pure `rename_target`, `resume_text`, and `reload_error` (3 tests); `theme.cycle` calls `cycle_builtin_theme`; the side panel session choice; `PaneContent::Plugin`.
- `packages/desktop-gpui/crates/app/src/chrome.rs`: the header subtitle and the New workspace `PanelRight` toggle.
- `packages/desktop-gpui/crates/app/src/panes.rs`: `CornerPills` over the timeline's top-right corner.
- `packages/desktop-gpui/crates/app/src/new_workspace.rs`: the 360 px side panel (left border, surface0), `side_panel_available`, `side_panel_open`, `set_side_panel_open`, `SidePanelChanged`, and `ShowNewWorkspacePanel`.
- `packages/desktop-gpui/crates/app/src/actions.rs`: the focus-role test also reads `plugin_views.rs` and `plugins.rs` (actions.rs is in this group's TASK-173 Files).

## Contract

- Produces `plugins::registered_plugins() -> Vec<Box<dyn bachuc_plugin_host::NativePlugin>>`, empty, passed to `init` in `main.rs`, and the app hosts: workspace panels as tabs, surface pages and overlays, New workspace side panels, plugin shortcuts, the header subtitle, corner pills, and the routes for `OpenNewWorkspace`, `OpenAgent` (and `OpenAgentAt`), `OpenWorkspace`, `OpenSettings`, and `OpenPluginsPage`.
- Tab menu: Copy resume command -> "Resume ID not available", "Resume command not available", or the command with "Resume command copied". Reload agent -> "Reloading agent...", `refresh_agent`, then "Reloaded agent" or the error text ("Failed to reload agent" when empty). Rename on a terminal -> the "Rename terminal" dialog; `Ok` retitles the tab, `Err` shows its text; a terminal not open yet keeps "The terminal is not open yet"; a file, diff, or new tab shows "This tab cannot be renamed".
- Surfaces: `Page` replaces the main column (the sidebar stays); `Overlay` is the dimmed backdrop (press closes) and the centered card. Escape and the close button close either one. A surface the index lacks shows "This plugin surface is unavailable." in statusDanger. The heading is the surface title, else its id, else the plugin id.
- Panels: requested location, else `Workspace` when listed, else the first; an unsupported request shows an error notice and opens no tab; identity `plugin:<plugin>/<panel>` plus `/<agent>` for an `Agent` panel; an open tab is focused. Planner call: `Explorer` also opens a workspace tab, because the explorer crate has no plugin slot.
- Shortcuts: a root `capture_key_down`. GPUI dispatches key bindings before key listeners, and the pure rule also drops any key that the first combo of an assigned host chord uses, whatever its context. `Mod` is Cmd. The first plugin with the combo wins; with its surface open, the key closes it; else `on_press` gets the active workspace id.
- Header subtitle: the first contribution replaces the project name when the workspace has a non-empty project id.
- Side panel: the toggle shows with a chosen project and a panel; it starts closed, and the choice lasts for the session across screens. It shows the panel that `ShowNewWorkspacePanel` named, else the first.
- With no plugin registered, the index is empty: no plugin view is built, no shortcut runs, and the header and screens look as before. `TimelineTransformsChanged`, `SidebarFilterChanged`, `SetComposerText`, and `AddComposerAttachment` redraw nothing in the shell.

## Verified

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-app` -> exit 0; 56 passed, 0 failed (`/tmp/wt-d-test2.txt`). New TASK-174 tests: no plugin registered; panel location (request, Workspace, first, empty list, an Explorer request on a Workspace-only panel -> the error); agent panel identity and the same identity twice (one tab); surface heading order; `Mod` -> Cmd; host `Cmd+K` beats a plugin `Mod+K`; an unassigned host `Cmd+Shift+G` lets a plugin `Mod+Shift+G` fire; first plugin wins; an empty chord takes no key; the anti-goal test; the subtitle with and without a project id; side panel availability and choice; the unavailable surface text; the rename targets; the resume text cases; the reload error text.
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-app` -> exit 0.
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> exit 0 (`/tmp/wt-d-build5.txt`).
- `rg -n "not available yet" packages/desktop-gpui/crates/app/src` -> no match (exit 1).
- The fake-plugin flows (overlay shows, Escape closes, the shortcut opens then closes), the rename dialog, the reload notices, and the "app looks as before" check need a window: UNVERIFIED (unattended): code tests only. Live daemon down: no live check.
- GitNexus `impact on_tab_menu` -> UNKNOWN, no callers resolved; `rg` shows it is called only from the `TabMenuEvent` subscription in `shell.rs`.

## Anti-goal

- Checkpoint 1, when the shortcut rule landed: `no_default_host_chord_ever_reaches_a_plugin` passed for every default chord of `bachuc_command`, including the nine `Cmd+Alt+Digit` keys, `Shift+Tab`, and `Escape`; source `/tmp/wt-d-test1.txt`; 2026-10-16 23:26.
- Checkpoint 2, at completion: the same test passed; source `/tmp/wt-d-test2.txt`; 2026-10-16 23:39.
- Tripwire: not tripped.

## Lessons

- `Client::view_agent` does nothing for an agent that is already viewed, so Reload agent calls `unview_agent` then `view_agent` to start the sync over. The timeline view compares `TimelineState::version`, which restarts at 0, so a stale frame is possible until the next timeline change (UNVERIFIED (unattended): code tests only).
- `theme.cycle` now calls `bachuc_theme::cycle_builtin_theme` and saves `appearance.json` through `with_builtin_theme`, which drops `pluginThemeId`. `state::next_theme` is now unused outside its test, and the build warns about it; `state.rs` is outside this TASK's Files.
- Planner calls kept as specified: `OpenSettings` and `OpenPluginsPage` open Settings on its current page, although TASK-177 now has `Settings::open_plugin_screen`; TASK-191 can route `OpenSettings` to it.
