# VS Code Light+ (Warm)

A client-only Paseo theme matching the configured warm Light+ workbench palette.
One theme appears in **Settings → Appearance**. No server entry, RPC, filesystem
access, network requests, or live settings synchronization.

## Color sources

Resolved on 2026-09-22 from VS Code 1.138.0 on macOS:

- Selected theme: `workbench.colorTheme: "Light+"`.
- Bundled source: `Visual Studio Code.app/Contents/Resources/app/extensions/theme-defaults/themes/light_plus.json`,
  which includes `light_vs.json`. Stock editor colors are `#FFFFFF` and `#000000`.
- User-level `workbench.colorCustomizations["[Light+]"]` overrides the workbench
  colors below. No unscoped workbench color overrides were present; this repository
  had no `.vscode` settings. This is a snapshot of that configuration, not stock Light+.
- `editor.tokenColorCustomizations["[Light+]"]` supplies separate syntax rules.
  Those are documented below but cannot be registered through Paseo's palette API.

Only theme color values are included here; private settings are neither copied nor read at runtime.

| Paseo key         | Hex       | VS Code override source                                                   |
| ----------------- | --------- | ------------------------------------------------------------------------- |
| `background`      | `#F5F3EE` | `editor.background`, `panel.background`, `terminal.background`            |
| `foreground`      | `#4C4843` | `editor.foreground`, `sideBar.foreground`, `terminal.foreground`          |
| `raised`          | `#EEEBE3` | `editorSuggestWidget.background`, `list.hoverBackground`                  |
| `control`         | `#EEEBE3` | `sideBar.background`                                                      |
| `border`          | `#DEDACF` | `editorGroup.border`, `panel.border`, `input.border`                      |
| `accent`          | `#B07B56` | `button.background`, `activityBarBadge.background`, `tab.activeBorderTop` |
| `mutedForeground` | `#8C877C` | `breadcrumb.foreground`, `tab.inactiveForeground`                         |
| `ring`            | `#CBB79E` | `focusBorder`                                                             |

`raised` and `control` intentionally share a color: the configured VS Code popup,
hover, and sidebar backgrounds do too. Paseo also uses `control` for inputs, so
its inputs differ from the configured VS Code `input.background: #F5F3EE`.

## Visual limits

Paseo expands eight palette values into its own tokens. It cannot independently
match VS Code's activity/title/status bars (`#E9E6DD`), selected rows (`#E5E0D3`),
button hover (`#99694A`), translucent editor selections (`#DCD8CBAA`), line highlights,
find matches, or per-tab borders. Fonts, icons, spacing, and layout remain Paseo's.

The terminal background and foreground match; its cursor follows the foreground
instead of VS Code's `editorCursor.foreground: #B07B56`. Terminal ANSI colors,
status colors and diff colors are derived by Paseo, not imported from VS Code.
For example, VS Code's gutter additions/deletions use `#7A8A4A` / `#B0593F`.

Syntax overrides are not supported by `addTheme`: comments `#93A1A1` (italic),
strings `#2A8C82`, numeric/language constants `#6C71C4`, functions and tags `#268BD2`,
keywords `#6E7A3D`, types `#A07842`, variables `#5B5347`, constant/language variables
`#C7553F`, and attributes `#9C7600`. VS Code semantic highlighting can further
affect rendered syntax; these are the configured TextMate values, not a claim
that every semantic token renders identically.

## Install and select

Requires Paseo daemon and client 0.8.0 or later with `addTheme` support. Verified
API contract: [Paseo theme reference](https://paseo.sh/docs/plugins/reference.md#contribute-a-theme).

```sh
cd plugins/vscode-warm-light
npm ci
npm run typecheck
npm run lint
paseo plugin install "$PWD" --id vscode-warm-light
paseo plugin ls vscode-warm-light --json
```

Open **Settings → Appearance** and select **VS Code Light+ (Warm)**.
After edits, run the checks and `paseo plugin reload vscode-warm-light`.
The returned registration cleanup removes the theme when the plugin unloads;
Paseo falls back to its default if the selected theme becomes unavailable.
The plugin does not modify VS Code settings or select itself automatically.
