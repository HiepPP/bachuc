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

### Re-check 1

Run by the UI checker on 2026-10-16, HEAD 9fec90f18 (includes fix commit dd8b40192). Bundle `BachucSettings` rebuilt before the run, light theme, example on `ws://127.0.0.1:6770/ws` (live is down). Reference: in-place Electron build on 6770, opened by path (an installed release app with the same bundle id runs at the same time; the checker did not touch it). Both windows 1200 wide; the GPUI window has a native title bar (32 px), the reference overlays it, so offsets are from the window top. Screenshots: `/tmp/gpui-ui-check/I/` (`gpui-*-r3.png`, `ref-*-r2.png`). `hosts.json` was backed up (`backup2/`) and is unchanged after the run; no file was added to the BachucGpui folder.

| Line                            | Result    | Evidence                                                                                                                                                                                                                                                                     |
| ------------------------------- | --------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Page title on each pane         | PASS      | Hosts, Appearance, Shortcuts show a large heading ("Appearance" y 78, reference y 76; "Shortcuts" 109 px wide, reference 112 px).                                                                                                                                            |
| Page title spacing              | FAIL      | Title to section label: GPUI 53 px (title y 78, label y 131), reference 45 px (y 76, label y 121). 8 px.                                                                                                                                                                     |
| Theme dropdown trigger          | PASS      | One dropdown with icon, "System", chevron: 118 x 32 px in both, right inset 17 px from the card in both.                                                                                                                                                                     |
| Theme dropdown menu             | FAIL      | Same width (about 200) and 28 px row pitch, check on the current row. GPUI items use a smaller font and icon (about 12 px; reference 14 px, 16 px icon). Items by scope: Light, Claude dark, System (reference adds a rule and Zinc, Midnight, Claude, Ghostty, Pure black). |
| Shortcut rows 57 px, `...` menu | PASS      | Rows at y 182, 239, 296: 57 px pitch, equal to the reference (171, 228, 285). Chord badge and `...` button on the right in both. The `...` menu opens Rebind and Clear, 28 px rows, in both.                                                                                 |
| Sidebar row pitch               | FAIL      | GPUI Hosts, Appearance, Shortcuts at y 94, 127, 160 (33 px pitch); reference nav rows at 131, 161, 191 (30 px pitch). 3 px per row.                                                                                                                                          |
| Fonts rows, row height          | FAIL      | Fonts card rows are 79 px (Interface size y 275 to 354); reference rows are 69 px (y 328 to 397). Input style also differs: GPUI white with border, reference grey fill, no border.                                                                                          |
| Fonts rows, missing rows        | known gap | Interface font, Content size, Code font, Highlight theme (Q-083). Recorded, not failed.                                                                                                                                                                                      |
| Settings sidebar groups         | known gap | App and Host groups (Q-084). Recorded, not failed.                                                                                                                                                                                                                           |

Cleanup: the example was quit; the lock was released; 6767 untouched. In the reference, one stray Escape left Settings and a click opened the existing closed Claude thread "Check Claude CLI computer use with separate cursor" (view only; nothing sent).

## Fix Round 2

Fix builder, 2026-10-16, on HEAD e034a125d plus the crate edits below. Only `crates/settings` changed; `Settings::new`, `SettingsEvent`, and `load_appearance` are unchanged.

### Cause

GPUI's default line height is 1.618 of the font size. The Electron settings text uses the browser `normal` line height, about 1.2 (14 px text gets a 17 px line, 12 px text gets 15 px). The page title spacing, the sidebar row pitch, and the Fonts row height all came from that extra line height.

### Changed

- `src/ui.rs`: new `LINE_HEIGHT_NORMAL = 1.2`.
- `src/settings.rs`: the Settings root sets `line_height(relative(LINE_HEIGHT_NORMAL))`, and every pane inherits it. Expected results: sidebar rows 28 px (the `minHeight`) plus a 2 px gap, so a 30 px pitch. Title to section label is about 45 px (the title line is 31 px, not 42 px). Card rows with a title and hint are 16 + 36 + 16 + 1 = 69 px. Shortcut rows stay at 57 px, because their 32 px slot is taller than the text.
- `src/pane_appearance.rs`, size inputs (`sizeInput`): 64 x 36 px (`Styled::h`), `surface2` fill, 1 px `border` color, `RADIUS_MD`, 12 px horizontal padding, 14 px right-aligned text. The `px` unit is 14 px with an 8 px gap.
- `src/pane_appearance.rs`, theme menu: the items are `PopupMenuItem::element` rows with a 16 px icon, an 8 px gap, and 14 px text. `PopupMenuItem::icon` forces an extra-small icon, and the menu item sets `text_sm` (about 12 px). The check stays on the right. The items stay Light, Claude dark, and System (by scope).

### Not changed

- The Shortcuts `...` menu still uses plain `PopupMenuItem::new` items (12 px text). Re-check 1 marked it PASS, so it stays out of this round.
- The source sets a 1 px `colors.border` (`#e4e4e7`) border on the size input. Re-check 1 reported "no border" for the reference, so the edge is faint there. The border follows the source.

### Verified

- GitNexus `impact` on `size_row`, `theme_dropdown`, and `Settings` render -> UNKNOWN (the index has no symbols for this crate). `rg` shows that `size_row` and `theme_dropdown` are private to `pane_appearance.rs`.
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-settings` -> PASS: 26 unit tests and 4 `project_config` tests.
- `cargo.sh fmt -p bachuc-settings --check` -> PASS (after `cargo.sh fmt`).
- `CARGO_BUILD_JOBS=4 cargo.sh clippy -p bachuc-settings --tests --examples` -> PASS, no errors. The pre-existing `too_many_arguments` warning on `size_row` remains.
- `CARGO_BUILD_JOBS=4 cargo.sh build -p bachuc-app` -> PASS.
- Recheck of title spacing (45 px), theme menu text and icons (14 and 16 px), sidebar pitch (30 px), and Fonts rows (69 px, grey input) -> DEFERRED (UI checker).

### Lessons

- A view that copies Electron text layout sets `line_height(relative(1.2))` at its root. GPUI's 1.618 default makes text rows taller than the Electron rows.
- `PopupMenuItem::icon` always draws an extra-small icon, and menu rows set `text_sm`. For Electron-sized rows, use `PopupMenuItem::element` with your own icon and `text_size`.

### Re-check 2

Run by the UI checker on 2026-10-16, HEAD abf2569f1 (includes fix commit 90399f699). Bundle `BachucSettings` rebuilt before the run, light theme, example on `ws://127.0.0.1:6770/ws` (live is down). Reference: the in-place Electron build on 6770. The GPUI window is 1200 x 832 (32 px title bar), the reference 1200 x 800, so offsets are between elements, not from the window top. The pointer and foreground owner rules arrived mid-run: the GPUI clicks ran before them, and the reference theme menu was not reopened (its capture is `ref-theme-menu-r2.png` from Re-check 1). Screenshots: `/tmp/gpui-ui-check/I/` (`gpui-appearance-r4.png`, `gpui-theme-menu-r4.png`, `ref-appearance-r4.png`, `*-input-z.png`).

| Line                        | Result | Evidence                                                                                                                                                                                                                                                   |
| --------------------------- | ------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Page title to section label | PASS   | "Appearance" title y 72, "Theme" label y 118: 46 px. Reference y 76 and 121: 45 px.                                                                                                                                                                        |
| Theme menu text and icons   | PASS   | Items Light, Claude dark, System: text 14 px ("Claude dark" 76 px wide; "System" x 922 to 970 in both apps), 16 px icon box at x 897 to 915 in both, row pitch 28 in both, check on System. Menu left edge 887 vs 885. Reference extra items by scope.     |
| Settings sidebar row pitch  | PASS   | Hosts, Appearance, Shortcuts at y 92, 122, 152: 30 px. Reference rows 30 px.                                                                                                                                                                               |
| Fonts rows height           | PASS   | Interface size y 288, Code size y 357: 69 px. Reference rows 69 px (y 363, 432).                                                                                                                                                                           |
| Size input fill and border  | PASS   | GPUI input 64 x 36, `#F4F4F6` fill, 1 px border `#E4E4E4`. The reference input has a 1 px border too: `#E3E3E5` at the left edge (zoom `ref-input-z.png`). The source value `#e4e4e7` matches. Re-check 1 "no border" was a faint edge, not a missing one. |

Remaining gaps are unchanged: Interface font, Content size, Code font, Highlight theme rows (Q-083), and the App and Host sidebar groups (Q-084).

Cleanup: the example and the reference app were quit; the lock was released; `hosts.json` was not changed by this group; no file was added to the BachucGpui folder; 6767 untouched.
