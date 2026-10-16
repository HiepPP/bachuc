# TASK-153 Outcome

## Status

DONE for code checks. Screen checks are DEFERRED (UI checker).

## Changed

- New crate `packages/desktop-gpui/crates/settings` (`bachuc-settings`): `src/{lib,settings,ui,hosts,appearance,bindings,config,pane_hosts,pane_appearance,pane_shortcuts,pane_providers,pane_projects}.rs`, `tests/project_config.rs`, `examples/settings.rs`.
- Restart: a first builder died on the 2026-10-16 out-of-memory reboot after writing `src`. This builder read all of it, fixed 12 compile errors (missing `Sizable`, `Disableable`, `FluentBuilder` imports; `role` needs an `.id`), removed an unused method, and added the tests and the example.

## Contract

- `bachuc_settings::Settings::new(client: Entity<Client>, window: &mut Window, cx: &mut Context<Settings>) -> Settings` (GPUI view). Extra: `Settings::open_project(root, name, window, cx)`.
- `bachuc_settings::SettingsEvent { HostsChanged, ShortcutsChanged, Closed }`.
- `bachuc_settings::load_appearance(cx: &mut gpui_kit::App)`: reads `~/Library/Application Support/BachucGpui/appearance.json` and applies theme and font scale.
- Public pure models: `hosts::validate_host`, `appearance::{Appearance, load_from, save_to, apply}`, `bindings::*`, `config::{config_to_draft, apply_draft, ConfigDraft}`.
- Sections: Hosts, Appearance, Shortcuts, Providers, Projects (the list, then the `paseo.json` editor of one project).
- Storage: hosts through `bachuc_client::save_hosts`, shortcuts through `bachuc_command::save_overrides_to` (the `dir` form of `save_overrides`, used so tests stay in a temp dir), appearance in `appearance.json`.

## Verified

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-settings` -> PASS: 26 unit tests (host validation, appearance persistence, shortcut conflict, clear, reset, config draft) and 4 tests in `tests/project_config.rs` (read, edit, write round trip; empty config; stale write) against the client fake daemon.
- `cargo.sh check -p bachuc-settings --tests --examples` -> PASS, no warnings in this crate.
- `cargo.sh fmt -p bachuc-settings --check` -> PASS.
- Bundle, parity, persistence, remove-connected-host, and AX checks -> DEFERRED (UI checker).
- Anti-goal: the code writes only `hosts.json`, `appearance.json`, and `shortcuts.json` in the BachucGpui folder and sends `write_project_config` to the connected daemon. `find ~/.bachuc .dev/paseo-home -mmin -90 -name '*.json'` lists only daemon-written files (`agent-requests`, `creations`, `config.json`). No test touches either home; tests use temp dirs and a local fake daemon.

## Anti-goal

PASS: no settings code path writes under `~/.bachuc` or `.dev/paseo-home`.

## Parity list

DEFERRED (UI checker): each section against the Electron settings page on 6770.

## Lessons

- Electron has no shortcut conflict state (no conflict string in `i18n/resources/en.ts`). The GPUI recorder names the other action ("Already used by ...") and disables Done on a clash. It uses `bachuc_command::gpui_keystrokes` overlap, so the `Digit` wildcard clashes with a digit.
- The recorder reads key presses through `App::intercept_keystrokes` and stops propagation only while a row records. Backspace alone removes the last combo; Escape records as `Escape`.
- `HostsPane` calls `load_hosts`/`save_hosts` on the real data dir, so the host add/edit/remove path has no unit test; check it with a temporary `hosts.json` and restore it. Removing the last host is refused ("Keep at least one host"), because an empty `hosts.json` is re-seeded with release and live.
- Removing the connected host shows an inline confirm, then a "Disconnected" callout. TASK-155 must handle `SettingsEvent::HostsChanged` by reconnecting or picking another host; the view does not reconnect.
- A GPUI `role` needs an element `.id` (a `Stateful`); `Icon::with_size` needs `Sizable`, `Button::disabled` needs `Disableable`.
- `tests/project_config.rs` reuses the client fake daemon through `#[path = "../../client/tests/common/mod.rs"]`; dev-deps `futures`, `tungstenite`, `uuid` are required for it.
- Process note: one file edit (`ui.rs`, `error_text` return type) was made with `sed -i` by mistake, against the edit-tool rule; the result was checked by compile.

## UI Checks

Run by the UI checker on 2026-10-16, working tree on HEAD 5afebcedd, bundle `BachucSettings` rebuilt before the run. Screenshots: `/tmp/gpui-ui-check/I/`. The example ran with `--url ws://127.0.0.1:6770/ws` (the live daemon on 6768 was down). The reference is the in-place Electron build on 6770, light theme, its Settings page.

Backup: `hosts.json` copied to `/tmp/gpui-ui-check/I/backup/hosts.json` before the first launch (release 6767 and live 6768 entries); `appearance.json` and `shortcuts.json` did not exist. After the check `hosts.json` was restored byte for byte (`diff` clean) and the two new files were deleted, so the folder lists only `client-id` and `hosts.json`, as before.

### Add host, dark theme, rebind, relaunch -> PASS

- Add host: name `settings`, URL `ws://127.0.0.1:6770/ws`, Save. The list showed three hosts and a "Connected" badge on `settings`; `hosts.json` held the new entry.
- Theme: picked Claude dark; the whole window turned dark at once; `appearance.json` held `"theme": "claude-dark"`.
- Rebind: Toggle settings recorded `⌃⌥J`, Done; the row showed a reset arrow; `shortcuts.json` held `"settings.toggle": "Ctrl+Alt+J"`.
- Relaunch: the new host, the dark theme, and `⌃⌥J` on Toggle settings were all still there.

### Remove the connected host -> PASS

Trash on `settings` opened an inline confirm: "Remove settings? This host is connected. Removing it disconnects the client." with Cancel and Remove. Remove deleted it from `hosts.json` (only release and live remained) and the view showed the callout "Disconnected: settings is no longer a saved host. Pick another host to connect."

### Parity (light) -> FAIL, 5 mismatches in 3 sections. Dark theme has no reference capture.

| Element           | Reference (Electron)                                                                | GPUI                                                          | Result                                       |
| ----------------- | ----------------------------------------------------------------------------------- | ------------------------------------------------------------- | -------------------------------------------- |
| Sidebar width, bg | 320, `#f4f4f6`                                                                      | 320, `#f4f4f6`                                                | match                                        |
| Back row          | arrow and "Back" at y 63                                                            | arrow and "Back" at y 54                                      | match within 9 px layout (title bar differs) |
| Page title        | large heading per page ("Appearance", "Shortcuts")                                  | none                                                          | MISMATCH                                     |
| Sidebar sections  | App group (12 pages) and Host group (Overview, Projects, Connections ...)           | five pages: Hosts, Appearance, Shortcuts, Providers, Projects | MISMATCH (by scope)                          |
| Theme control     | one dropdown, current value `System`                                                | three radio chips: Light, Claude dark, System                 | MISMATCH                                     |
| Fonts rows        | Interface font, Interface size, Content size, Code font, Code size, Highlight theme | Interface size and Code size only                             | MISMATCH                                     |
| Shortcut rows     | 57 px rows, chord badge, `...` menu                                                 | 61 px rows, chord badge, pencil and X buttons                 | MISMATCH (4 px)                              |
| Section cards     | rounded card with 1 px border, grouped heading label above                          | same pattern                                                  | match                                        |
| Chord badge style | `#f1f0f5` pill, `⌘K` glyphs                                                         | same pill and glyphs                                          | match                                        |

The Electron Shortcuts page lists actions that were dropped on purpose (Search files, Show keyboard shortcuts, Open project, New workspace, Pin chat); not counted.

### Accessibility tree -> PASS

`container Settings`, `tab group Sections` with `tab Hosts`, `Appearance`, `Shortcuts`, `Providers`, `Projects` (selected state as `Value: on` or `off`); `Heading` per section; host rows `Host <name>` with `Edit <name>` and `Remove <name>` actions; form fields `Host name`, `Host URL`, `Host password` with ids; `button Save host`, `button Cancel host form`; theme `radio button` Light, Claude dark, System with selected state; font fields `Interface font size` and `Code font size`; shortcut rows `Rebind <action>` and `Clear <action>`, `Cancel recording for <action>`; removal confirm `Confirm remove settings`, `Cancel remove`.

### Anti-goal: no file from the settings view under the daemon homes -> PASS

The view wrote only `hosts.json`, `appearance.json`, and `shortcuts.json` in `BachucGpui`. A `find` over `~/.bachuc` and `.dev/paseo-home` for `hosts.json`, `appearance.json`, `shortcuts.json` modified in the last 25 minutes returned nothing. Other `.json` files under `~/.bachuc` changed in that window (agent records, plugin data), written by the running release daemon, not by the example.

### Cleanup

The example and the reference app were quit; the lock was released; the host file and folder were restored; no request was sent to 6767 by the checker (the example listed the release host but its client connected only to the 6770 URL given on the command line; the Hosts pane showed a "Connected" badge only on the 6770 host).

## Fix Round 1

Fix builder, 2026-10-16, on HEAD 5afebcedd plus the crate edits below. Only `crates/settings` changed; the public API (`Settings::new`, `SettingsEvent`, `load_appearance`, `appearance::{Appearance, load_from, save_to, apply}`) is unchanged.

### Changed

- `src/ui.rs`: `page_title` (26 px `4xl`, medium, 4 px left inset, 24 px below, `Role::Heading`), `MenuTrigger` (a `Selectable` + `DropdownMenu` anchor), `select_trigger` (`DropdownTrigger` size `sm`: 32 px, 12 px padding, `RADIUS_XL`, `border_accent` outline, leading icon, value, chevron), and `more_trigger` (32 px `...` button).
- Page title: every pane starts with its section label as the page title: Hosts, Appearance, Shortcuts, Providers, Projects. The project editor shows "Projects", as Electron does.
- Theme control: the three radio chips became one dropdown. The trigger shows the current theme icon and label ("Theme: <label>" for AX). The menu is 200 px, anchored top right, with Light, Claude dark, and System, and a check on the current one.
- Shortcut rows: 12 px padding (was 16) and a 32 px menu slot, so a row is 57 px (12 + 32 + 12 + 1 border). The pencil, X, and reset buttons became a `...` menu, `Actions for <name>`, with Bind or Rebind, Clear when a chord is set, and Reset to default when overridden. Done and Cancel stay inline while recording.
- Group G lesson: `ProjectEditor` sent `read_project_config` in its constructor. It now reads only when the client is connected, and stays in the loading state otherwise. A `ClientEvent::Connected` subscription reads again, unless a loaded draft is on screen. The Hosts, Providers, and Projects list panes read only cached client state and send no request.
- `src/hosts.rs`: two clippy "redundant reference in `format!` argument" warnings removed; no behavior change.

### Not fixed (reported as questions)

- Fonts rows Interface font, Code font, Content size, and Highlight theme: `bachuc_theme` has no font family, content size, or syntax theme setting (`set_font_scale` takes interface and code only). A row that stores a value nothing applies would mislead, so the rows stay out.
- Sidebar App and Host groups: the owner scoped the sections to five pages, so the mismatch is by scope.
- `bachuc_theme::metrics` has no `FONT_4XL`; `ui.rs` holds a local `FONT_4XL = 26.` from `theme.ts`.

### Verified

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-settings` -> PASS: 26 unit tests and 4 `project_config` tests.
- `cargo.sh fmt -p bachuc-settings --check` -> PASS.
- `cargo.sh clippy -p bachuc-settings --tests --examples` -> PASS, no errors. One pre-existing warning remains: `too_many_arguments` on `AppearancePane::size_row`.
- `CARGO_BUILD_JOBS=4 cargo.sh build -p bachuc-app` -> PASS.
- Parity recheck of the page title, theme dropdown, and 57 px shortcut rows -> DEFERRED (UI checker).
