# TASK-152 Command center and keyboard shortcuts

Group: H (writes the new `crates/command`)
Class: code

## Brief

Goal: Add the crate `bachuc-command` with the Electron command center palette and the Electron shortcut table, so the app can match Electron key for key.

Change: five fixed shortcuts and no palette -> the Electron command center and a shortcut registry with stored overrides.

Design: [packages/app/src/command-center/command-center.tsx](packages/app/src/command-center/command-center.tsx), [packages/app/src/keyboard/keyboard-shortcuts.ts](packages/app/src/keyboard/keyboard-shortcuts.ts), [packages/app/src/components/keyboard-shortcuts-dialog.tsx](packages/app/src/components/keyboard-shortcuts-dialog.tsx)

Boundaries:

- Port `DEFAULT_BINDINGS` of `keyboard-shortcuts.ts` (line 1186, built from `ParsedShortcutBinding` at line 116) with the labels from its help rows (`buildKeyboardShortcutHelpSections`, line 1680). Keep the macOS bindings only. Keep only actions whose feature is in this plan (see [watchtower/CONTEXT.md](watchtower/CONTEXT.md) Decisions); list the dropped actions in the outcome.
- Planner call: `Shortcut.chord` holds the Electron combo string (for example `mod+shift+p`); a multi-key chord (Electron `ChordState`, line 120) is the combos joined by one space. `None` means unassigned. Contexts collapse to one global scope in this plan; list the context-only bindings in the outcome.
- Overrides persist in `~/Library/Application Support/BachucGpui/shortcuts.json` as a JSON object of action id to combo string or `null`, as Electron's `ShortcutOverrides` (line 1189; `UNASSIGNED_COMBO = null`, line 1159). A missing key means the default; `null` or `""` means unassigned and must not crash or bind (root `CLAUDE.md`, Debugging).
- The command center lists app actions and items that the app passes in, with fuzzy search, Electron row style, and shortcut hints with `kbd`.
- The crate knows no app internals: the app passes items and runs the picked id.

How:

- Port the table and the override store with tests.
- Build the palette view.
- Add `examples/command.rs` with sample items.

Files:

- [packages/desktop-gpui/crates/command](packages/desktop-gpui/crates/command) (new crate `bachuc-command`: `Cargo.toml`, `src`, `tests`, `examples/command.rs`)

Interfaces:

- Consumes: `bachuc_theme::tokens(cx: &gpui_kit::App) -> Tokens` from TASK-145
- Consumes: `bachuc_theme::metrics` (module of `pub const` `f32` values: `SPACE_0_5`, `SPACE_1`, `SPACE_1_5`, `SPACE_2`, `SPACE_3`, `SPACE_4`, `SPACE_6`, `SPACE_8`, `RADIUS_SM`, `RADIUS_BASE`, `RADIUS_MD`, `RADIUS_LG`, `RADIUS_XL`, `RADIUS_2XL`, `FONT_CODE`, `FONT_SM`, `FONT_BASE`, `FONT_CONTENT`, `FONT_LG`, `FONT_XL`, `FONT_2XL`, `ICON_XS`, `ICON_SM`, `ICON_MD`, `ICON_LG`, `CONTROL_XS`, `CONTROL_SM`, `CONTROL_MD`, `HEADER_HEIGHT`, `TAB_BAR_HEIGHT`, `MAX_CONTENT_WIDTH`, `SIDEBAR_WIDTH`, `SIDEBAR_MIN_WIDTH`, `SIDEBAR_MAX_WIDTH`, `TRAFFIC_LIGHT_WIDTH`, `TRAFFIC_LIGHT_HEIGHT`, `SETTINGS_SIDEBAR_WIDTH`, `SETTINGS_DETAIL_MIN_WIDTH`, `SETTINGS_CONTENT_WIDTH`) from TASK-145
- Consumes: `bachuc_icons::IconName` (enum with one variant per generated icon, named as the lucide import, for example `FolderPlus`, `GitBranch`, `X`), `bachuc_icons::icon(name: IconName) -> gpui_kit::component::Icon` (sized with `ICON_MD`; change it with `with_size` and `text_color`), and `bachuc_icons::Assets` (a GPUI `AssetSource` that serves the SVGs and falls through to the gpui-kit assets) from TASK-145
- Produces: `bachuc_command::Shortcut { action: String, label: String, chord: Option<String> }`, `bachuc_command::default_shortcuts() -> Vec<Shortcut>`, `bachuc_command::load_shortcuts() -> Vec<Shortcut>` (defaults plus overrides), and `bachuc_command::save_overrides(shortcuts: &[Shortcut]) -> anyhow::Result<()>` for TASK-153, TASK-155
- Produces: `bachuc_command::CommandItem { id: String, title: gpui_kit::SharedString, group: gpui_kit::SharedString, icon: Option<bachuc_icons::IconName>, shortcut: Option<String> }` and `bachuc_command::CommandCenter` (a GPUI view) with `CommandCenter::new(items: Vec<CommandItem>, window: &mut gpui_kit::Window, cx: &mut gpui_kit::Context<CommandCenter>) -> CommandCenter` for TASK-155
- Produces: `bachuc_command::CommandEvent` (enum `Run { id: String }`, `Dismissed`) emitted by `CommandCenter` for TASK-155

Expected result:

- The palette matches the reference app's command center at rest and with a query.
- `load_shortcuts` returns the Electron defaults, and overrides survive a restart.
- Cases:
  - An override with an empty chord loads as unassigned.
  - A corrupt `shortcuts.json` loads the defaults and keeps the file.
  - A query with no match shows the Electron empty state.

Anti-goal: opening the palette and typing a query takes at most 16 ms per keystroke for 500 items; tripwire; read with a benchmark test after search lands and at completion.

## Verify

- `packages/desktop-gpui/cargo.sh test -p bachuc-command` -> pass, including the default table, `null` and `""` overrides, a multi-key chord, a missing key, the corrupt-file case, fuzzy ranking, and a 500-item search benchmark under 16 ms
- Agent check: bundle `packages/desktop-gpui/bundle.sh example command BachucCommand io.github.hieppp.bachuc-command`; compare with the reference app's command center on 6770 using `cua_repl` screenshots at rest and with a query -> the parity list in the outcome shows match or records each mismatch
- Agent check: `cua_repl` `getAXState` -> the search input and items have roles and labels
