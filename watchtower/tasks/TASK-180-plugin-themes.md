# TASK-180 Plugin themes

Group: J (writes `crates/theme`)
Class: code

## Brief

Goal: Let the theme crate hold plugin themes and show one of them, with the same token mapping as Electron, so a theme from a plugin such as claude-look colors the whole app.

Change: the theme crate knows two fixed token sets and a three-way `ThemeChoice` -> it also keeps a list of custom themes, a selected custom theme id, and builds each custom theme's tokens from its eight colors.

Design: [packages/app/src/plugins/themes/index.ts](packages/app/src/plugins/themes/index.ts) (validation lines 23-39, `buildDarkPluginTheme` lines 58-84, `buildLightPluginTheme` lines 86-114, option id line 135, fallback to the default in [packages/app/src/appearance/provider.tsx](packages/app/src/appearance/provider.tsx) lines 42-43) and the derived tokens of `buildLightSemanticColors` and the dark builder in [packages/app/src/styles/theme.ts](packages/app/src/styles/theme.ts) (lines 262-320 and 395-430). The contribution contract is [packages/plugin/src/contracts.ts](packages/plugin/src/contracts.ts) lines 56-72.

Boundaries:

- Follows [ADR-0045](watchtower/decisions/ADR-0045-native-rust-plugin-clients-over-webview-panes-or-shared-logic.md) and [ADR-0043](watchtower/decisions/ADR-0043-electron-tokens-and-lucide-icons-over-gpui-kit-defaults.md): tokens stay the Electron values; a custom theme only replaces what Electron's plugin builders replace.
- Dependency rule (Planner call): `bachuc-theme` must not depend on `bachuc-plugin-host`. `bachuc-plugin-host` depends on `bachuc-theme` (for its `ui` module) and on `bachuc-icons`, which depends on `bachuc-theme`, so that edge makes a cycle. This crate defines its own `CustomTheme` with the same eight colors as `bachuc_plugin_host::PluginThemeColors`. TASK-177 maps contributions into it (`watch_plugin_themes`). This TASK consumes nothing from TASK-170 beyond the color set.
- Cycle shortcut (Planner call, Q-103): while a plugin theme is selected, the cycle-theme shortcut goes to the next built-in theme; a test covers it.
- Keep `ThemeChoice` and every existing public item unchanged: `crates/settings/src/appearance.rs:80-100`, `pane_appearance.rs:23-34`, and `crates/app/src/state.rs:271` match `ThemeChoice` exhaustively. The custom theme sits on top of the choice: when the selected id is in the list, it wins; else the choice applies.
- Mapping in `Tokens::from_custom`, from the base of the same scheme (`Tokens::light()` or `Tokens::claude_dark()`):
  - Both: `surface0` <- background, `surface1` <- raised, `surface2` <- control, `surface3` <- border, `surface4` <- ring, `surface_diff_empty` <- raised, `surface_sidebar_hover` <- raised, `foreground`, `foreground_muted` <- muted_foreground, `foreground_extra_muted` <- ring, `border`, `border_accent` <- border, `accent` and `accent_bright` <- accent or foreground, `accent_foreground` <- background, `ring`, `terminal_background` <- background, `terminal_foreground` and `terminal_cursor` <- foreground, `ansi[8]` <- ring.
  - Light: `surface_sidebar` <- control, `surface_sidebar_selected` <- border, `surface_workspace` <- background, `popover` <- background, `ansi[0]` <- foreground.
  - Dark: `surface_sidebar` <- background, `surface_sidebar_selected` <- control, `surface_workspace` <- raised, `popover` <- control, `ansi[0]` <- control.
  - Every other token (status, tints, dots, diff, destructive, selection) keeps the base value, as Electron keeps its built-in status colors and `destructive`.
- Applying: with an active custom theme, `resync` sets the gpui-kit mode from its scheme (`Theme::change`), `tokens(cx)` returns its tokens, and a system appearance change does not switch it. With no active custom theme, behavior is today's.
- Planner call: when the selected id is missing from the list (plugin off or not loaded yet), the stored selection stays and `ThemeChoice` applies; the custom theme comes back when the list holds it again. Electron falls back to its default theme instead; keeping the choice avoids a flash at start, before `plugin.list` answers.
- `set_custom_themes` replaces the list, and calls `resync` only when the active custom theme or its colors changed.
- Keep each rule in a pure function and test it: `from_custom` and `active_custom` need no `App`.

Files:

- [packages/desktop-gpui/crates/theme](packages/desktop-gpui/crates/theme) (`src`, `tests`)

Interfaces:

- Produces: `bachuc_theme::CustomThemeColors` (`background`, `foreground`, `raised`, `control`, `border`, `muted_foreground`, `ring: Hsla`, `accent: Option<Hsla>`, derives `Clone, Copy, Debug, PartialEq`), `bachuc_theme::CustomTheme` (`id: SharedString`, `name: SharedString`, `scheme: Scheme`, `colors: CustomThemeColors`, derives `Clone, Debug, PartialEq`), and `bachuc_theme::Tokens::from_custom(colors: &CustomThemeColors, scheme: Scheme) -> Tokens` for TASK-177
- Produces: `bachuc_theme::set_custom_themes(themes: Vec<CustomTheme>, cx: &mut App)`, `bachuc_theme::custom_themes(cx: &App) -> Vec<CustomTheme>`, `bachuc_theme::set_custom_theme(id: Option<SharedString>, cx: &mut App)` (stores the selection; `None` returns to `ThemeChoice`), `bachuc_theme::custom_theme(cx: &App) -> Option<SharedString>` (the stored selection), `bachuc_theme::active_custom_theme(cx: &App) -> Option<CustomTheme>` (the selection when the list holds it), and `bachuc_theme::active_custom<'a>(selected: Option<&str>, themes: &'a [CustomTheme]) -> Option<&'a CustomTheme>` for TASK-177

Expected result:

- After `set_custom_themes` with a dark theme and `set_custom_theme(Some(id))`, `tokens(cx).surface0` is its background and the gpui-kit theme is dark.
- Cases:
  - Light colors without `accent` -> `accent` and `accent_bright` equal `foreground`; `surface_sidebar` equals control.
  - Dark colors with `accent` -> `accent` is that color; `surface_sidebar` equals background; `status_success` equals the `claude_dark` value.
  - Selection set before the list holds it -> `ThemeChoice` applies; the list arrives -> the custom theme applies.
  - The list drops the active theme -> `ThemeChoice` applies and `custom_theme` still returns the id.
  - `set_custom_theme(None)` -> `ThemeChoice` applies.

Anti-goal: the built-in themes stay identical: the token comparison test against `packages/app/src/styles/theme.ts` and `every_theme_matches_the_highlight_package_in_light_and_dark` pass unchanged, and `tokens(cx)` with no custom theme equals `Tokens::light()` or `Tokens::claude_dark()`; tripwire; read at completion.

## Verify

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-theme` -> pass, including the existing token, syntax, and font tests unchanged and new pure tests: the light and dark `from_custom` mappings field by field, the accent fallback, base status colors kept, and `active_custom` with a present id, a missing id, and `None`
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-theme` -> exit 0
- `rg -n "bachuc-plugin-host" packages/desktop-gpui/crates/theme/Cargo.toml` -> no match
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> exit 0 with no edit outside this group's files
