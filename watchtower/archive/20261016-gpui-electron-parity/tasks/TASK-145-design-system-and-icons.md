# TASK-145 Design system tokens and lucide icons

Group: A (writes `crates/theme` and the new `crates/icons`)
Class: code

## Brief

Goal: Give every GPUI view the Electron colors, type sizes, spacing, radii, control heights, and icons, so later TASKs can match the Electron look exactly.

Change: gpui-kit default themes and no icons -> `bachuc-theme` tokens for `light` and `claude` dark, plus a `bachuc-icons` crate with generated lucide SVGs.

Boundaries:

- Follows [ADR-0043](watchtower/decisions/ADR-0043-electron-tokens-and-lucide-icons-over-gpui-kit-defaults.md): port `light` and `claude` dark only.
- Port values exactly from [packages/app/src/styles/theme.ts](packages/app/src/styles/theme.ts): the light semantic colors (lines 324-346), the `claude` dark colors (`claudeDarkColors`, line 522), status colors (147-161), status dots (205-219), diff colors (117-125), terminal colors (light: 245-260; `claude` dark: the shared `darkTerminalAnsi` at 374-389 with black and bright black from the `claude` config at 537-538, and terminal background, foreground, and selection from `buildDarkSemanticColors` at 442-452), font sizes (578-588), weights (601-606), spacing (561-576), radii (608-617), border widths (619-623), icon sizes (594-599), and shadows (674-693 dark, 737-756 light).
- Port control heights from [packages/app/src/components/ui/control-geometry.ts](packages/app/src/components/ui/control-geometry.ts) (lines 25-49), and layout sizes from [packages/app/src/constants/layout.ts](packages/app/src/constants/layout.ts): `HEADER_INNER_HEIGHT` 36, `WORKSPACE_SECONDARY_HEADER_HEIGHT` 36, `MAX_CONTENT_WIDTH` 820, traffic-light clearance 78x45. The left sidebar default width is 320, min 200, max 600 ([packages/app/src/stores/panel-store/state.ts](packages/app/src/stores/panel-store/state.ts) lines 24-26). Settings: sidebar 320 and detail min 400 (`layout.ts` lines 20-21), content column 720 ([packages/app/src/screens/settings-screen.tsx](packages/app/src/screens/settings-screen.tsx) line 1550). `TAB_BAR_HEIGHT` is the app's `WORKSPACE_SECONDARY_HEADER_HEIGHT`, and `CONTROL_XS` is `CONTROL_HEIGHTS.tight` (28).
- Fonts: the UI font is the system font (SF Pro on macOS); the mono font is `SFMono-Regular` with `Menlo` as fallback. Keep the font scale settings: interface and code sizes scale the type tokens.
- Keep the current public items working: `init`, `Scheme`, `scheme`, `TurnLabel`, `turn_label_colors`. `claude` dark maps to `Scheme::Dark`.
- `init` also sets the gpui-kit theme colors from the tokens, so gpui-kit components (buttons, inputs, menus) use Electron colors.
- Icons: lucide 0.546.0 icon data is in `node_modules/lucide-react-native/dist/esm/icons/<kebab-name>.js`. It holds `path`, `circle`, `line`, `rect`, `polyline`, and `ellipse` nodes (for example `git-branch.js` and `settings.js`); the generator writes every node kind. It collects every named import from `lucide-react-native` in `packages/app/src` (about 191 files). Planner call: skip the namespace import in `packages/app/src/plugins/icons.ts`, because plugin surfaces are out of scope. Each SVG has a 24x24 viewBox, stroke 2, round caps and joins, `fill="none"`, and `stroke="currentColor"`. Commit the SVGs, so a build does not need Node.
- GPUI takes one asset source. `bachuc_icons::Assets` serves the generated SVGs and falls through to the gpui-kit assets, so gpui-kit components keep their own icons (menu, select, dialog close).
- An icon is a gpui-component `Icon` loaded with `Icon::path` (`gpui-component-0.7.3/src/icon.rs` line 117); size it with `with_size` and color it with `text_color`.
- Do not change files under `packages/app` or `node_modules`.

How:

- Write the token struct, the two themes, the metrics constants, and `set_theme` with a system choice.
- Write `crates/icons/codegen/generate-icons.mjs` and run it.
- Write the `IconName` enum, the asset source, and the `icon` element.
- Add `examples/tokens.rs` in `crates/theme` that shows swatches, type sizes, and 40 icons in both themes.

Files:

- [packages/desktop-gpui/crates/theme](packages/desktop-gpui/crates/theme) (`Cargo.toml`, `src`, `examples/tokens.rs`)
- [packages/desktop-gpui/crates/icons](packages/desktop-gpui/crates/icons) (new crate `bachuc-icons`: `Cargo.toml`, `src`, `codegen/generate-icons.mjs`, `assets/*.svg`)
- [packages/desktop-gpui/Cargo.lock](packages/desktop-gpui/Cargo.lock) (generated)

Interfaces:

- Produces: `bachuc_theme::Tokens` (a `Copy` struct of `gpui_kit::Hsla` fields named after the `theme.ts` tokens in snake case: `surface0` to `surface4`, `surface_sidebar`, `surface_sidebar_hover`, `surface_sidebar_selected`, `surface_workspace`, `surface_diff_empty`, `interaction_highlight`, `foreground`, `foreground_muted`, `foreground_extra_muted`, `border`, `border_accent`, `accent`, `accent_bright`, `accent_foreground`, `destructive`, `destructive_foreground`, `ring`, `popover`, `status_success`, `status_danger`, `status_warning`, `status_merged`, `tint_success`, `tint_danger`, `tint_warning`, `tint_merged`, `dot_success`, `dot_danger`, `dot_warning`, `dot_running`, `diff_addition`, `diff_deletion`, `terminal_background`, `terminal_foreground`, `terminal_cursor`, `terminal_selection`, and `ansi: [gpui_kit::Hsla; 16]`) for TASK-147 to TASK-155
- Produces: `bachuc_theme::tokens(cx: &gpui_kit::App) -> Tokens` for TASK-147 to TASK-155
- Produces: `bachuc_theme::ThemeChoice` (enum `Light`, `ClaudeDark`, `System`), `bachuc_theme::set_theme(choice: ThemeChoice, cx: &mut gpui_kit::App)`, and `bachuc_theme::set_font_scale(interface: f32, code: f32, cx: &mut gpui_kit::App)` for TASK-153, TASK-155
- Produces: `bachuc_theme::metrics` (module of `pub const` `f32` values: `SPACE_0_5`, `SPACE_1`, `SPACE_1_5`, `SPACE_2`, `SPACE_3`, `SPACE_4`, `SPACE_6`, `SPACE_8`, `RADIUS_SM`, `RADIUS_BASE`, `RADIUS_MD`, `RADIUS_LG`, `RADIUS_XL`, `RADIUS_2XL`, `FONT_CODE`, `FONT_SM`, `FONT_BASE`, `FONT_CONTENT`, `FONT_LG`, `FONT_XL`, `FONT_2XL`, `ICON_XS`, `ICON_SM`, `ICON_MD`, `ICON_LG`, `CONTROL_XS`, `CONTROL_SM`, `CONTROL_MD`, `HEADER_HEIGHT`, `TAB_BAR_HEIGHT`, `MAX_CONTENT_WIDTH`, `SIDEBAR_WIDTH`, `SIDEBAR_MIN_WIDTH`, `SIDEBAR_MAX_WIDTH`, `TRAFFIC_LIGHT_WIDTH`, `TRAFFIC_LIGHT_HEIGHT`, `SETTINGS_SIDEBAR_WIDTH`, `SETTINGS_DETAIL_MIN_WIDTH`, `SETTINGS_CONTENT_WIDTH`) for TASK-147 to TASK-155
- Produces: `bachuc_icons::IconName` (enum with one variant per generated icon, named as the lucide import, for example `FolderPlus`, `GitBranch`, `X`), `bachuc_icons::icon(name: IconName) -> gpui_kit::component::Icon` (sized with `ICON_MD`; change it with `with_size` and `text_color`), and `bachuc_icons::Assets` (a GPUI `AssetSource` that serves the SVGs and falls through to the gpui-kit assets) for TASK-147 to TASK-155

Expected result:

- Every listed token equals its `theme.ts` value for both themes; a test compares them within 1 unit per RGB channel and 0.01 alpha.
- `set_theme(ThemeChoice::System, cx)` follows the system appearance; `Light` and `ClaudeDark` force a theme.
- The generator writes one SVG per imported lucide name, and a second run gives no change.
- The tokens example shows the swatches and icons in both themes; icons are crisp at 14 and 16 px.

Anti-goal: no Electron token is changed or invented: every color in `Tokens` comes from `theme.ts`; tripwire; read with the token comparison test after the themes land and at completion.

## Verify

- `packages/desktop-gpui/cargo.sh test -p bachuc-theme` -> all tests pass, including the token comparison test for `light` and `claude` dark and the existing turn label test
- `node packages/desktop-gpui/crates/icons/codegen/generate-icons.mjs` twice -> `shasum` of `crates/icons/assets/*.svg` is the same after both runs
- `rg -o 'from "lucide-react-native"' -l packages/app/src | wc -l` and the generator's printed name count -> the generator reports every imported name and no missing file
- `packages/desktop-gpui/cargo.sh test -p bachuc-icons` -> pass, including a test that each `IconName` loads a non-empty SVG from `Assets`, a test that an SVG with `circle` and `line` nodes keeps them, and a test that a gpui-kit asset path still loads through `Assets`
- `packages/desktop-gpui/cargo.sh test --workspace` -> pass; the existing crates still build with the new theme
- Agent check: `packages/desktop-gpui/bundle.sh example tokens BachucTokens io.github.hieppp.bachuc-tokens`, open it, and take `cua_repl` screenshots in both themes -> swatches and icons render; icons are sharp at 14 and 16 px
- `npm run lint -- packages/desktop-gpui/crates/icons/codegen/generate-icons.mjs` -> exit 0
