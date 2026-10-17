# TASK-177 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/settings/Cargo.toml`: adds `bachuc-plugin-host`.
- `packages/desktop-gpui/Cargo.lock` (generated): the new dependency edge. The file was already modified before this run.
- `packages/desktop-gpui/crates/settings/src/plugins.rs` (new): the pure rules and `watch_plugin_themes`, with 12 unit tests.
- `packages/desktop-gpui/crates/settings/src/settings.rs`: `Page` (section or plugin screen) replaces the private `section` field. Adds the "Plugins" label with one row per screen, the plugin page, `Settings::open_plugin_screen`, and the host and client subscriptions.
- `packages/desktop-gpui/crates/settings/src/appearance.rs`: `Appearance::plugin_theme`, `pluginThemeId` load and save, `with_builtin_theme`, `with_plugin_theme`, and `apply` calls `set_custom_theme`. 4 new tests.
- `packages/desktop-gpui/crates/settings/src/pane_appearance.rs`: the theme picker lists plugin themes after a separator; the trigger shows the plugin theme name and swatch; picking calls `persist`.
- `packages/desktop-gpui/crates/settings/src/ui.rs`: `page_title` takes `impl Into<SharedString>`, plus `select_trigger_with` and `swatch`.
- `packages/desktop-gpui/crates/settings/src/lib.rs`: `mod plugins`, `pub use plugins::watch_plugin_themes`, doc.

## Contract

- Produces: `Settings::open_plugin_screen(&mut self, plugin_id: &str, settings_id: &str, window: &mut Window, cx: &mut Context<Settings>)` and `bachuc_settings::watch_plugin_themes(cx: &mut App)`.
- Kept: `Settings::new`, `Settings::open_project`, `SettingsEvent`, and `load_appearance` keep their signatures. `Section` stays private.
- Sidebar: with an active plugin screen, a "Plugins" heading and one row per screen (icon, title, index order) show below the five pages. With none, the label and rows are absent.
- Plugin page: the header is `<plugin id> · <screen title>` (`Settings` when the screen is gone). The body is the screen's view, built once per (plugin id, screen id) and rebuilt when the plugin contributes again (`Rc::ptr_eq` on the factory). Views of screens that left the index are dropped on `ContributionsChanged`.
- States, each a muted `note`, offline first: "Connect to this host to open plugin settings." then "This plugin settings screen is unavailable." An open page keeps its place when its plugin goes inactive.
- Redraw: `ContributionsChanged`, `PluginStateChanged`, and the client `Connected` and `Disconnected` events call `cx.notify()`. Other client events do not.
- `open_plugin_screen` always selects the page. An id the index lacks shows the unavailable text.
- Themes:
  - `watch_plugin_themes` maps each `ThemeContribution` to a `CustomTheme` with id `<plugin_id>/theme/<id>` and the scheme from `appearance`. It calls `set_custom_themes` at once and on each `ContributionsChanged`. With no host it does nothing.
  - The picker reads the same mapping from the host index at draw time, so it does not depend on the order of the two subscribers.
  - Picking a plugin theme keeps the stored `ThemeChoice` and sets `pluginThemeId`. Picking a built-in sets `theme` and removes `pluginThemeId`.
  - A stored id with no matching theme stays in the file and checks the stored built-in choice.
- Report for the lead: the app's `theme.cycle` (`crates/app/src/shell.rs`, `run_action`) loads `appearance.json`, changes only `theme`, saves, and calls `apply`. `plugin_theme` stays set, so while a plugin theme is selected the cycle has no visible effect until the picker picks a built-in. `bachuc_theme::cycle_builtin_theme` (TASK-180) clears the selection but the app does not call it. The app is not this group's file.
- TASK-191 must call `bachuc_settings::watch_plugin_themes(cx)` after `bachuc_plugin_host::init` and, on `PluginHostEvent::OpenSettings`, call `Settings::open_plugin_screen`.

## Verified

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-settings` -> exit 0. 49 unit tests passed (16 new), 4 `project_config` tests passed, 0 failed. Log: `/tmp/wt-g177-test1.txt`.
  - New pure tests: nav rows with and without screens and their order; page state offline, unavailable (unknown screen id, another plugin's id), ready; the inactive-then-back case; the header; view pruning; contribution to `CustomTheme` id, scheme, and colors; picker order with a separator and without one; the checked item with a selected, a missing, and no plugin theme; the trigger choice; `pluginThemeId` load, keep on another save, and clear on a built-in pick.
  - `open_plugin_screen` itself needs a `Window` and `Context`. Its known and unknown id behavior is covered through `page_state` (the page it selects is a plain `Page::Plugin(ScreenId)`). The method is checked by reading only. UNVERIFIED (unattended): code tests only.
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-settings` -> exit 0.
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> exit 0 (`/tmp/wt-g177-build2.txt`). The first attempt failed only inside `bachuc-composer`, group C's crate (unresolved `crate::draft`, `bachuc_plugin_host`, missing `model_groups`); the retry after about 4 minutes passed. No edit outside `crates/settings`, apart from the generated `Cargo.lock`.
- GitNexus impact: not run per symbol. `rg` shows `Appearance`, `page_title`, and `select_trigger` are used only inside `crates/settings` (the `ui` module is private; `Appearance` struct literals are only in this crate). The app reads `Appearance` fields by name and uses `load_from`, `save_to`, `apply`.
- UI check: Plugin screens and themes get their screen check after TASK-191 -> DEFERRED (UI checker). No plugin crate is registered before TASK-191, so Settings looks as today; this is UNVERIFIED (unattended): code tests only.

## Anti-goal

- Checkpoint 0, before the first change: `saved_appearance_survives_a_reload` and `every_theme_round_trips` read in `appearance.rs`; both use `..Appearance::default()` and load from a file; source: file read; 2026-10-16.
- Checkpoint 1, at completion: both pass unchanged (`test appearance::tests::saved_appearance_survives_a_reload ... ok`, `test appearance::tests::every_theme_round_trips ... ok`); source `/tmp/wt-g177-test1.txt`; 2026-10-16.
- A file written before this TASK loads to the same values: `a_file_without_a_plugin_theme_loads_as_before` and `an_old_file_without_the_new_keys_loads_the_defaults` pass; a save of such a file writes no `pluginThemeId`.
- Tripwire: not tripped.

## Lessons

- A theme picker that needs the plugin theme list reads it from the host index at draw time and subscribes to `ContributionsChanged`. Reading `bachuc_theme::custom_themes` would depend on which subscriber of the same event runs first.
- A GPUI view crate can still test its menu and page rules: return plain enums (`ThemeItem`, `PageState`) from pure functions and draw them in the view.
