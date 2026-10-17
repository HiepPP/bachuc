# TASK-191 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/app/Cargo.toml`: 15 path dependencies `bachuc-plugin-<id>` (`../../plugins/<id>`), no `features` key.
- `packages/desktop-gpui/crates/app/src/plugins.rs`: private table `PLUGINS` (id and `plugin()` per crate); `registered_plugins()` returns the 15 plugins in plugin id order; 2 new tests replace `no_plugin_is_registered_yet`.
- `packages/desktop-gpui/crates/app/src/shell.rs`: only the call sites below.
- `packages/desktop-gpui/Cargo.lock` (generated): the 15 dependency edges of `bachuc-app`.

## Contract

- `registered_plugins() -> Vec<Box<dyn NativePlugin>>` returns, in order: `assistant-cite`, `board`, `claude-look`, `edit-diffs`, `idle-runtime-closer`, `jev-orchestrator`, `next-prompt-actions`, `plugins-nav`, `prompt-translate`, `skill-pins`, `stop-orb`, `thread-branch`, `thread-context-attach`, `watchtower-board`, `workspace-spaces`. `main.rs` already passes it to `bachuc_plugin_host::init`.
- App-side calls wired in `shell.rs` (the "app calls" of TASK-175 to TASK-178):
  - TASK-176: `CommandCenter::set_plugin_context(workspace, agent, cx)` right after `open_command_center` creates the center, and again from `sync_plugin_active` while the center is open. The ids are `Shell::plugin_active`, the same pair given to `PluginHost::set_active`.
  - TASK-177: `bachuc_settings::watch_plugin_themes(cx)` in `Shell::new`, right after the shell finds the host that `main` created with `init`. `PluginHostEvent::OpenSettings { plugin_id, settings_id }` opens Settings, then calls `Settings::open_plugin_screen`. `OpenPluginsPage` still opens Settings on its current page (Settings has no plugins page).
  - TASK-178: `PluginHostEvent::RevealTimelinePassage` opens the agent tab, then calls `TimelineView::reveal_passage(message_id, text, window, cx)` on that agent's timeline (`Shell::agent_timeline`).
  - TASK-175: the shell subscribes to `SidebarPluginEvent`. `OpenSurface` opens the plugin surface as a `Page` with no params. `ProjectAdded` needs no app work (the project list follows the daemon) and has an empty arm. `Shell::set_surface` calls `Sidebar::set_active_surface` with the open `Page` surface (an overlay gives `None`), on open, close, and host switch.
  - TASK-184 names no app call (outcome: "TASK-191 registers the crate"). The plugin crates name none either (`rg "the app calls|TASK-191"` in the TASK-181 to TASK-194 outcomes finds only "registered by TASK-191").
- Safe defaults taken:
  - A sidebar nav row without an action opens its surface as a page, the Electron route behavior; `SidebarItem` carries no presentation.
  - The reveal runs after `open_agent`; a lazy tab is made by `activate_key`, so its timeline exists. If the tab has no timeline yet, no reveal runs.
  - `ProjectAdded` triggers nothing (no spec line gives it an action).

## Verified

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-app` -> exit 0, 57 passed, 0 failed. New: `the_registered_ids_are_the_fifteen_in_catalog_order` (ids equal the 15 in order, sorted, unique) and `every_plugin_id_is_its_crate_id_and_has_a_server_folder` (each `plugin().id()` equals the table id, the folder `hiep-plugins/plugins/<id>` exists, and the crate manifest is named `bachuc-plugin-<id>`). Log `/tmp/t191-test.txt`.
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-app` -> exit 0.
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> exit 0. One warning, `next_theme` never used in `state.rs` (pre-existing, TASK-174 lesson). Log `/tmp/t191-build2.txt`. The first build compiled all 15 plugin crates with no error.
- `packages/desktop-gpui/cargo.sh tree -p bachuc-app -e normal --depth 1 | rg -c "bachuc-plugin-"` -> 16.
- `rg -n "bachuc-(composer|sidebar|timeline|command|settings|workspace|explorer|app)[ .=]" packages/desktop-gpui/plugins/*/Cargo.toml` -> no match (exit 1).
- `ls packages/desktop-gpui/target/debug/deps/libgpui_kit-*.rlib | wc -l` -> 7, the same as before the first dependency was added.
- UI check (prompt-translate, skill-pins, and watchtower-board pills on one agent composer, GPUI against Electron 6770) -> DEFERRED (UI checker).
- The wiring (command center context, Settings plugin screen, reveal, sidebar surface row) needs a window: UNVERIFIED (unattended): code tests only. Live daemon 6768 is down.
- GitNexus: no exported symbol edited; `Shell` methods are crate-private additions, `open_command_center` and `sync_plugin_active` changed only by added lines (checked by `rg`).

## Anti-goal

- Checkpoint 0, before the first dependency was added: 7 `libgpui_kit-*.rlib`; source `ls packages/desktop-gpui/target/debug/deps`; 2026-10-17 02:18.
- Checkpoint 1, at completion, after the build and the test run: 7; same source; 2026-10-17 02:20.
- Tripwire: not tripped.

## UI Checks

Checked 2026-10-17 by the UI checker with `open -g`, AX reads, AX presses, and window screenshots (no pointer, no foreground). Live runs on 6768 with `prompt-translate`, `skill-pins`, and `watchtower-board` `running`. Electron 6770 shows all three as `failed`, so it draws no pill.

- UI check (prompt-translate, skill-pins, and watchtower-board pills on one agent composer, GPUI against Electron 6770) -> UNVERIFIED (unattended): needs pointer. No agent composer can open (AX press on sidebar rows does nothing: `/tmp/wt-ui/gpui-t144.jpg`). Seen instead: the GPUI New workspace draft composer draws all three pills, `Default` (Caveman mode), `Skills`, and `Watchtower`, plus a `main` branch pill (`/tmp/wt-ui/gpui-sidepanel2.jpg`). The reference shows none of them, so no side-by-side match exists.

## Lessons

- The plugin crates share the app's one gpui-kit feature set, so linking 15 more crates adds no gpui-kit build.
- `Option<(Entity<T>, Subscription)>` is not `Clone`; copy the entity with `.as_ref().map(|(entity, _)| entity.clone())`.
- A plugin crate's lib name is its package name with `_`, for example `bachuc_plugin_edit_diffs`.

## UI Checks, round 2

Checked 2026-10-17 by the UI checker with `open -g`, AX reads, AX presses, and window screenshots (no pointer, no foreground). Electron 6770 shows all three plugins as `failed`, so it draws none of their pills.

- UI check (prompt-translate, skill-pins, and watchtower-board pills on one agent composer, GPUI against Electron 6770) -> PASS. The GPUI agent composer of `wt-long-thread` shows `Default` (Caveman mode, prompt-translate), `Skills` (skill-pins), and `Watchtower` (watchtower-board, in the track row above the input) (`/tmp/wt-ui2/after-press-ws31.jpg`). AX press on `Skills` and on `Caveman mode` opened their menus, and AX press on `Watchtower` opened the Watchtower overlay (`/tmp/wt-ui2/watchtower-overlay.jpg`). The reference shows none, so there is no side-by-side match, and the line's "where Electron shows them" holds vacuously.
